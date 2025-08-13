//! Handles the real-time WebSocket connection to OpenAI for voice interaction.

use super::RealtimeClientEvent;
use crate::{
    audio_utils,
    state::AppState,
    ws::{protocol::ServerMessage, session::send_msg},
};
use anyhow::{Context, Result};
use async_openai::types::realtime::{
    self as oai_realtime, ClientEvent as OAIClientEvent, ServerEvent as OAIServerEvent,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{client::IntoClientRequest, protocol::Message as WsMessage},
};
use tracing::info;

/// Runs the main loop for the OpenAI Realtime API connection.
///
/// This function connects to the OpenAI WebSocket, handles session setup,
/// and then enters a loop to proxy messages between our client and OpenAI.
pub async fn run(
    state: &Arc<AppState>,
    mut rx: mpsc::Receiver<RealtimeClientEvent>,
    socket_tx: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> Result<()> {
    let url = "wss://api.openai.com/v1/realtime?model=gpt-4o-realtime-preview-2024-10-01";
    let api_key = state
        .config
        .openai_api_key
        .as_ref()
        .context("OpenAI API key not found")?;

    let mut request = url.into_client_request()?;
    request
        .headers_mut()
        .insert("Authorization", format!("Bearer {}", api_key).parse()?);
    request
        .headers_mut()
        .insert("OpenAI-Beta", "realtime=v1".parse()?);

    let (ws_stream, _) = connect_async(request)
        .await
        .context("Failed to connect to OpenAI Realtime WebSocket")?;
    let (mut openai_tx, mut openai_rx) = ws_stream.split();
    info!("Connected to OpenAI Realtime API.");

    // Configure the real-time session parameters.
    let session_config = oai_realtime::SessionResource {
        model: Some("gpt-4o-realtime-preview-2024-10-01".to_string()),
        modalities: Some(vec!["text".to_string(), "audio".to_string()]),
        voice: Some(oai_realtime::RealtimeVoice::Alloy),
        input_audio_format: Some(oai_realtime::AudioFormat::PCM16),
        output_audio_format: Some(oai_realtime::AudioFormat::PCM16),
        input_audio_transcription: Some(oai_realtime::AudioTranscription {
            model: Some("whisper-1".to_string()),
            ..Default::default()
        }),
        turn_detection: Some(oai_realtime::TurnDetection::ServerVAD {
            threshold: 0.5,
            prefix_padding_ms: 200,
            silence_duration_ms: 700,
            interrupt_response: Some(true),
            create_response: Some(true),
        }),
        ..Default::default()
    };
    let event = OAIClientEvent::SessionUpdate(oai_realtime::SessionUpdateEvent {
        session: session_config,
        event_id: None,
    });
    openai_tx
        .send(WsMessage::Text(serde_json::to_string(&event)?.into()))
        .await?;

    // Main event loop for the OpenAI connection.
    loop {
        tokio::select! {
            biased;
            // Handle events from our application (e.g., audio to send).
            Some(event) = rx.recv() => {
                match event {
                    RealtimeClientEvent::Audio(data) => {
                        let audio_i16: Vec<i16> = data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
                        let encoded_audio = audio_utils::encode_i16(&audio_i16);
                        let append_event = oai_realtime::InputAudioBufferAppendEvent { audio: encoded_audio, event_id: None };
                        openai_tx.send(WsMessage::Text(serde_json::to_string(&OAIClientEvent::InputAudioBufferAppend(append_event))?.into())).await?;
                    }
                    RealtimeClientEvent::TextToSpeak(text) => {
                         let item = oai_realtime::Item {
                             r#type: Some(oai_realtime::ItemType::Message),
                             role: Some(oai_realtime::ItemRole::System),
                             content: Some(vec![oai_realtime::ItemContent {
                                 r#type: oai_realtime::ItemContentType::InputText,
                                 text: Some(text), audio: None, transcript: None,
                             }]),
                             id: None, status: None, call_id: None, name: None, arguments: None, output: None
                         };
                         let create_event = oai_realtime::ConversationItemCreateEvent { item, event_id: None, previous_item_id: None };
                         openai_tx.send(WsMessage::Text(serde_json::to_string(&OAIClientEvent::ConversationItemCreate(create_event))?.into())).await?;

                         let response_event = oai_realtime::ResponseCreateEvent{ response: None, event_id: None };
                         openai_tx.send(WsMessage::Text(serde_json::to_string(&OAIClientEvent::ResponseCreate(response_event))?.into())).await?;
                    }
                }
            },
            // Handle events from the OpenAI server (e.g., audio to play).
            Some(msg_result) = openai_rx.next() => {
                if let Ok(WsMessage::Text(text)) = msg_result {
                    if let Ok(server_event) = serde_json::from_str::<OAIServerEvent>(&text) {
                        let mut sink = socket_tx.lock().await;
                        match server_event {
                            OAIServerEvent::ConversationItemInputAudioTranscriptionDelta(e) => send_msg(&mut sink, ServerMessage::TranscriptionUpdate { text: e.delta, is_final: false }).await?,
                            OAIServerEvent::ConversationItemInputAudioTranscriptionCompleted(e) => send_msg(&mut sink, ServerMessage::TranscriptionUpdate { text: e.transcript, is_final: true }).await?,
                            OAIServerEvent::ResponseAudioDelta(e) => send_msg(&mut sink, ServerMessage::AudioChunk { data: e.delta }).await?,
                            OAIServerEvent::InputAudioBufferSpeechStarted(_) => send_msg(&mut sink, ServerMessage::AiSpeakingStart).await?,
                            OAIServerEvent::InputAudioBufferSpeechStopped(_) => send_msg(&mut sink, ServerMessage::AiSpeakingEnd).await?,
                            OAIServerEvent::ResponseDone(_) => send_msg(&mut sink, ServerMessage::AiSpeakingEnd).await?,
                            OAIServerEvent::Error(e) => send_msg(&mut sink, ServerMessage::Error { message: e.error.message }).await?,
                            _ => {}
                        }
                    }
                }
            },
        }
    }
}
