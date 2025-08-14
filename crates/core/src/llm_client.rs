use anyhow::{Result, anyhow};
use async_openai::{
    Client,
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionTool, CreateChatCompletionRequestArgs,
        CreateChatCompletionResponse,
    },
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

/// Represents a tool call requested by the LLM.
pub type ToolCall = async_openai::types::ChatCompletionMessageToolCall;

/// Represents the events that can be yielded from a streaming text response.
#[derive(Debug, Clone)]
pub enum LLMStreamEvent {
    TextChunk(String),
}

/// A stream of text chunks from the LLM.
pub type LLMStream = Pin<Box<dyn Stream<Item = Result<LLMStreamEvent, OpenAIError>> + Send>>;

/// Represents the two possible outcomes of the LLM's initial decision-making turn.
#[derive(Debug, Clone)]
pub enum LLMAction {
    /// The LLM decided to respond directly with text.
    TextResponse(String),
    /// The LLM decided to call one or more tools.
    ToolCall(Vec<ToolCall>),
}

/// A generic client for interacting with an LLM.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Makes a single, non-streaming call to the LLM to decide on the next action.
    async fn decide_action(
        &self,
        system_prompt: String,
        history_with_user_message: Vec<ChatCompletionRequestMessage>,
        tools: Vec<ChatCompletionTool>,
    ) -> Result<LLMAction>;

    /// Makes a streaming call to the LLM after tools have been executed.
    async fn stream_after_tools(
        &self,
        system_prompt: String,
        history_with_tool_results: Vec<ChatCompletionRequestMessage>,
    ) -> Result<LLMStream>;
}

/// An implementation of `LLMClient` for any OpenAI-compatible API.
pub struct OpenAICompatibleClient {
    client: Client<OpenAIConfig>,
    model: String,
}

impl OpenAICompatibleClient {
    /// Creates a new client for an OpenAI-compatible service.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration for the OpenAI client, including API key and base URL.
    /// * `model` - The specific model identifier to use for chat completions (e.g., "gpt-4o").
    pub fn new(config: OpenAIConfig, model: String) -> Self {
        Self {
            client: Client::with_config(config),
            model,
        }
    }
}

#[async_trait]
impl LLMClient for OpenAICompatibleClient {
    async fn decide_action(
        &self,
        _system_prompt: String,
        history_with_user_message: Vec<ChatCompletionRequestMessage>,
        tools: Vec<ChatCompletionTool>,
    ) -> Result<LLMAction> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(history_with_user_message)
            .tools(tools)
            .tool_choice("auto")
            .build()?;

        let response: CreateChatCompletionResponse = self.client.chat().create(request).await?;
        let choice = &response.choices[0];

        if let Some(tool_calls) = &choice.message.tool_calls {
            Ok(LLMAction::ToolCall(tool_calls.clone()))
        } else if let Some(content) = &choice.message.content {
            Ok(LLMAction::TextResponse(content.clone()))
        } else {
            Err(anyhow!(
                "LLM response had neither text content nor tool calls."
            ))
        }
    }

    async fn stream_after_tools(
        &self,
        _system_prompt: String,
        history_with_tool_results: Vec<ChatCompletionRequestMessage>,
    ) -> Result<LLMStream> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(history_with_tool_results)
            .stream(true)
            .build()?;

        let stream = self.client.chat().create_stream(request).await?;

        Ok(Box::pin(stream.filter_map(|result| async {
            match result {
                Ok(response) => {
                    let choice = &response.choices[0];
                    if let Some(content) = &choice.delta.content {
                        if !content.is_empty() {
                            return Some(Ok(LLMStreamEvent::TextChunk(content.clone())));
                        }
                    }
                    None
                }
                Err(e) => Some(Err(e)),
            }
        })))
    }
}
