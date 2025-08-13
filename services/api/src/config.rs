use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::Level;

/// A custom error type for configuration loading failures.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingVar(String),
    #[error("Invalid value for environment variable {0}: {1}")]
    InvalidValue(String, String),
}

/// Defines the supported backend providers for the Curriculum service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Gemini,
}

/// Holds all configuration loaded from the environment at startup.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_address: SocketAddr,
    pub database_url: String,
    pub provider: Provider,
    pub openai_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub chat_model: String,
    pub log_level: Level,
    pub prompts_path: PathBuf,
}

impl Config {
    /// Loads configuration from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        // Only load from .env in non-test mode to avoid contamination
        if !cfg!(test) {
            dotenvy::dotenv().ok();
        }

        let bind_address_str =
            std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let bind_address = bind_address_str
            .parse::<SocketAddr>()
            .map_err(|e| ConfigError::InvalidValue("BIND_ADDRESS".to_string(), e.to_string()))?;

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingVar("DATABASE_URL".to_string()))?;

        let provider_str =
            std::env::var("REALTIME_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let provider = match provider_str.to_lowercase().as_str() {
            "gemini" => Provider::Gemini,
            _ => Provider::OpenAI,
        };

        let openai_api_key = std::env::var("OPENAI_API_KEY").ok();
        let gemini_api_key = std::env::var("GEMINI_API_KEY").ok();

        let chat_model = std::env::var("CHAT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        let log_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| "INFO".to_string());
        let log_level = log_level_str.parse::<Level>().map_err(|_| {
            ConfigError::InvalidValue(
                "RUST_LOG".to_string(),
                format!("'{}' is not a valid log level", log_level_str),
            )
        })?;

        let prompts_path = std::env::var("PROMPTS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./prompts"));

        match provider {
            Provider::OpenAI => {
                if openai_api_key.is_none() {
                    return Err(ConfigError::MissingVar(
                        "OPENAI_API_KEY must be set for 'openai' provider".to_string(),
                    ));
                }
            }
            Provider::Gemini => {
                if gemini_api_key.is_none() {
                    return Err(ConfigError::MissingVar(
                        "GEMINI_API_KEY must be set for 'gemini' provider".to_string(),
                    ));
                }
            }
        }

        Ok(Self {
            bind_address,
            database_url,
            provider,
            openai_api_key,
            gemini_api_key,
            chat_model,
            log_level,
            prompts_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use tracing::Level;

    fn clear_env_vars() {
        unsafe {
            env::remove_var("BIND_ADDRESS");
            env::remove_var("DATABASE_URL");
            env::remove_var("REALTIME_PROVIDER");
            env::remove_var("OPENAI_API_KEY");
            env::remove_var("GEMINI_API_KEY");
            env::remove_var("CHAT_MODEL");
            env::remove_var("RUST_LOG");
            env::remove_var("PROMPTS_PATH");
        }
    }

    fn set_minimal_env_openai() {
        unsafe {
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("REALTIME_PROVIDER", "openai");
            env::set_var("OPENAI_API_KEY", "test-openai-key");
        }
    }

    #[test]
    fn test_config_error_display() {
        let missing_var = ConfigError::MissingVar("TEST_VAR".to_string());
        assert_eq!(
            format!("{}", missing_var),
            "Missing environment variable: TEST_VAR"
        );

        let invalid_value =
            ConfigError::InvalidValue("TEST_VAR".to_string(), "bad_value".to_string());
        assert_eq!(
            format!("{}", invalid_value),
            "Invalid value for environment variable TEST_VAR: bad_value"
        );
    }

    #[test]
    fn test_provider_debug_and_clone() {
        let openai = Provider::OpenAI;
        let gemini = Provider::Gemini;

        assert!(format!("{:?}", openai).contains("OpenAI"));
        assert!(format!("{:?}", gemini).contains("Gemini"));

        let cloned = openai.clone();
        assert_eq!(openai, cloned);
    }

    #[test]
    #[serial]
    fn test_config_from_env_minimal_openai() {
        clear_env_vars();
        set_minimal_env_openai();

        let config = Config::from_env().expect("Config should load successfully");

        assert_eq!(config.bind_address.to_string(), "0.0.0.0:3000");
        assert_eq!(config.database_url, "postgresql://test:test@localhost/test");
        assert_eq!(config.provider, Provider::OpenAI);
        assert_eq!(config.openai_api_key, Some("test-openai-key".to_string()));
        assert_eq!(config.gemini_api_key, None);
        assert_eq!(config.chat_model, "gpt-4o");
        assert_eq!(config.log_level, Level::INFO);
        assert_eq!(config.prompts_path, PathBuf::from("./prompts"));
    }

    #[test]
    #[serial]
    fn test_config_from_env_gemini_provider() {
        clear_env_vars();
        unsafe {
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("REALTIME_PROVIDER", "gemini");
            env::set_var("GEMINI_API_KEY", "test-gemini-key");
        }

        let config = Config::from_env().expect("Config should load successfully");

        assert_eq!(config.provider, Provider::Gemini);
        assert_eq!(config.gemini_api_key, Some("test-gemini-key".to_string()));
        assert_eq!(config.openai_api_key, None);
    }

    #[test]
    #[serial]
    fn test_config_from_env_custom_values() {
        clear_env_vars();
        unsafe {
            env::set_var("BIND_ADDRESS", "127.0.0.1:8080");
            env::set_var(
                "DATABASE_URL",
                "postgresql://custom:custom@localhost/custom",
            );
            env::set_var("REALTIME_PROVIDER", "openai");
            env::set_var("OPENAI_API_KEY", "custom-openai-key");
            env::set_var("GEMINI_API_KEY", "custom-gemini-key");
            env::set_var("CHAT_MODEL", "gpt-3.5-turbo");
            env::set_var("RUST_LOG", "debug");
            env::set_var("PROMPTS_PATH", "/custom/prompts");
        }

        let config = Config::from_env().expect("Config should load successfully");

        assert_eq!(config.bind_address.to_string(), "127.0.0.1:8080");
        assert_eq!(
            config.database_url,
            "postgresql://custom:custom@localhost/custom"
        );
        assert_eq!(config.provider, Provider::OpenAI);
        assert_eq!(config.openai_api_key, Some("custom-openai-key".to_string()));
        assert_eq!(config.gemini_api_key, Some("custom-gemini-key".to_string()));
        assert_eq!(config.chat_model, "gpt-3.5-turbo");
        assert_eq!(config.log_level, Level::DEBUG);
        assert_eq!(config.prompts_path, PathBuf::from("/custom/prompts"));
    }

    #[test]
    #[serial]
    fn test_config_invalid_bind_address() {
        clear_env_vars();
        unsafe {
            env::set_var("BIND_ADDRESS", "not-a-valid-address");
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("OPENAI_API_KEY", "test-openai-key");
        }

        let err = Config::from_env().unwrap_err();
        match err {
            ConfigError::InvalidValue(var, _) => assert_eq!(var, "BIND_ADDRESS"),
            _ => panic!("Expected InvalidValue for BIND_ADDRESS"),
        }
    }

    #[test]
    #[serial]
    fn test_config_invalid_log_level() {
        clear_env_vars();
        unsafe {
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("OPENAI_API_KEY", "test-openai-key");
            env::set_var("RUST_LOG", "not-a-level");
        }

        let err = Config::from_env().unwrap_err();
        match err {
            ConfigError::InvalidValue(var, _) => assert_eq!(var, "RUST_LOG"),
            _ => panic!("Expected InvalidValue for RUST_LOG"),
        }
    }

    #[test]
    #[serial]
    fn test_config_missing_openai_key() {
        clear_env_vars();
        unsafe {
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("REALTIME_PROVIDER", "openai");
        }

        let err = Config::from_env().unwrap_err();
        match err {
            ConfigError::MissingVar(msg) => {
                assert!(msg.contains("OPENAI_API_KEY"));
            }
            _ => panic!("Expected MissingVar for OPENAI_API_KEY"),
        }
    }

    #[test]
    #[serial]
    fn test_config_missing_gemini_key() {
        clear_env_vars();
        unsafe {
            env::set_var("DATABASE_URL", "postgresql://test:test@localhost/test");
            env::set_var("REALTIME_PROVIDER", "gemini");
        }

        let err = Config::from_env().unwrap_err();
        match err {
            ConfigError::MissingVar(msg) => {
                assert!(msg.contains("GEMINI_API_KEY"));
            }
            _ => panic!("Expected MissingVar for GEMINI_API_KEY"),
        }
    }
}
