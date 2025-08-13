pub mod agent;
pub mod curriculum;
pub mod generic_types;
pub mod llm_client;
pub mod realtime_api;
pub mod topic;

/// Represents commands that the core logic issues to an external runtime.
///
/// This enum is the primary API for decoupling the agent's decision-making
/// from the runtime's execution of side effects (like speaking text or
/// finalizing a session).
#[derive(Debug, Clone)]
pub enum Command {
    /// Command the runtime to speak the given text to the user.
    SpeakText(String),
    /// Command indicating the session is complete, with a final message.
    SessionComplete(String),
}
