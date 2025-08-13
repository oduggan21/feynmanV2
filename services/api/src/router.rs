//! Axum Router Configuration
//!
//! This module defines the complete HTTP routing for the application,
//! including the REST API, WebSocket endpoint, and OpenAPI documentation.

use crate::{
    handlers,
    models::{
        CreateSessionPayload, ErrorResponse, Message, MessageRole, Session, SessionStatus,
        UpdateSessionStatusPayload,
    },
    state::AppState,
    ws::ws_handler,
};

use axum::{
    Router,
    routing::{get, patch},
};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::create_session,
        handlers::list_sessions,
        handlers::get_session,
        handlers::update_session_status,
    ),
    components(
        schemas(Session, Message, CreateSessionPayload, UpdateSessionStatusPayload, ErrorResponse, SessionStatus, MessageRole)
    ),
    tags(
        (name = "Feynman API", description = "Session management for the Feynman teaching agent")
    )
)]
pub struct ApiDoc;

/// Creates the main Axum router for the application.
pub fn create_router(app_state: Arc<AppState>) -> Router {
    // Group all routes that require AppState into their own router.
    let api_router = Router::new()
        .route(
            "/sessions",
            get(handlers::list_sessions).post(handlers::create_session),
        )
        .route("/sessions/{id}", get(handlers::get_session))
        .route(
            "/sessions/{id}/status",
            patch(handlers::update_session_status),
        )
        .route("/ws", get(ws_handler))
        // Apply the state ONLY to this group of routes.
        .with_state(app_state);

    // Create the final router that merges the stateful routes
    // with the stateless routes (like Swagger UI).
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(api_router)
}
