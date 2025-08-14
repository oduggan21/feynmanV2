//! Curriculum Generation Service
//!
//! This module provides services for generating educational curricula by breaking down
//! topics into manageable subtopics. It serves as the foundation for initializing
//! learning sessions in the Feynman agent system.

use anyhow::{Context, Result};
use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
};
use async_trait::async_trait;
use std::collections::HashMap;

/// Defines the contract for any service that can generate a curriculum.
///
/// This abstraction allows the system to swap between different curriculum
/// generation approaches (e.g., AI-powered, static mock, database-backed)
/// while maintaining a consistent interface for bootstrapping agent sessions.
#[async_trait]
pub trait CurriculumService: Send + Sync {
    /// Generates a list of key subtopics for a given main topic.
    ///
    /// This method breaks down a broad educational topic into specific,
    /// learnable subtopics that can be tracked individually.
    ///
    /// # Arguments
    ///
    /// * `topic` - The main subject area to generate subtopics for.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of subtopic names or an error.
    async fn generate_subtopics(&self, topic: &str) -> Result<Vec<String>>;
}

/// An implementation of `CurriculumService` that uses an OpenAI-compatible API.
///
/// This service leverages Large Language Models to generate contextually
/// appropriate subtopics for any given topic, providing dynamic and intelligent
/// curriculum generation.
pub struct LLMCurriculumService {
    client: Client<OpenAIConfig>,
    model: String,
    prompts: HashMap<String, String>,
}

impl LLMCurriculumService {
    /// Creates a new LLM-based curriculum service.
    ///
    /// # Arguments
    ///
    /// * `config` - OpenAI API configuration (API key, base URL, etc.).
    /// * `model` - Model identifier to use for generation (e.g., "gpt-4o").
    /// * `prompts` - A map of template strings, which must include a key
    ///   for `"generate_subtopics"`.
    pub fn new(config: OpenAIConfig, model: String, prompts: HashMap<String, String>) -> Self {
        Self {
            client: Client::with_config(config),
            model,
            prompts,
        }
    }
}

#[async_trait]
impl CurriculumService for LLMCurriculumService {
    async fn generate_subtopics(&self, topic: &str) -> Result<Vec<String>> {
        let prompt_template = self
            .prompts
            .get("generate_subtopics")
            .context("Missing prompt template: 'generate_subtopics'")?;
        let prompt = prompt_template.replace("{topic}", topic);

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(vec![
                ChatCompletionRequestSystemMessageArgs::default()
                    .content("You are a helpful assistant that generates curriculum.")
                    .build()?
                    .into(),
                ChatCompletionRequestUserMessageArgs::default()
                    .content(prompt)
                    .build()?
                    .into(),
            ])
            .build()?;

        let response = self.client.chat().create(request).await?;

        let answer = response
            .choices
            .get(0)
            .context("No response choice from LLM")?
            .message
            .content
            .as_ref()
            .context("No content in LLM response")?;

        // Parse structured subtopics from the response by looking for list items.
        let subtopics: Vec<String> = answer
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if let Some(idx) = line.find(|c: char| c == '.' || c == ')') {
                    let name = line[idx + 1..].trim().to_string();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
                None
            })
            .collect();

        Ok(subtopics)
    }
}

/// A mock `CurriculumService` for development and integration testing.
///
/// This implementation provides predictable, deterministic output, which is
/// useful for testing scenarios without external dependencies or API costs.
pub struct MockCurriculumService;

#[async_trait]
impl CurriculumService for MockCurriculumService {
    /// Generates a standard 4-subtopic curriculum for any given topic.
    ///
    /// This implementation provides a consistent curriculum structure that
    /// follows a logical progression from introductory to advanced concepts.
    async fn generate_subtopics(&self, topic: &str) -> Result<Vec<String>> {
        Ok(vec![
            format!("Introduction to {}", topic),
            "Core Concepts".to_string(),
            "Practical Applications".to_string(),
            "Advanced Topics".to_string(),
        ])
    }
}
