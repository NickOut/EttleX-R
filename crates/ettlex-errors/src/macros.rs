//! Test assertion macros for ExError

/// Assert that an error has the expected kind.
///
/// # Panics
///
/// Panics if the error kind does not match.
#[macro_export]
macro_rules! assert_err_kind {
    ($err:expr, $kind:expr) => {
        assert_eq!(
            $err.kind(),
            $kind,
            "Expected error kind {:?} but got {:?}",
            $kind,
            $err.kind()
        );
    };
}

/// Assert that a specific field of an ExError has the expected value.
///
/// Supported fields: `entity_id`, `op`, `message`.
#[macro_export]
macro_rules! assert_err_field {
    ($err:expr, entity_id, $expected:expr) => {
        assert_eq!(
            $err.entity_id(),
            Some($expected),
            "Expected entity_id {:?} but got {:?}",
            $expected,
            $err.entity_id()
        );
    };
    ($err:expr, op, $expected:expr) => {
        assert_eq!(
            $err.op(),
            Some($expected),
            "Expected op {:?} but got {:?}",
            $expected,
            $err.op()
        );
    };
    ($err:expr, message, $expected:expr) => {
        assert_eq!($err.message(), $expected);
    };
}
