//! Configuration management for the navia library
//!
//! Provides runtime configuration for core library behavior.

use crate::error::{NaviaError, NaviaResult, ValidationError};
use serde::{Deserialize, Serialize};

/// Logging level configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Error messages only
    Error,
    /// Informational messages and errors
    Info,
    /// Debug messages and all above
    Debug,
}

impl Default for LogLevel {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        return LogLevel::Debug;

        #[cfg(not(debug_assertions))]
        return LogLevel::Info;
    }
}

/// Core configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaviaConfig {
    /// Logging level
    pub log_level: LogLevel,

    /// Maximum message size in bytes
    pub max_message_size: usize,

    /// Minimum seed length in bytes for security
    pub min_seed_length: usize,
}

impl Default for NaviaConfig {
    fn default() -> Self {
        Self {
            log_level: LogLevel::default(),
            max_message_size: 1_048_576, // 1MB
            min_seed_length: 32,
        }
    }
}

impl NaviaConfig {
    /// Create a development configuration
    pub fn development() -> Self {
        Self {
            log_level: LogLevel::Debug,
            min_seed_length: 16,
            ..Default::default()
        }
    }

    /// Create a production configuration
    pub fn production() -> Self {
        Self {
            log_level: LogLevel::Info,
            min_seed_length: 32,
            ..Default::default()
        }
    }

    /// Load configuration from environment
    pub fn from_env() -> Self {
        match std::env::var("NAVIA_ENV").as_deref() {
            Ok("production") | Ok("prod") => Self::production(),
            Ok("development") | Ok("dev") => Self::development(),
            _ => {
                #[cfg(debug_assertions)]
                return Self::development();

                #[cfg(not(debug_assertions))]
                return Self::production();
            }
        }
    }

    /// Validate configuration values
    pub fn validate(&self) -> NaviaResult<()> {
        if self.max_message_size == 0 {
            return Err(NaviaError::Validation(
                ValidationError::InvalidMessageFormat {
                    details: "max_message_size must be greater than 0".to_string(),
                },
            ));
        }

        if self.min_seed_length < 16 {
            return Err(NaviaError::Validation(ValidationError::InvalidSeed {
                reason: "min_seed_length must be at least 16 bytes".to_string(),
            }));
        }

        Ok(())
    }
}

/// Global configuration instance
static CONFIG: once_cell::sync::OnceCell<parking_lot::RwLock<NaviaConfig>> =
    once_cell::sync::OnceCell::new();

/// Initialize the global configuration
pub fn init_config(config: NaviaConfig) -> NaviaResult<()> {
    config.validate()?;

    CONFIG
        .set(parking_lot::RwLock::new(config))
        .map_err(|_| NaviaError::External("Configuration already initialized".to_string()))?;

    Ok(())
}

/// Get a reference to the global configuration
pub fn get_config() -> &'static parking_lot::RwLock<NaviaConfig> {
    CONFIG.get_or_init(|| parking_lot::RwLock::new(NaviaConfig::from_env()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NaviaConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_development_config() {
        let config = NaviaConfig::development();
        assert_eq!(config.log_level, LogLevel::Debug);
        assert_eq!(config.min_seed_length, 16);
    }

    #[test]
    fn test_production_config() {
        let config = NaviaConfig::production();
        assert_eq!(config.log_level, LogLevel::Info);
        // Production config has stricter requirements
        assert_eq!(config.min_seed_length, 32);
    }

    #[test]
    fn test_config_validation() {
        let mut config = NaviaConfig::default();

        config.max_message_size = 0;
        assert!(config.validate().is_err());

        config.max_message_size = 1024;
        config.min_seed_length = 8;
        assert!(config.validate().is_err());
    }
}
