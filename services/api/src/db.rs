//! Data Access Layer
//!
//! This module contains all the functions for interacting with the PostgreSQL database.
//! It uses `sqlx` for compile-time checked queries and robust connection pooling.

use anyhow::Result;
use feynman_core::agent::FeynmanAgent;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Message, MessageRole, Session, SessionStatus};

/// A wrapper around the `PgPool` to provide a clear data access interface.
#[derive(Clone)]
pub struct Db {
    pool: PgPool,
}

impl Db {
    /// Creates a new `Db` instance.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Runs all pending `sqlx` migrations.
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Creates a new session and its initial agent state in a single transaction.
    pub async fn create_session(
        &self,
        user_id: &str,
        topic: &str,
        initial_state: &FeynmanAgent,
    ) -> Result<Session> {
        let mut tx = self.pool.begin().await?;

        let session = sqlx::query_as!(
            Session,
            r#"
            INSERT INTO sessions (user_id, topic)
            VALUES ($1, $2)
            RETURNING id, user_id, topic, status as "status: _", created_at, updated_at
            "#,
            user_id,
            topic
        )
        .fetch_one(&mut *tx)
        .await?;

        let state_json = serde_json::to_value(initial_state)?;

        sqlx::query!(
            "INSERT INTO agent_states (session_id, state_json) VALUES ($1, $2)",
            session.id,
            state_json
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(session)
    }

    /// Retrieves a single session by its ID, scoped to a specific user.
    pub async fn get_session(&self, session_id: Uuid, user_id: &str) -> Result<Option<Session>> {
        let session = sqlx::query_as!(
            Session,
            r#"
            SELECT id, user_id, topic, status as "status: _", created_at, updated_at
            FROM sessions
            WHERE id = $1 AND user_id = $2
            "#,
            session_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(session)
    }

    /// Lists all sessions for a given user, ordered by most recent.
    pub async fn list_sessions(&self, user_id: &str) -> Result<Vec<Session>> {
        let sessions = sqlx::query_as!(
            Session,
            r#"
            SELECT id, user_id, topic, status as "status: _", created_at, updated_at
            FROM sessions
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(sessions)
    }

    /// Adds a new message to a session's conversation history.
    pub async fn add_message(
        &self,
        session_id: Uuid,
        role: MessageRole,
        content: &str,
    ) -> Result<Message> {
        let message = sqlx::query_as!(
            Message,
            r#"
            INSERT INTO messages (session_id, role, content)
            VALUES ($1, $2, $3)
            RETURNING id, session_id, role as "role: _", content, created_at
            "#,
            session_id,
            role as _,
            content
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(message)
    }

    /// Retrieves the full message history for a session, ordered chronologically.
    pub async fn get_session_messages(&self, session_id: Uuid) -> Result<Vec<Message>> {
        let messages = sqlx::query_as!(
            Message,
            r#"
            SELECT id, session_id, role as "role: _", content, created_at
            FROM messages
            WHERE session_id = $1
            ORDER BY created_at ASC
            "#,
            session_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(messages)
    }

    /// Retrieves the most recent agent state for a session.
    pub async fn get_latest_agent_state(&self, session_id: Uuid) -> Result<Option<FeynmanAgent>> {
        let record = sqlx::query!(
            "SELECT state_json FROM agent_states WHERE session_id = $1 ORDER BY created_at DESC LIMIT 1",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;

        match record {
            Some(rec) => {
                let agent: FeynmanAgent = serde_json::from_value(rec.state_json)?;
                Ok(Some(agent))
            }
            None => Ok(None),
        }
    }

    /// Persists a new version of the agent's state.
    pub async fn update_agent_state(&self, session_id: Uuid, state: &FeynmanAgent) -> Result<()> {
        let state_json = serde_json::to_value(state)?;
        sqlx::query!(
            "INSERT INTO agent_states (session_id, state_json) VALUES ($1, $2)",
            session_id,
            state_json
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Updates the status of a session (e.g., from 'active' to 'ended').
    pub async fn update_session_status(
        &self,
        session_id: Uuid,
        status: SessionStatus,
    ) -> Result<Session> {
        let session = sqlx::query_as!(
            Session,
            r#"
            UPDATE sessions
            SET status = $1
            WHERE id = $2
            RETURNING id, user_id, topic, status as "status: _", created_at, updated_at
            "#,
            status as _,
            session_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(session)
    }
}
