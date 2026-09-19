//! Official asynchronous Rust SDK for the `ViaPost` API.
//!
//! ```no_run
//! # async fn example() -> Result<(), viapost::Error> {
//! use viapost::{SendRequest, ViaPost};
//!
//! let client = ViaPost::new(std::env::var("VIAPOST_API_KEY").unwrap())?;
//! let request = SendRequest::new("hello@example.com", ["person@example.com"])
//!     .subject("Hello from Rust");
//! let result = client.send().create(&request, Some("order-123")).await?;
//! println!("accepted: {}", result.accepted.len());
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod models;
mod resources;

pub use client::{ClientBuilder, ViaPost, DEFAULT_BASE_URL, DEFAULT_MAX_RESPONSE_BYTES};
pub use error::Error;
pub use models::*;
pub use resources::{
    AutomationsResource, ContactsResource, DomainsResource, MessagesResource, SegmentsResource,
    SendResource, TemplatesResource, UsageResource, WebhooksResource,
};

/// Crate version embedded in the SDK user agent.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
