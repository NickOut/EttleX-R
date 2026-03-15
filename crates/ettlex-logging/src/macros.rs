//! Canonical logging macros
//!
//! These macros provide a structured, consistent way to log operations.

/// Log the start of an operation
///
/// # Example
///
/// ```
/// # use ettlex_logging::log_op_start;
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
/// # use ettlex_logging::log_op_end;
/// log_op_end!("create_ettle", duration_ms = 42u64);
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
/// # use ettlex_logging::log_op_error;
/// # use ettlex_errors::{ExError, ExErrorKind};
/// let err = ExError::new(ExErrorKind::NotFound).with_message("not found");
/// log_op_error!("read_ettle", err, duration_ms = 10u64);
/// ```
#[macro_export]
macro_rules! log_op_error {
    ($op:expr, $err:expr, duration_ms = $duration:expr) => {{
        let ex_err: ettlex_errors::ExError = $err.into();
        tracing::error!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_END_ERROR,
            duration_ms = $duration,
            "err.kind" = ?ex_err.kind(),
            "err.code" = ex_err.code(),
        );
    }};
    ($op:expr, $err:expr, duration_ms = $duration:expr, $($field:tt)*) => {{
        let ex_err: ettlex_errors::ExError = $err.into();
        tracing::error!(
            component = module_path!(),
            op = $op,
            event = ettlex_core_types::schema::EVENT_END_ERROR,
            duration_ms = $duration,
            "err.kind" = ?ex_err.kind(),
            "err.code" = ex_err.code(),
            $($field)*
        );
    }};
}
