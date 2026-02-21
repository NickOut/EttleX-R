//! Sensitive data marker for automatic redaction
//!
//! The `Sensitive<T>` wrapper ensures that sensitive data (passwords,
//! tokens, API keys) is never accidentally logged or displayed.

use std::fmt;

/// Wrapper for sensitive data that redacts itself in Debug and Display
///
/// # Example
///
/// ```
/// use ettlex_core_types::Sensitive;
///
/// let password = Sensitive::new("secret123");
/// println!("{:?}", password); // Prints: ***REDACTED***
/// println!("{}", password);   // Prints: ***REDACTED***
///
/// // Access the actual value when needed
/// assert_eq!(password.expose(), &"secret123");
/// ```
pub struct Sensitive<T>(T);

impl<T> Sensitive<T> {
    /// Wrap a sensitive value
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Expose the underlying sensitive value
    ///
    /// Use this method sparingly and only when the sensitive data
    /// must be accessed (e.g., for authentication).
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Consume the wrapper and return the inner value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> fmt::Debug for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***REDACTED***")
    }
}

impl<T> fmt::Display for Sensitive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***REDACTED***")
    }
}

impl<T: Clone> Clone for Sensitive<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_debug_redaction() {
        let secret = Sensitive::new("my-secret-password");
        let debug_str = format!("{:?}", secret);
        assert_eq!(debug_str, "***REDACTED***");
        assert!(!debug_str.contains("my-secret-password"));
    }

    #[test]
    fn test_sensitive_display_redaction() {
        let secret = Sensitive::new("api-key-12345");
        let display_str = format!("{}", secret);
        assert_eq!(display_str, "***REDACTED***");
        assert!(!display_str.contains("api-key"));
    }

    #[test]
    fn test_sensitive_expose() {
        let secret = Sensitive::new(42);
        assert_eq!(secret.expose(), &42);
    }

    #[test]
    fn test_sensitive_into_inner() {
        let secret = Sensitive::new(String::from("test"));
        let inner = secret.into_inner();
        assert_eq!(inner, "test");
    }

    #[test]
    fn test_sensitive_clone() {
        let secret1 = Sensitive::new(String::from("test"));
        let secret2 = secret1.clone();
        assert_eq!(secret1.expose(), secret2.expose());
    }

    #[test]
    fn test_sensitive_with_struct() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct User {
            username: String,
            password: Sensitive<String>,
        }

        let user = User {
            username: "alice".to_string(),
            password: Sensitive::new("secret123".to_string()),
        };

        let debug_str = format!("{:?}", user);
        assert!(debug_str.contains("alice"));
        assert!(debug_str.contains("***REDACTED***"));
        assert!(!debug_str.contains("secret123"));
    }
}
