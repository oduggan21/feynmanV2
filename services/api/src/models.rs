//! API and Database Models
//!
//! This module defines the core data structures used for both database mapping
//! with `sqlx` and for generating OpenAPI documentation with `utoipa`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::fmt;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(sqlx::Type, Debug, Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq)]
#[sqlx(type_name = "session_status", rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Ended,
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize, ToSchema, Clone, Copy, PartialEq)]
#[sqlx(type_name = "message_role", rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Ai,
}

// Implement Display for easy conversion to a string, useful for logging and debugging.
impl fmt::Display for MessageRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Ai => write!(f, "ai"),
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, FromRow, Debug, Clone)]
pub struct Session {
    #[schema(value_type = String, format = Uuid)]
    pub id: Uuid,
    pub user_id: String,
    pub topic: String,
    #[schema(value_type = String, example = "active")]
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, ToSchema, FromRow, Debug, Clone)]
pub struct Message {
    pub id: i64,
    #[schema(value_type = String, format = Uuid)]
    pub session_id: Uuid,
    #[schema(value_type = String, example = "user")]
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateSessionPayload {
    #[schema(example = "Quantum Mechanics")]
    pub topic: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateSessionStatusPayload {
    #[schema(example = "ended")]
    pub status: SessionStatus,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json;

    #[test]
    fn test_session_status_serialization() {
        // Test serialization
        let active = SessionStatus::Active;
        let ended = SessionStatus::Ended;

        let active_json = serde_json::to_string(&active).unwrap();
        let ended_json = serde_json::to_string(&ended).unwrap();

        assert_eq!(active_json, "\"Active\"");
        assert_eq!(ended_json, "\"Ended\"");
    }

    #[test]
    fn test_session_status_deserialization() {
        // Test deserialization
        let active: SessionStatus = serde_json::from_str("\"Active\"").unwrap();
        let ended: SessionStatus = serde_json::from_str("\"Ended\"").unwrap();

        assert_eq!(active, SessionStatus::Active);
        assert_eq!(ended, SessionStatus::Ended);
    }

    #[test]
    fn test_session_status_equality() {
        assert_eq!(SessionStatus::Active, SessionStatus::Active);
        assert_eq!(SessionStatus::Ended, SessionStatus::Ended);
        assert_ne!(SessionStatus::Active, SessionStatus::Ended);
    }

    #[test]
    fn test_message_role_serialization() {
        let user = MessageRole::User;
        let ai = MessageRole::Ai;

        let user_json = serde_json::to_string(&user).unwrap();
        let ai_json = serde_json::to_string(&ai).unwrap();

        assert_eq!(user_json, "\"User\"");
        assert_eq!(ai_json, "\"Ai\"");
    }

    #[test]
    fn test_message_role_deserialization() {
        let user: MessageRole = serde_json::from_str("\"User\"").unwrap();
        let ai: MessageRole = serde_json::from_str("\"Ai\"").unwrap();

        assert_eq!(user, MessageRole::User);
        assert_eq!(ai, MessageRole::Ai);
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(format!("{}", MessageRole::User), "user");
        assert_eq!(format!("{}", MessageRole::Ai), "ai");
    }

    #[test]
    fn test_session_serialization() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let session = Session {
            id: session_id,
            user_id: "test_user_123".to_string(),
            topic: "Quantum Physics".to_string(),
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("Quantum Physics"));
        assert!(json.contains("test_user_123"));
        assert!(json.contains("Active"));

        // Test round-trip serialization
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, session.id);
        assert_eq!(deserialized.user_id, session.user_id);
        assert_eq!(deserialized.topic, session.topic);
        assert_eq!(deserialized.status, session.status);
    }

    #[test]
    fn test_message_serialization() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let message = Message {
            id: 42,
            session_id,
            role: MessageRole::User,
            content: "What is quantum entanglement?".to_string(),
            created_at: now,
        };

        let json = serde_json::to_string(&message).unwrap();
        assert!(json.contains("What is quantum entanglement?"));
        assert!(json.contains("User"));
        assert!(json.contains("42"));

