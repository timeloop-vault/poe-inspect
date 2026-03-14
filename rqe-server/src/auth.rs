use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

/// API key extractor. Validates `X-API-Key` header against `RQE_API_KEY` env var.
///
/// If `RQE_API_KEY` is not set or empty, auth is disabled (all requests pass).
/// Add `_auth: ApiKey` to handler parameters to require authentication.
///
/// Designed for future swap: replace internals with JWT/OAuth validation
/// without changing handler signatures.
pub struct ApiKey;

impl<S> FromRequestParts<S> for ApiKey
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let expected = match std::env::var("RQE_API_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ => return Ok(Self), // Auth disabled
        };

        let provided = parts.headers.get("x-api-key").and_then(|v| v.to_str().ok());

        match provided {
            Some(key) if key == expected => Ok(Self),
            Some(_) => Err(AuthError::InvalidKey),
            None => Err(AuthError::MissingKey),
        }
    }
}

pub enum AuthError {
    MissingKey,
    InvalidKey,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            Self::MissingKey => (StatusCode::UNAUTHORIZED, "missing X-API-Key header"),
            Self::InvalidKey => (StatusCode::FORBIDDEN, "invalid API key"),
        };
        (status, msg).into_response()
    }
}
