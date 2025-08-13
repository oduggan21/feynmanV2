//! Manages the primary WebSocket connection lifecycle for an agent session.

use super::{
    cycle::handle_react_cycle,
    protocol::{ClientMessage, ServerMessage},
    provider,
};
use crate::{models, state::AppState};
use anyhow::{Context, Result, anyhow};
use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use feynman_core::agent::{FeynmanAgent, FeynmanService};
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::{
    sync::{Mutex, mpsc},
    task::JoinHandle,
};
use tracing::{Instrument, error, info, instrument, warn};
use uuid::Uuid;

/// Axum handler to upgrade an HTTP connection to a WebSocket.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Main handler for an individual WebSocket connection.
///
/// This function is the entry point for a new connection. It performs the initial
/// handshake to initialize the session state and then spawns the main agent
/// session loop.
#[instrument(name = "ws_session", skip_all, fields(session_id))]
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let temp_id: u32 = rand::random();
    tracing::Span::current().record("session_id", &temp_id.to_string());
    info!("New WebSocket connection. Awaiting initialization...");

    let (socket_tx, mut socket_rx) = socket.split();
    let socket_tx_arc = Arc::new(Mutex::new(socket_tx));

    // The first message from the client must be an `init` message.
    let (session_id, topic, agent_state, history) =
        if let Some(Ok(ws_msg)) = socket_rx.next().await {
            match ws_msg {
                Message::Text(text) => initialize_session_state(&text, &state).await,
                _ => Err(anyhow!("First message was not a text `init` message.")),
            }
        } else {
            info!("Client disconnected before sending init message.");
            return;
        }
        .unwrap_or_else(|e| {
            // If initialization fails, send an error and terminate.
            error!("Session initialization failed: {:?}", e);
            let socket_tx = socket_tx_arc.clone();
            tokio::spawn(async move {
                let mut sink = socket_tx.lock().await;
                let _ = send_msg(
                    &mut sink,
                    ServerMessage::Error {
                        message: e.to_string(),
                    },
                )
                .await;
            });
            // Return dummy values to signal termination.
            (
                Uuid::nil(),
                String::new(),
                FeynmanAgent::new("".into(), vec![]),
                vec![],
            )
        });

    // If session_id is nil, initialization failed, so we stop.
    if session_id.is_nil() {
        return;
    }

    // Send the `Initialized` message to the client to confirm success.
    if send_msg(
        &mut *socket_tx_arc.lock().await,
        ServerMessage::Initialized {
            session_id,
            agent_state: agent_state.clone(),
            history: history.clone(),
        },
    )
    .await
    .is_err()
    {
        error!("Failed to send Initialized message to client.");
        return;
    }

    // Spawn the main session loop in a separate, instrumented task.
    let session_span = tracing::info_span!("agent_runtime", %session_id, %topic);
    tokio::spawn(
        async move {
            if let Err(e) = run_agent_session(
                state,
                socket_tx_arc,
                socket_rx,
                session_id,
                agent_state,
                history,
            )
            .await
            {
                error!(error = ?e, "Agent session terminated with error.");
            }
            info!("Agent session finished.");
        }
        .instrument(session_span),
    );
}

/// Parses the `init` message and loads the corresponding session state from the database.
async fn initialize_session_state(
    init_text: &str,
    state: &Arc<AppState>,
) -> Result<(Uuid, String, FeynmanAgent, Vec<models::Message>)> {
    let init_msg: ClientMessage = serde_json::from_str(init_text)?;
    let (topic, session_id) = if let ClientMessage::Init { topic, session_id } = init_msg {
        (
            topic,
            session_id.context("`session_id` is required for `init`")?,
        )
    } else {
        return Err(anyhow!("First message must be `init`"));
    };

    tracing::Span::current().record("topic", &topic);
    tracing::Span::current().record("session_id", &session_id.to_string());
    info!("Resuming existing session");

    let agent_state = state
        .db
        .get_latest_agent_state(session_id)
        .await?
        .context("Session state not found")?;
    let history = state.db.get_session_messages(session_id).await?;
    Ok((session_id, topic, agent_state, history))
}

