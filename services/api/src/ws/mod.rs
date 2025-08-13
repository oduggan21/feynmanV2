//! WebSocket Session Management
//!
//! This module contains the core logic for handling real-time agent sessions
//! over WebSockets. It is structured into submodules for clarity:
//!
//! - `protocol`: Defines the JSON-based message format for client-server communication.
//! - `session`: Manages the WebSocket connection lifecycle, from handshake to termination.
//! - `cycle`: Implements the agent's "ReAct" (Reason-Act) logic for processing user input.
//! - `provider`: Handles connections to third-party real-time voice APIs (OpenAI, Gemini).

mod cycle;
pub mod protocol;
mod provider;
pub mod session;

pub use session::ws_handler;
