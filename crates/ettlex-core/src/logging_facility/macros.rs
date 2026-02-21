//! Canonical logging macros
//!
//! These macros provide a structured, consistent way to log operations.

/// Log the start of an operation
///
/// # Example
///
/// ```
/// # use ettlex_core::log_op_start;
/// log_op_start!("create_ettle");
/// log_op_start!("create_ettle", ettle_id = "e123");
/// ```
#[macro_export]
macro_rules! log_op_start {
    ($op:expr) => {
        tracing::info!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_START,
        );
    };
    ($op:expr, $($field:tt)*) => {
        tracing::info!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_START,
            $($field)*
        );
    };
}

/// Log the successful end of an operation
///
/// # Example
///
/// ```
/// # use ettlex_core::log_op_end;
/// log_op_end!("create_ettle", duration_ms = 42);
/// ```
#[macro_export]
macro_rules! log_op_end {
    ($op:expr, duration_ms = $duration:expr) => {
        tracing::info!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_END,
            duration_ms = $duration,
        );
    };
    ($op:expr, duration_ms = $duration:expr, $($field:tt)*) => {
        tracing::info!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_END,
            duration_ms = $duration,
            $($field)*
        );
    };
}

/// Log an operation error
///
/// # Example
///
/// ```ignore
/// # use ettlex_core::{log_op_error, errors::EttleXError};
/// let err = EttleXError::EttleNotFound { ettle_id: "e1".to_string() };
/// log_op_error!("read_ettle", err, duration_ms = 10);
/// ```
#[macro_export]
macro_rules! log_op_error {
    ($op:expr, $err:expr, duration_ms = $duration:expr) => {{
        use $crate::errors::ExError;
        let ex_err: ExError = $err.into();
        tracing::error!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_END_ERROR,
            duration_ms = $duration,
            err_kind = ?ex_err.kind(),
            err_code = ex_err.code(),
        );
    }};
    ($op:expr, $err:expr, duration_ms = $duration:expr, $($field:tt)*) => {{
        use $crate::errors::ExError;
        let ex_err: ExError = $err.into();
        tracing::error!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_END_ERROR,
            duration_ms = $duration,
            err_kind = ?ex_err.kind(),
            err_code = ex_err.code(),
            $($field)*
        );
    }};
}
