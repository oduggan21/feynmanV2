//! Handles the real-time WebSocket connection to Google Gemini for voice interaction.

use super::RealtimeClientEvent;
use crate::{
    audio_utils,
    state::AppState,
    ws::{protocol::ServerMessage, session::send_msg},
};
use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use rubato::Resampler;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage};
use tracing::{error, info, warn};

// --- Local Gemini Realtime Types (for encapsulation) ---
mod gemini_realtime_types {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) enum ClientMessage {
        Setup(BidiGenerateContentSetup),
        RealtimeInput(BidiGenerateContentRealtimeInput),
        ClientContent(BidiGenerateContentClientContent),
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct BidiGenerateContentClientContent {
        pub turns: Vec<Content>,
        pub turn_complete: bool,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct BidiGenerateContentSetup {
        pub model: String,
        pub generation_config: GenerationConfig,
    }
    #[derive(Serialize)]
    pub(super) struct Content {
        pub role: String,
        pub parts: Vec<Part>,
    }
    #[derive(Serialize)]
    pub(super) struct Part {
        pub text: String,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct GenerationConfig {
        pub response_modalities: Vec<ResponseModality>,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "UPPERCASE")]
    pub(super) enum ResponseModality {
        Text,
        Audio,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct BidiGenerateContentRealtimeInput {
        pub audio: Blob,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct Blob {
        pub mime_type: String,
        pub data: String,
    }
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct ServerMessage {
        pub setup_complete: Option<serde_json::Value>,
        pub server_content: Option<LiveServerContent>,
    }
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct LiveServerContent {
        pub model_turn: Option<ServerContentTurn>,
        pub input_transcription: Option<ServerTranscription>,
        pub turn_complete: Option<bool>,
    }
    #[derive(Deserialize, Debug)]
    pub(super) struct ServerContentTurn {
        pub parts: Vec<ServerPart>,
    }
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct ServerPart {
        pub text: Option<String>,
        pub inline_data: Option<ServerBlob>,
    }
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct ServerBlob {
        pub data: String,
    }
    #[derive(Deserialize, Debug)]
    pub(super) struct ServerTranscription {
        pub text: String,
    }
}

/// Runs the main loop for the Gemini Realtime API connection.
///
/// This function connects to the Gemini WebSocket, handles the specific setup
/// protocol, and then enters a loop to proxy messages, performing audio
/// resampling as needed.
pub async fn run(
    state: &Arc<AppState>,
    mut rx: mpsc::Receiver<RealtimeClientEvent>,
    socket_tx: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> Result<()> {
    let api_key = state.config.gemini_api_key.clone().unwrap();
    let url = format!(
        "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key={}",
        api_key
    );

    let (ws_stream, _) = connect_async(url).await?;
    info!("Connected to Gemini Realtime WebSocket.");
    let (mut gemini_tx, mut gemini_rx) = ws_stream.split();

    // Create resamplers to convert between frontend and Gemini sample rates.
    let mut input_resampler = audio_utils::create_resampler(
        audio_utils::FRONTEND_AUDIO_PLAYER_SAMPLE_RATE,
        audio_utils::GEMINI_LIVE_API_PCM16_SAMPLE_RATE,
        512,
    )?;
    let mut output_resampler = audio_utils::create_resampler(
        audio_utils::GEMINI_LIVE_API_PCM16_SAMPLE_RATE,
        audio_utils::FRONTEND_AUDIO_PLAYER_SAMPLE_RATE,
        512,
    )?;

    // Send initial setup message.
    let setup_msg = gemini_realtime_types::ClientMessage::Setup(
        gemini_realtime_types::BidiGenerateContentSetup {
            model: "models/gemini-2.0-flash-exp".to_string(),
            generation_config: gemini_realtime_types::GenerationConfig {
                response_modalities: vec![gemini_realtime_types::ResponseModality::Audio],
            },
        },
    );
    gemini_tx
        .send(WsMessage::Text(serde_json::to_string(&setup_msg)?.into()))
        .await?;

    // Send the system prompt immediately after setup to complete the handshake.
    info!("Sending system prompt to Gemini to complete setup.");
    let system_prompt_turn = gemini_realtime_types::ClientMessage::ClientContent(
        gemini_realtime_types::BidiGenerateContentClientContent {
            turns: vec![gemini_realtime_types::Content {
                role: "system".to_string(),
                parts: vec![gemini_realtime_types::Part {
                    text: state.system_prompt.to_string(),
                }],
            }],
            turn_complete: false, // Keep the turn open for the user to speak
        },
    );
    let system_prompt_payload = serde_json::to_string(&system_prompt_turn)?;
    gemini_tx
        .send(WsMessage::Text(system_prompt_payload.into()))
        .await?;

    let mut is_ready = false;
    loop {
        tokio::select! {
            // Handle events from our application.
            Some(event) = rx.recv() => {
                if !is_ready {
                    warn!("Received client event before Gemini setup was complete. Ignoring.");
                    continue;
                }
                match event {
                    RealtimeClientEvent::Audio(data) => {
                        let pcm_i16: Vec<i16> = data.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
                        let pcm_f32 = audio_utils::convert_i16_to_f32(&pcm_i16);
                        let input_chunk_size = input_resampler.input_frames_next();
                        let mut resampled_f32 = Vec::new();
                        for chunk in pcm_f32.chunks(input_chunk_size) {
                            if let Ok(res) = input_resampler.process(&[chunk.to_vec()], None) {
                                resampled_f32.extend_from_slice(&res[0]);
                            }
                        }
                        let base64_data = audio_utils::encode_f32_to_base64_i16(&resampled_f32);
                        let audio_msg = gemini_realtime_types::ClientMessage::RealtimeInput(
                            gemini_realtime_types::BidiGenerateContentRealtimeInput {
                                audio: gemini_realtime_types::Blob {
                                    mime_type: "audio/pcm;rate=16000".to_string(),
                                    data: base64_data,
                                }
                            }
                        );
                        gemini_tx.send(WsMessage::Text(serde_json::to_string(&audio_msg)?.into())).await?;
                    }
                    RealtimeClientEvent::TextToSpeak(text) => {
                        let tts_msg = gemini_realtime_types::ClientMessage::ClientContent(
                            gemini_realtime_types::BidiGenerateContentClientContent {
                                turns: vec![gemini_realtime_types::Content {
                                    role: "model".to_string(),
                                    parts: vec![gemini_realtime_types::Part { text }],
                                }],
                                turn_complete: true,
                            }
                        );
                        gemini_tx.send(WsMessage::Text(serde_json::to_string(&tts_msg)?.into())).await?;
                    }
                 }
            },
            // Handle events from the Gemini server.
            Some(msg_result) = gemini_rx.next() => {
                match msg_result {
                    Ok(WsMessage::Text(text)) => {
                        if !is_ready {
                            // Wait for the `setup_complete` message.
                            match serde_json::from_str::<gemini_realtime_types::ServerMessage>(&text) {
                                Ok(gemini_msg) => {
                                    if gemini_msg.setup_complete.is_some() {
                                        info!("Gemini session setup is complete. Ready for bidirectional streaming.");
                                        is_ready = true;

                                        info!("Signaling start of user turn to Gemini.");
                                        let start_turn_msg = gemini_realtime_types::ClientMessage::ClientContent(
                                            gemini_realtime_types::BidiGenerateContentClientContent {
                                                turns: vec![],
                                                turn_complete: false,
                                            },
                                        );
                                        let start_turn_payload = serde_json::to_string(&start_turn_msg)?;
                                        gemini_tx.send(WsMessage::Text(start_turn_payload.into())).await?;
                                    } else {
                                        error!("Received unexpected JSON during Gemini setup: {:?}", gemini_msg);
                                    }
                                }
                                Err(_) => {
                                    error!("Failed to parse Gemini message during setup. Raw text: {}", text);
                                }
                            }
                        } else {
                            // Process regular content messages after setup.
                            if let Ok(gemini_msg) = serde_json::from_str::<gemini_realtime_types::ServerMessage>(&text) {
                                let mut sink = socket_tx.lock().await;
                                if let Some(content) = gemini_msg.server_content {
                                    if let Some(transcription) = content.input_transcription {
                                        send_msg(&mut sink, ServerMessage::TranscriptionUpdate { text: transcription.text, is_final: true }).await?;
                                    }
                                    if let Some(ref model_turn) = content.model_turn {
                                        for part in &model_turn.parts {
                                            if let Some(blob) = &part.inline_data {
                                                let pcm_f32 = audio_utils::decode_f32_from_base64_i16(&blob.data);
                                                let output_chunk_size = output_resampler.input_frames_next();
                                                let mut resampled_f32 = Vec::new();
                                                for chunk in pcm_f32.chunks(output_chunk_size) {
                                                    if let Ok(res) = output_resampler.process(&[chunk.to_vec()], None) {
                                                        resampled_f32.extend_from_slice(&res[0]);
                                                    }
                                                }
                                                let resampled_base64 = audio_utils::encode_f32_to_base64_i16(&resampled_f32);
                                                send_msg(&mut sink, ServerMessage::AudioChunk { data: resampled_base64 }).await?;
                                            }
                                        }
                                    }
                                     if content.turn_complete == Some(true) {
                                        send_msg(&mut sink, ServerMessage::AiSpeakingEnd).await?;
                                     } else if content.model_turn.is_some() {
                                        send_msg(&mut sink, ServerMessage::AiSpeakingStart).await?;
                                     }
                                }
                            }
                        }
                    },
                    Ok(WsMessage::Close(close_frame)) => {
                        error!(?close_frame, "Gemini WebSocket connection closed by server.");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from Gemini WebSocket: {}", e);
                        break;
                    }
                    _ => {}
                }
            },
        }
    }
    Ok(())
}
