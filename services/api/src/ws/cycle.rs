//! Contains the logic for the agent's "ReAct" (Reason and Act) cycle.

use crate::{
    models::MessageRole,
    state::AppState,
    ws::{protocol::ServerMessage, provider::RealtimeClientEvent, session::send_msg},
};
use anyhow::{Context, Result};
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, ChatCompletionToolArgs, FunctionObjectArgs,
};
use axum::extract::ws::{Message, WebSocket};
use feynman_core::{
    agent::FeynmanAgent,
    llm_client::{LLMAction, LLMStreamEvent},
};
use futures_util::{StreamExt, stream::SplitSink};
use rmcp::{
    model::{CallToolRequestParam, RawContent},
    service::{RoleClient, RunningService},
};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

/// Handles a single user interaction, driving the agent through a ReAct cycle.
///
/// This involves:
/// 1.  Constructing the prompt with the latest agent state and history.
/// 2.  Calling the LLM to decide on an action (speak or use a tool).
/// 3.  If a tool is chosen, executing it and feeding the result back to the LLM.
/// 4.  Streaming the final text response back to the client.
/// 5.  Optionally, sending the final text to the real-time provider for text-to-speech.
#[allow(clippy::too_many_arguments)]
pub async fn handle_react_cycle(
    state: &Arc<AppState>,
    session_id: Uuid,
    history: &mut Vec<crate::models::Message>,
    agent_state_arc: &Arc<tokio::sync::Mutex<FeynmanAgent>>,
    mcp_client: &RunningService<RoleClient, ()>,
    user_text: &str,
    socket_tx: &Arc<Mutex<SplitSink<WebSocket, Message>>>,
    realtime_tx: &Option<mpsc::Sender<RealtimeClientEvent>>,
) -> Result<()> {
    // Add the new user message to the database and local history.
    let new_user_msg = state
        .db
        .add_message(session_id, MessageRole::User, user_text)
        .await?;
    history.push(new_user_msg);

    // Construct the system prompt with the current agent state.
    let current_agent_state = agent_state_arc.lock().await.clone();
    let state_json = serde_json::to_string_pretty(&current_agent_state)?;
    let system_prompt_with_state = format!(
        "{}\n\n# Current Context for This Turn\n\n**Current Curriculum Status:**\n```json\n{}\n```",
        state.system_prompt, state_json
    );

    // Build the full message history for the LLM.
    let mut messages: Vec<ChatCompletionRequestMessage> = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt_with_state)
            .build()?
            .into(),
    ];
    for msg in history.iter() {
        match msg.role {
            MessageRole::User => messages.push(
                ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build()?
                    .into(),
            ),
            MessageRole::Ai => messages.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content.clone())
                    .build()?
                    .into(),
            ),
        };
    }

    // Get the list of available tools for the agent.
    let tools = mcp_client
        .list_all_tools()
        .await?
        .into_iter()
        .map(|t| {
            Ok(ChatCompletionToolArgs::default()
                .function(
                    FunctionObjectArgs::default()
                        .name(t.name)
                        .description(t.description.unwrap_or_default())
                        .parameters(serde_json::to_value(&*t.input_schema)?)
                        .build()?,
                )
                .build()?)
        })
        .collect::<Result<Vec<_>>>()?;

    // Ask the LLM to decide on the next action.
    let action = state
        .llm_client
        .decide_action("".to_string(), messages.clone(), tools)
        .await?;

    let mut full_response = String::new();
    match action {
        LLMAction::TextResponse(response_text) => {
            // If the LLM decides to just respond, use the provided text.
            full_response = response_text
        }
        LLMAction::ToolCall(tool_calls) => {
            // If the LLM decides to use tools, execute them.
            let mut tool_results = vec![];
            for call in &tool_calls {
                let result = mcp_client
                    .peer()
                    .call_tool(CallToolRequestParam {
                        name: call.function.name.clone().into(),
                        arguments: Some(serde_json::from_str(&call.function.arguments)?),
                    })
                    .await?;

                let annotated_content = result
                    .content
                    .context("Tool call returned no content")?
                    .pop()
                    .context("Content list was empty")?;
                let result_text = match annotated_content.raw {
                    RawContent::Text(text_content) => text_content.text,
                    _ => "{\"error\": \"Unexpected content type from tool\"}".to_string(),
                };
                tool_results.push(result_text);
            }

            // Append the tool calls and their results to the history.
            let mut history_with_tools = messages;
            history_with_tools.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .tool_calls(tool_calls.clone())
                    .build()?
                    .into(),
            );
            for (i, result) in tool_results.iter().enumerate() {
                history_with_tools.push(
                    ChatCompletionRequestToolMessageArgs::default()
                        .tool_call_id(tool_calls[i].id.clone())
                        .content(result.clone())
                        .build()?
                        .into(),
                );
            }

            // Call the LLM again with the tool results to get the final response.
            let mut final_stream = state
                .llm_client
                .stream_after_tools("".to_string(), history_with_tools)
                .await?;
            while let Some(event_result) = final_stream.next().await {
                if let Ok(LLMStreamEvent::TextChunk(chunk)) = event_result {
                    full_response.push_str(&chunk);
                }
            }
        }
    }

    // Save the final AI response to the database.
    if !full_response.is_empty() {
        let new_ai_msg = state
            .db
            .add_message(session_id, MessageRole::Ai, &full_response)
            .await?;
        history.push(new_ai_msg);
    }

    // Send the response to the client, either via TTS or as text.
    if let Some(tx) = realtime_tx {
        let _ = tx
            .send(RealtimeClientEvent::TextToSpeak(full_response))
            .await;
    } else {
        let mut sink = socket_tx.lock().await;
        send_msg(&mut sink, ServerMessage::ResponseStart).await?;
        send_msg(
            &mut sink,
            ServerMessage::ResponseChunk {
                chunk: full_response,
            },
        )
        .await?;
        send_msg(&mut sink, ServerMessage::ResponseEnd).await?;
    }

    Ok(())
}
