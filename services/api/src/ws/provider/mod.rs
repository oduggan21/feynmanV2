//! Manages real-time, provider-specific WebSocket connections for voice I/O.

pub mod gemini;
pub mod openai;

use super::{protocol::ServerMessage, session::send_msg};
use crate::{config::Provider, state::AppState};
use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use bytes::Bytes;
use futures_util::stream::SplitSink;
use std::sync::Arc;
use tokio::{
    sync::{mpsc, Mutex},
    task::JoinHandle,
};
use tracing::error;

/// An internal event passed to the active real-time provider task.
#[derive(Debug)]
pub enum RealtimeClientEvent {
    /// A chunk of audio data from the client.
    Audio(Bytes),
    /// Text that the AI should speak.
    TextToSpeak(String),
}

/// Starts a new task for the configured real-time provider (OpenAI or Gemini).
///
/// This function sets up a channel for communication and spawns a Tokio task
/// that will run the provider-specific logic.
///
/// # Returns
/// A tuple containing:
/// 1. A `mpsc::Sender` to send `RealtimeClientEvent`s to the provider task.
/// 2. A `JoinHandle` for the spawned task.
pub async fn start_realtime_provider(
    state: Arc<AppState>,
    socket_tx: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> Result<(mpsc::Sender<RealtimeClientEvent>, JoinHandle<()>)> {
    let (tx, rx) = mpsc::channel(128);
    let provider_config = state.config.provider.clone();

    let handle = tokio::spawn(async move {
        let result = match provider_config {
            Provider::OpenAI => openai::run(&state, rx, socket_tx.clone()).await,
            Provider::Gemini => gemini::run(&state, rx, socket_tx.clone()).await,
        };
        if let Err(e) = result {
            error!(?provider_config, error = ?e, "Realtime provider task failed");
            let mut sink = socket_tx.lock().await;
            let _ = send_msg(
                &mut sink,
                ServerMessage::Error {
                    message: format!("Voice connection failed: {}", e),
                },
            )
            .await;
        }
    });

    Ok((tx, handle))
}