/// The main event loop for an active WebSocket session.
///
/// This function listens for messages from the client, updates from the agent's
/// internal state, and orchestrates the interaction between them.
async fn run_agent_session(
    state: Arc<AppState>,
    socket_tx: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    mut socket_rx: SplitStream<WebSocket>,
    session_id: Uuid,
    agent_state: FeynmanAgent,
    mut history: Vec<models::Message>,
) -> Result<()> {
    let agent_state_arc = Arc::new(tokio::sync::Mutex::new(agent_state));
    let (state_update_tx, mut state_update_rx) = mpsc::channel(8);
    let feynman_service = FeynmanService::new(agent_state_arc.clone(), Some(state_update_tx));
    let (server_transport, client_transport) = tokio::io::duplex(4096);

    // Spawn the agent's tool-handling service.
    let agent_tool_handle = tokio::spawn(async move {
        if let Ok(service) = feynman_service.serve(server_transport).await {
            let _ = service.waiting().await;
        }
    });
    let mcp_client = ().serve(client_transport).await?;

    let mut realtime_tx: Option<mpsc::Sender<provider::RealtimeClientEvent>> = None;
    let mut realtime_task_handle: Option<JoinHandle<()>> = None;

    loop {
        tokio::select! {
            // Handle messages from the client WebSocket.
            Some(msg_result) = socket_rx.next() => {
                match msg_result {
                    Ok(ws_msg) => match ws_msg {
                        Message::Text(text) => {
                            if let Ok(msg) = serde_json::from_str::<ClientMessage>(&text) {
                                match msg {
                                    ClientMessage::UserMessage { text } => {
                                        handle_react_cycle(&state, session_id, &mut history, &agent_state_arc, &mcp_client, &text, &socket_tx, &realtime_tx).await?;
                                    }
                                    ClientMessage::SetVoiceEnabled { enabled } => {
                                        if enabled {
                                            if let Some(handle) = realtime_task_handle.take() { handle.abort(); }
                                            let (tx, handle) = provider::start_realtime_provider(state.clone(), socket_tx.clone()).await?;
                                            realtime_tx = Some(tx);
                                            realtime_task_handle = Some(handle);
                                        } else {
                                            if let Some(handle) = realtime_task_handle.take() {
                                                handle.abort();
                                                info!("Aborted realtime provider task.");
                                            }
                                            realtime_tx = None;
                                            info!("Voice disabled by client.");
                                        }
                                    }
                                    _ => warn!("Ignoring unexpected text message post-init."),
                                }
                            }
                        },
                        Message::Binary(data) => {
                            if let Some(tx) = &realtime_tx {
                               if let Err(e) = tx.send(provider::RealtimeClientEvent::Audio(data.into())).await {
                                   error!("Failed to send audio to provider task: {}", e);
                               }
                            } else {
                                warn!("Received audio data from client, but no voice provider is active.");
                            }
                        },
                        Message::Close(_) => {
                            info!("Client sent close frame. Shutting down session.");
                            break;
                        },
                        Message::Ping(_) | Message::Pong(_) => {},
                    },
                    Err(e) => {
                        error!("Error receiving from client WebSocket: {:?}", e);
                        break;
                    }
                }
            },
            // Handle state updates from the agent's internal logic.
            Some(new_state) = state_update_rx.recv() => {
                state.db.update_agent_state(session_id, &new_state).await?;
                send_msg(&mut *socket_tx.lock().await, ServerMessage::StateUpdate { state: new_state }).await?;
            },
            // If all channels close, exit the loop.
            else => break,
        }
    }

    // Clean up background tasks on exit.
    if let Some(handle) = realtime_task_handle.take() {
        handle.abort();
    }
    agent_tool_handle.abort();
    info!("WebSocket connection closed and agent session terminated.");
    Ok(())
}

/// A helper function to serialize and send a `ServerMessage` to the client.
pub(crate) async fn send_msg(
    socket_tx: &mut SplitSink<WebSocket, Message>,
    msg: ServerMessage,
) -> Result<()> {
    let serialized = serde_json::to_string(&msg)?;
    socket_tx.send(Message::Text(serialized.into())).await?;
    Ok(())
}
