//! Feynman API Library Crate
//!
//! This library contains all the core logic for the Feynman web service,
//! including the application state, database access, API handlers, WebSocket
//! logic, and routing. The `main.rs` binary is a thin wrapper around this library.

pub mod audio_utils;
pub mod config;
pub mod db;
pub mod handlers;
pub mod models;
pub mod router;
pub mod state;
pub mod ws;
