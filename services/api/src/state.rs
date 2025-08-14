//! Shared Application State
//!
//! This module defines the `AppState` struct, which holds all shared,
//! clonable resources like database pools and service clients.

use crate::config::Config;
use feynman_core::{curriculum::CurriculumService, llm_client::LLMClient};
use std::sync::Arc;

/// The shared application state, created once at startup and passed to all handlers.
/// All fields are public to be accessible from other modules.
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<crate::db::Db>,
    pub curriculum_service: Arc<dyn CurriculumService>,
    pub llm_client: Arc<dyn LLMClient>,
    pub system_prompt: Arc<String>,
    pub config: Arc<Config>,
}
