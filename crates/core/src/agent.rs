//! Feynman Agent Service
//!
//! This module implements the core Feynman learning agent that tracks educational progress
//! through subtopics using the Model Context Protocol (MCP). The agent follows the Feynman
//! technique principle of breaking down complex topics into understandable components.

use crate::topic::SubTopic;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

// --- Agent State ---

/// Core state representation of the Feynman learning agent.
///
/// This struct tracks the overall learning progress for a main topic by managing
/// collections of subtopics in different completion states.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct FeynmanAgent {
    /// The main topic or subject area being studied (e.g., "Data Structures").
    pub main_topic: String,
    /// Subtopics that have been completely mastered across all criteria.
    pub covered_subtopics: HashMap<String, SubTopic>,
    /// Subtopics that are still being learned or have incomplete coverage.
    pub incomplete_subtopics: HashMap<String, SubTopic>,
}

impl FeynmanAgent {
    /// Creates a new Feynman agent for a specific topic.
    ///
    /// All provided subtopics start in the incomplete state and must be
    /// progressively marked as complete through the learning process.
    pub fn new(main_topic: String, subtopics: Vec<SubTopic>) -> Self {
        let incomplete_subtopics = subtopics
            .into_iter()
            .map(|st| (st.name.clone(), st))
            .collect();
        Self {
            main_topic,
            covered_subtopics: HashMap::new(),
            incomplete_subtopics,
        }
    }
}

// --- Data Structures for Tools ---

/// Arguments for updating the learning status of a specific subtopic criterion.
///
/// This struct is used by the `update_subtopic_status` MCP tool to modify
/// the completion state of individual learning criteria within subtopics.
#[derive(Deserialize, JsonSchema, Debug)]
pub struct UpdateSubtopicStatusArgs {
    /// The name of the subtopic to update (must match a subtopic in the agent state).
    pub subtopic_name: String,
    /// The learning criterion to update: 'definition', 'mechanism', or 'example'.
    #[schemars(description = "The criterion to update: 'definition', 'mechanism', or 'example'")]
    pub criterion: String,
    /// Whether this criterion has been satisfied (true) or not (false).
    #[schemars(description = "The new status: true if covered, false if not")]
    pub is_covered: bool,
}

// --- Service and Handler Implementation ---

/// The main service implementation for the Feynman learning agent.
///
/// This service provides MCP (Model Context Protocol) tools that allow external
/// agents (like an LLM) to interact with and modify the learning state.
pub struct FeynmanService {
    /// Shared agent state protected by an async mutex for concurrent access.
    pub agent_state: Arc<tokio::sync::Mutex<FeynmanAgent>>,
    /// Optional channel for broadcasting state changes to subscribers.
    pub state_tx: Option<mpsc::Sender<FeynmanAgent>>,
    /// MCP tool router for handling incoming tool calls.
    tool_router: ToolRouter<Self>,
}

#[tool_handler]
impl ServerHandler for FeynmanService {
    /// Returns server information and capabilities, advertising tool support.
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tool_router]
impl FeynmanService {
    /// Creates a new Feynman service instance.
    pub fn new(
        agent_state: Arc<tokio::sync::Mutex<FeynmanAgent>>,
        state_tx: Option<mpsc::Sender<FeynmanAgent>>,
    ) -> Self {
        Self {
            agent_state,
            state_tx,
            tool_router: Self::tool_router(),
        }
    }

    /// Retrieves the current status of the entire learning session.
    ///
    /// This tool provides a complete snapshot of the agent's state, including
    /// all subtopics and their current completion status.
    #[tool(
        description = "Get the current status of the teaching session, including all complete and incomplete subtopics."
    )]
    pub async fn get_session_status(&self) -> Result<String, String> {
        info!("Executing tool 'get_session_status'");
        let agent = self.agent_state.lock().await;
        serde_json::to_string(&*agent)
            .map_err(|e| format!("Failed to serialize agent state: {}", e))
    }

    /// Updates the learning status for a specific criterion of a subtopic.
    ///
    /// This is the core tool for tracking learning progress. It allows an LLM
    /// to mark individual learning criteria (definition, mechanism, example)
    /// as complete for a specific subtopic. If all criteria for a subtopic
    /// become complete, it is moved to the `covered_subtopics` map.
    #[tool(
        description = "Update the status of a specific learning criterion for a subtopic (e.g., mark 'definition' for 'Linked List' as covered)."
    )]
    pub async fn update_subtopic_status(
        &self,
        args: Parameters<UpdateSubtopicStatusArgs>,
    ) -> Result<String, String> {
        info!(args = ?args.0, "Executing tool 'update_subtopic_status'");
        let mut agent = self.agent_state.lock().await;
        let subtopic_name = &args.0.subtopic_name;

        let result = if let Some(subtopic) = agent.incomplete_subtopics.get_mut(subtopic_name) {
            match args.0.criterion.to_lowercase().as_str() {
                "definition" => subtopic.has_definition = args.0.is_covered,
                "mechanism" => subtopic.has_mechanism = args.0.is_covered,
                "example" => subtopic.has_example = args.0.is_covered,
                _ => return Err(format!("Invalid criterion: '{}'", args.0.criterion)),
            }

            info!(subtopic = %subtopic_name, criterion = %args.0.criterion, is_covered = %args.0.is_covered, "Agent state updated");

            if subtopic.is_complete() {
                if let Some(completed) = agent.incomplete_subtopics.remove(subtopic_name) {
                    agent
                        .covered_subtopics
                        .insert(subtopic_name.clone(), completed);
                    Ok(format!(
                        "OK. Subtopic '{}' is now fully covered.",
                        subtopic_name
                    ))
                } else {
                    Ok("OK. Status updated.".to_string())
                }
            } else {
                Ok(format!(
                    "OK. Updated criterion '{}' for subtopic '{}'.",
                    args.0.criterion, subtopic_name
                ))
            }
        } else if agent.covered_subtopics.contains_key(subtopic_name) {
            Ok(format!(
                "OK. Subtopic '{}' is already fully covered.",
                subtopic_name
            ))
        } else {
            Err(format!("Subtopic '{}' not found.", subtopic_name))
        };

        if let Some(tx) = &self.state_tx {
            if tx.send(agent.clone()).await.is_err() {
                tracing::warn!("Failed to broadcast state update: receiver dropped.");
            }
        }

        result
    }

    /// Concludes the learning session when all subtopics are complete.
    ///
    /// This tool provides a clear action for the LLM to take when the learning
    /// session has been successfully completed, signaling that all educational
    /// objectives have been met.
    #[tool(
        description = "Ends the teaching session successfully once all subtopics are fully covered."
    )]
    pub async fn conclude_session(&self) -> Result<String, String> {
        info!("Executing tool 'conclude_session'");
        // This tool's primary purpose is to give the LLM a clear action to take
        // when the lesson is over. The actual session status update is typically
        // handled by a separate mechanism (e.g., a REST API call).
        Ok("OK. Session will be concluded.".to_string())
    }
}