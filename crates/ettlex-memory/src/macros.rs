//! Logging macro wrappers for downstream consumers.
//!
//! These macros provide structured logging for agent-api operations.
//! They inline the `tracing` calls directly so downstream crates only need
//! `ettlex-memory` as a workspace dependency.

/// Log the start of an operation.
#[macro_export]
macro_rules! log_op_start {
    ($op:expr) => {
        ::tracing::info!(
            component = ::std::module_path!(),
            op = $op,
            event = "start",
        );
    };
    ($op:expr, $($field:tt)*) => {
        ::tracing::info!(
            component = ::std::module_path!(),
            op = $op,
            event = "start",
            $($field)*
        );
    };
}

/// Log the successful end of an operation.
#[macro_export]
macro_rules! log_op_end {
    ($op:expr, duration_ms = $duration:expr) => {
        ::tracing::info!(
            component = ::std::module_path!(),
            op = $op,
            event = "end",
            duration_ms = $duration,
        );
    };
    ($op:expr, duration_ms = $duration:expr, $($field:tt)*) => {
        ::tracing::info!(
            component = ::std::module_path!(),
            op = $op,
            event = "end",
            duration_ms = $duration,
            $($field)*
        );
    };
}

/// Log an operation error.
#[macro_export]
macro_rules! log_op_error {
    ($op:expr, $err:expr, duration_ms = $duration:expr) => {{
        use ettlex_memory::ExError;
        let ex_err: ExError = $err.into();
        ::tracing::error!(
            component = ::std::module_path!(),
            op = $op,
            event = "end_error",
            duration_ms = $duration,
            "err.kind" = ?ex_err.kind(),
            "err.code" = ex_err.code(),
        );
    }};
    ($op:expr, $err:expr, duration_ms = $duration:expr, $($field:tt)*) => {{
        use ettlex_memory::ExError;
        let ex_err: ExError = $err.into();
        ::tracing::error!(
            component = ::std::module_path!(),
            op = $op,
            event = "end_error",
            duration_ms = $duration,
            "err.kind" = ?ex_err.kind(),
            "err.code" = ex_err.code(),
            $($field)*
        );
    }};
}
