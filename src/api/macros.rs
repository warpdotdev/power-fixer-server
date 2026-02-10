//! Macros for reducing boilerplate in API response handling.

/// Returns a successful JSON response with StatusCode::OK.
///
/// # Example
/// ```ignore
/// json_ok!(MyResponse { field: value })
/// ```
#[macro_export]
macro_rules! json_ok {
    ($data:expr) => {
        (axum::http::StatusCode::OK, axum::extract::Json($data))
    };
}

/// Returns a JSON response with the specified status code.
///
/// # Example
/// ```ignore
/// json_err!(BAD_REQUEST, MyResponse { error: "invalid" })
/// ```
#[macro_export]
macro_rules! json_err {
    ($status:ident, $data:expr) => {
        (axum::http::StatusCode::$status, axum::extract::Json($data))
    };
}

/// Returns a successful GenericResponse with StatusCode::OK.
///
/// # Example
/// ```ignore
/// ok!("Operation completed")
/// ok!("Created item {}", id)
/// ```
#[macro_export]
macro_rules! ok {
    ($msg:expr) => {
        (
            axum::http::StatusCode::OK,
            axum::extract::Json($crate::api::types::GenericResponse {
                success: true,
                message: $msg.to_string(),
            }),
        )
    };
    ($fmt:expr, $($arg:tt)*) => {
        (
            axum::http::StatusCode::OK,
            axum::extract::Json($crate::api::types::GenericResponse {
                success: true,
                message: format!($fmt, $($arg)*),
            }),
        )
    };
}

/// Returns an error GenericResponse with the specified status code.
///
/// # Example
/// ```ignore
/// err!(INTERNAL_SERVER_ERROR, "Database error")
/// err!(BAD_REQUEST, "Invalid field: {}", field_name)
/// ```
#[macro_export]
macro_rules! err {
    ($status:ident, $msg:expr) => {
        (
            axum::http::StatusCode::$status,
            axum::extract::Json($crate::api::types::GenericResponse {
                success: false,
                message: $msg.to_string(),
            }),
        )
    };
    ($status:ident, $fmt:expr, $($arg:tt)*) => {
        (
            axum::http::StatusCode::$status,
            axum::extract::Json($crate::api::types::GenericResponse {
                success: false,
                message: format!($fmt, $($arg)*),
            }),
        )
    };
}
