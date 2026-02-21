//! Logging initialization module
//!
//! Provides a single initialization point for the logging facility.

use std::sync::Once;
use tracing_subscriber::{util::SubscriberInitExt, EnvFilter};

/// Logging profile configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Human-readable output for development
    Development,
    /// JSON structured output for production
    Production,
    /// Test capture mode for deterministic testing
    Test,
}

static INIT_ONCE: Once = Once::new();

/// Initialize the logging facility
///
/// This function should be called once at application startup.
/// It sets up the tracing subscriber based on the selected profile.
///
/// # Profiles
///
/// - **Development**: Human-readable logs with debug level
/// - **Production**: JSON structured logs with info level
/// - **Test**: Capture mode for test assertions
///
/// # Example
///
/// ```
/// use ettlex_core::logging_facility::{init, Profile};
///
/// init(Profile::Development);
/// ```
pub fn init(profile: Profile) {
    INIT_ONCE.call_once(|| {
        match profile {
            Profile::Development => {
                tracing_subscriber::fmt()
                    .with_env_filter(
                        EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| EnvFilter::new("ettlex=debug")),
                    )
                    .init();
            }
            Profile::Production => {
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(
                        EnvFilter::try_from_default_env()
                            .unwrap_or_else(|_| EnvFilter::new("ettlex=info")),
                    )
                    .init();
            }
            Profile::Test => {
                // Test capture is initialized separately via init_test_capture()
                // This branch is a no-op for the standard init() path
                tracing_subscriber::registry().init();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_idempotent() {
        // Multiple calls should not panic
        init(Profile::Test);
        init(Profile::Test);
        init(Profile::Test);
    }

    #[test]
    fn test_profile_equality() {
        assert_eq!(Profile::Development, Profile::Development);
        assert_ne!(Profile::Development, Profile::Production);
    }
}
