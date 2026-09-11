/// Error returned by the `ViaPost` SDK.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Invalid SDK configuration or request data.
    #[error("validation error: {0}")]
    Validation(String),

    /// An HTTP transport error occurred.
    #[error("ViaPost request failed before receiving a response")]
    Transport(#[source] reqwest::Error),

    /// The configured deadline for the complete operation, including retries, expired.
    #[error("ViaPost request exceeded the configured {timeout:?} operation timeout")]
    Timeout {
        /// Configured deadline for the complete SDK operation.
        timeout: std::time::Duration,
    },

    /// The `ViaPost` API returned a non-success status.
    #[error("ViaPost API request failed with status {status}: {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// API error code, when returned.
        code: Option<String>,
        /// Safe API-provided error message.
        message: String,
        /// Correlation identifier, when returned.
        request_id: Option<String>,
    },

    /// The decoded response exceeded the configured limit.
    #[error("ViaPost response exceeded the configured {limit}-byte limit")]
    ResponseTooLarge {
        /// Maximum allowed decoded response size.
        limit: usize,
    },

    /// A successful response could not be decoded as the expected type.
    #[error("ViaPost response did not match the expected schema")]
    Decode(#[source] serde_json::Error),

    /// A request body could not be serialized.
    #[error("ViaPost request could not be encoded")]
    Encode(#[source] serde_json::Error),
}
