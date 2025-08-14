//! Axum Handlers for the REST API
//!
//! This module contains the logic for handling HTTP requests for session management.
//! It uses `utoipa` doc comments to generate OpenAPI documentation.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
};
use feynman_core::topic::SubTopic;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

use crate::{
    models::{
        CreateSessionPayload, ErrorResponse, MessageRole, Session, UpdateSessionStatusPayload,
    },
    state::AppState,
};

pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    InternalServerError(anyhow::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, Json(ErrorResponse { message })).into_response()
            }
            ApiError::NotFound(message) => {
                (StatusCode::NOT_FOUND, Json(ErrorResponse { message })).into_response()
            }
            ApiError::InternalServerError(err) => {
                error!("Internal Server Error: {:?}", err);
                let message = "An internal server error occurred.".to_string();
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { message }),
                )
                    .into_response()
            }
        }
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::InternalServerError(err.into())
    }
}

/// Create a new Feynman teaching session.
#[utoipa::path(
    post,
    path = "/sessions",
    request_body = CreateSessionPayload,
    responses(
        (status = 201, description = "Session created successfully", body = Session),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("x-user-id" = String, Header, description = "The ID of the user creating the session")
    )
)]
pub async fn create_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateSessionPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("x-user-id header is required".to_string()))?;

    let subtopic_names = state
        .curriculum_service
        .generate_subtopics(&payload.topic)
        .await?;

    let subtopics: Vec<SubTopic> = subtopic_names.into_iter().map(SubTopic::new).collect();

    let initial_state = feynman_core::agent::FeynmanAgent::new(payload.topic.clone(), subtopics);

    let session = state
        .db
        .create_session(user_id, &payload.topic, &initial_state)
        .await?;

    let first_subtopic = initial_state
        .incomplete_subtopics
        .values()
        .next()
        .map(|st| st.name.clone())
        .unwrap_or_else(|| "the first topic".to_string());

    let welcome_message = format!(
        "Hello! I'm ready to learn about {}. It looks like our first topic is '{}'. Could you start by telling me what that is?",
        payload.topic, first_subtopic
    );

    state
        .db
        .add_message(session.id, MessageRole::Ai, &welcome_message)
        .await?;

    Ok((StatusCode::CREATED, Json(session)))
}

/// List all sessions for a user.
#[utoipa::path(
    get,
    path = "/sessions",
    responses(
        (status = 200, description = "List of sessions", body = [Session]),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("x-user-id" = String, Header, description = "The ID of the user")
    )
)]
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Session>>, ApiError> {
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("x-user-id header is required".to_string()))?;
    let sessions = state.db.list_sessions(user_id).await?;
    Ok(Json(sessions))
}

/// Get a specific session by its ID.
#[utoipa::path(
    get,
    path = "/sessions/{id}",
    responses(
        (status = 200, description = "Session details", body = Session),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "Session ID"),
        ("x-user-id" = String, Header, description = "The ID of the user")
    )
)]
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("x-user-id header is required".to_string()))?;

    let session = state
        .db
        .get_session(id, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Session with id '{}' not found", id)))?;

    Ok((StatusCode::OK, Json(session)))
}

/// Update the status of a session.
#[utoipa::path(
    patch,
    path = "/sessions/{id}/status",
    request_body = UpdateSessionStatusPayload,
    responses(
        (status = 200, description = "Session status updated successfully", body = Session),
        (status = 404, description = "Session not found"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    params(
        ("id" = Uuid, Path, description = "Session ID"),
        ("x-user-id" = String, Header, description = "The ID of the user")
    )
)]
pub async fn update_session_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateSessionStatusPayload>,
) -> Result<impl IntoResponse, ApiError> {
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("x-user-id header is required".to_string()))?;

    // First, ensure the session exists and belongs to the user.
    let _ = state
        .db
        .get_session(id, user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Session with id '{}' not found", id)))?;

    // Now, update the status.
    let updated_session = state.db.update_session_status(id, payload.status).await?;

    Ok((StatusCode::OK, Json(updated_session)))
}
