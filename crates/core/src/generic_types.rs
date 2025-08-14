/// Generic configuration for initializing a real-time session with any provider.
#[derive(Debug, Clone, Default)]
pub struct GenericSessionConfig {
    pub instructions: String,
    // Add other common fields here in the future if needed.
}

/// Generic events that any real-time provider can emit back to the application.
#[derive(Debug, Clone)]
pub enum GenericServerEvent {
    /// A transcription of the user's speech.
    Transcription { text: String, is_final: bool },
    /// A chunk of spoken audio from the AI (base64 encoded).
    AudioChunk(String),
    /// A signal that the AI is about to start speaking.
    Speaking,
    /// A signal that the AI has finished speaking.
    SpeakingDone,
    /// An error from the provider.
    Error(String),
    /// The connection was closed.
    Closed,
}
