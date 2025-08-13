//! Main Entrypoint for the Feynman API Service
//!
//! This binary is responsible for:
//! 1. Loading configuration from the environment.
//! 2. Initializing the database connection pool and running migrations.
//! 3. Initializing shared services (like the LLM and Curriculum clients).
//! 4. Constructing the Axum router and applying middleware.
//! 5. Starting the web server and handling graceful shutdown.

use anyhow::Context;
use async_openai::config::OpenAIConfig;
use feynman_api::{
    config::{Config, Provider},
    db::Db,
    router::create_router,
    state::AppState,
};
use feynman_core::{
    curriculum::{CurriculumService, LLMCurriculumService},
    llm_client::{LLMClient, OpenAICompatibleClient},
};
use sqlx::PgPool;
use std::{collections::HashMap, fs, net::SocketAddr, sync::Arc};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Listens for the `Ctrl+C` signal to gracefully shut down the server.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    info!("Received shutdown signal. Shutting down gracefully...");
}

/// A helper function to load prompts from a directory.
fn load_prompts(prompts_path: &std::path::Path) -> anyhow::Result<HashMap<String, String>> {
    let mut prompts = HashMap::new();
    for entry in std::fs::read_dir(prompts_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            let prompt_key = path
                .file_stem()
                .and_then(|s| s.to_str())
                .context("Could not get file stem")?
                .to_string();
            let content = fs::read_to_string(&path)?;
            prompts.insert(prompt_key, content);
        }
    }
    Ok(prompts)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // --- 1. Load Configuration ---
    let config = Config::from_env().context("Failed to load configuration")?;

    // --- 2. Initialize Logging ---
    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .init();
    info!("Configuration loaded. Initializing application state...");

    // --- 3. Initialize Database ---
    let pool = PgPool::connect(&config.database_url)
        .await
        .context("Failed to connect to database")?;
    let db = Arc::new(Db::new(pool));
    db.run_migrations().await?;
    info!("Database connection established and migrations are up-to-date.");

    // --- 4. Initialize Shared Services ---
    let prompts = load_prompts(&config.prompts_path)?;
    let system_prompt = Arc::new(
        prompts
            .get("system_prompt")
            .context("system_prompt.md not found in prompts directory")?
            .clone(),
    );

    let (curriculum_service, llm_client): (Arc<dyn CurriculumService>, Arc<dyn LLMClient>) =
        match &config.provider {
            Provider::OpenAI => {
                info!("Using OpenAI provider.");
                let api_key = config.openai_api_key.as_ref().unwrap();
                let openai_config = OpenAIConfig::new()
                    .with_api_key(api_key)
                    .with_api_base("https://api.openai.com/v1/");
                (
                    Arc::new(LLMCurriculumService::new(
                        openai_config.clone(),
                        config.chat_model.clone(),
                        prompts,
                    )),
                    Arc::new(OpenAICompatibleClient::new(
                        openai_config,
                        config.chat_model.clone(),
                    )),
                )
            }
            Provider::Gemini => {
                info!("Using Gemini provider.");
                let api_key = config.gemini_api_key.as_ref().unwrap();
                let openai_config = OpenAIConfig::new()
                    .with_api_key(api_key)
                    .with_api_base("https://generativelanguage.googleapis.com/v1beta/openai");

                (
                    Arc::new(LLMCurriculumService::new(
                        openai_config.clone(),
                        config.chat_model.clone(),
                        prompts,
                    )),
                    Arc::new(OpenAICompatibleClient::new(
                        openai_config,
                        config.chat_model.clone(),
                    )),
                )
            }
        };

    let app_state = Arc::new(AppState {
        db,
        curriculum_service,
        llm_client,
        system_prompt,
        config: Arc::new(config.clone()),
    });

    // --- 5. Create Router and Apply Middleware ---
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(app_state).layer(cors);

    // --- 6. Start Server ---
    info!(
        provider = ?config.provider,
        model = %config.chat_model,
        bind_address = %config.bind_address,
        "Service configured. Starting server..."
    );
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    info!("Server has shut down.");
    Ok(())
}
