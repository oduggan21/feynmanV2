//! Defines the WebSocket message protocol between the browser client and the API server.

use crate::models;
use feynman_core::agent::FeynmanAgent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages sent from the client (browser) to the server.
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Initializes or resumes a session. This must be the first message.
    #[serde(rename = "init")]
    Init {
        /// The main topic for the teaching session.
        topic: String,
        /// The unique identifier of the session to resume.
        session_id: Option<Uuid>,
    },
    /// A text message from the user to the agent.
    #[serde(rename = "user_message")]
    UserMessage { text: String },
    /// Toggles the voice input/output feature.
    #[serde(rename = "set_voice_enabled")]
    SetVoiceEnabled { enabled: bool },
}

/// Messages sent from the server to the client (browser).
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Confirms successful session initialization and provides the initial state.
    Initialized {
        session_id: Uuid,
        agent_state: FeynmanAgent,
        history: Vec<models::Message>,
    },
    /// Pushes a complete, updated agent state to the client.
    StateUpdate { state: FeynmanAgent },
    /// Reports a fatal error to the client.
    Error { message: String },
    /// Signals the beginning of a streamed text response from the AI.
    ResponseStart,
    /// A chunk of a streamed text response.
    ResponseChunk { chunk: String },
    /// Signals the end of a streamed text response.
    ResponseEnd,
    /// An update on the user's speech-to-text transcription.
    TranscriptionUpdate { text: String, is_final: bool },
    /// A chunk of audio data (base64 encoded PCM16) for the AI's voice.
    AudioChunk { data: String },
    /// Signals that the AI has started speaking.
    AiSpeakingStart,
    /// Signals that the AI has finished speaking.
    AiSpeakingEnd,
}