        // Test round-trip serialization
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, message.id);
        assert_eq!(deserialized.session_id, message.session_id);
        assert_eq!(deserialized.role, message.role);
        assert_eq!(deserialized.content, message.content);
    }

    #[test]
    fn test_create_session_payload_deserialization() {
        let json = r#"{"topic": "Machine Learning Basics"}"#;
        let payload: CreateSessionPayload = serde_json::from_str(json).unwrap();

        assert_eq!(payload.topic, "Machine Learning Basics");
    }

    #[test]
    fn test_create_session_payload_missing_field() {
        let json = r#"{}"#;
        let result: Result<CreateSessionPayload, _> = serde_json::from_str(json);

        assert!(result.is_err()); // Should fail because topic is required
    }

    #[test]
    fn test_update_session_status_payload_deserialization() {
        let json_active = r#"{"status": "Active"}"#;
        let json_ended = r#"{"status": "Ended"}"#;

        let payload_active: UpdateSessionStatusPayload = serde_json::from_str(json_active).unwrap();
        let payload_ended: UpdateSessionStatusPayload = serde_json::from_str(json_ended).unwrap();

        assert_eq!(payload_active.status, SessionStatus::Active);
        assert_eq!(payload_ended.status, SessionStatus::Ended);
    }

    #[test]
    fn test_error_response_serialization() {
        let error = ErrorResponse {
            message: "Session not found".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("Session not found"));

        let expected = r#"{"message":"Session not found"}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn test_session_clone() {
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let session = Session {
            id: session_id,
            user_id: "test_user".to_string(),
            topic: "Test Topic".to_string(),
            status: SessionStatus::Active,
            created_at: now,
            updated_at: now,
        };

        let cloned = session.clone();
        assert_eq!(session.id, cloned.id);
        assert_eq!(session.user_id, cloned.user_id);
        assert_eq!(session.topic, cloned.topic);
        assert_eq!(session.status, cloned.status);
    }

    #[test]
    fn test_message_clone() {
        let message = Message {
            id: 1,
            session_id: Uuid::new_v4(),
            role: MessageRole::Ai,
            content: "Hello!".to_string(),
            created_at: Utc::now(),
        };

        let cloned = message.clone();
        assert_eq!(message.id, cloned.id);
        assert_eq!(message.session_id, cloned.session_id);
        assert_eq!(message.role, cloned.role);
        assert_eq!(message.content, cloned.content);
    }

    #[test]
    fn test_debug_formatting() {
        let session = Session {
            id: Uuid::new_v4(),
            user_id: "debug_test".to_string(),
            topic: "Debug Test".to_string(),
            status: SessionStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let debug_str = format!("{:?}", session);
        assert!(debug_str.contains("Session"));
        assert!(debug_str.contains("debug_test"));
        assert!(debug_str.contains("Debug Test"));
    }

    #[test]
    fn test_enum_copy_trait() {
        let status1 = SessionStatus::Active;
        let status2 = status1; // This should work because SessionStatus implements Copy

        assert_eq!(status1, status2);

        let role1 = MessageRole::User;
        let role2 = role1; // This should work because MessageRole implements Copy

        assert_eq!(role1, role2);
    }

    #[test]
    fn test_invalid_enum_deserialization() {
        // Test invalid SessionStatus
        let invalid_status = r#""Invalid""#;
        let result: Result<SessionStatus, _> = serde_json::from_str(invalid_status);
        assert!(result.is_err());

        // Test invalid MessageRole
        let invalid_role = r#""InvalidRole""#;
        let result: Result<MessageRole, _> = serde_json::from_str(invalid_role);
        assert!(result.is_err());
    }

    #[test]
    fn test_datetime_handling() {
        // Test with a specific datetime
        let specific_time = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();

        let session = Session {
            id: Uuid::new_v4(),
            user_id: "time_test".to_string(),
            topic: "Time Test".to_string(),
            status: SessionStatus::Active,
            created_at: specific_time,
            updated_at: specific_time,
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.created_at, specific_time);
        assert_eq!(deserialized.updated_at, specific_time);
    }

    #[test]
    fn test_uuid_handling() {
        // Test with a specific UUID
        let specific_uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();

        let session = Session {
            id: specific_uuid,
            user_id: "uuid_test".to_string(),
            topic: "UUID Test".to_string(),
            status: SessionStatus::Ended,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, specific_uuid);
    }
}
