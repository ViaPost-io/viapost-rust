use std::{
    fmt,
    net::IpAddr,
    str::FromStr,
    time::{Duration, SystemTime},
};

use futures_util::StreamExt;
use reqwest::{
    header::{
        HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_ENCODING, CONTENT_LENGTH, USER_AGENT,
    },
    Method, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    error::Error,
    resources::{
        AutomationsResource, DomainsResource, MessagesResource, SendResource, TemplatesResource,
        UsageResource, WebhooksResource,
    },
    VERSION,
};

pub const DEFAULT_BASE_URL: &str = "https://api.viapost.io/";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_RETRIES: u8 = 2;
const MAX_BACKOFF_DELAY: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct ViaPost {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: Url,
    pub(crate) headers: HeaderMap,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u8,
    pub(crate) max_response_bytes: usize,
}

impl fmt::Debug for ViaPost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ViaPost")
            .field("base_url", &self.base_url.as_str())
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("max_response_bytes", &self.max_response_bytes)
            .field("api_key", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl ViaPost {
    pub fn new(api_key: impl Into<String>) -> Result<Self, Error> {
        Self::builder(api_key).build()
    }

    pub fn builder(api_key: impl Into<String>) -> ClientBuilder {
        ClientBuilder::new(api_key)
    }

    pub fn send(&self) -> SendResource<'_> {
        SendResource::new(self)
    }
    pub fn messages(&self) -> MessagesResource<'_> {
        MessagesResource::new(self)
    }
    pub fn domains(&self) -> DomainsResource<'_> {
        DomainsResource::new(self)
    }
    pub fn templates(&self) -> TemplatesResource<'_> {
        TemplatesResource::new(self)
    }
    pub fn webhooks(&self) -> WebhooksResource<'_> {
        WebhooksResource::new(self)
    }
    pub fn automations(&self) -> AutomationsResource<'_> {
        AutomationsResource::new(self)
    }
    pub fn usage(&self) -> UsageResource<'_> {
        UsageResource::new(self)
    }

    pub(crate) fn body<T: Serialize + ?Sized>(value: &T) -> Result<Value, Error> {
        serde_json::to_value(value).map_err(Error::Encode)
    }

    pub(crate) async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<Value>,
        body: Option<Value>,
        extra_headers: Option<HeaderMap>,
    ) -> Result<T, Error> {
        tokio::time::timeout(
            self.timeout,
            self.request_with_retries(method, path, query, body, extra_headers),
        )
        .await
        .map_err(|_| Error::Timeout {
            timeout: self.timeout,
        })?
    }

    async fn request_with_retries<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<Value>,
        body: Option<Value>,
        extra_headers: Option<HeaderMap>,
    ) -> Result<T, Error> {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|_| {
                Error::Validation("request path could not be joined to the base URL".into())
            })?;
        let retryable = matches!(method, Method::GET | Method::HEAD);
        let mut attempt = 0_u8;

        loop {
            let mut headers = self.headers.clone();
            if let Some(extra) = &extra_headers {
                headers.extend(extra.clone());
            }
            let mut request = self
                .http
                .request(method.clone(), url.clone())
                .headers(headers);
            if let Some(value) = &query {
                request = request.query(value);
            }
            if let Some(value) = &body {
                request = request.json(value);
            }
            let response = request.send().await.map_err(Error::Transport)?;
            let status = response.status();
            let response_headers = response.headers().clone();
            let bytes = read_limited(response, self.max_response_bytes).await?;

            if retryable && is_retryable(status) && attempt < self.max_retries {
                let delay = retry_delay(&response_headers, attempt);
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
                continue;
            }

            if !status.is_success() {
                let api_key = self
                    .headers
                    .get(AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.strip_prefix("Bearer "));
                return Err(api_error(status, &response_headers, &bytes, api_key));
            }
            if bytes.is_empty() {
                return serde_json::from_value(Value::Null).map_err(Error::Decode);
            }
            return serde_json::from_slice(&bytes).map_err(Error::Decode);
        }
    }

    pub(crate) async fn request_empty(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<(), Error> {
        let _: Value = self.request(method, path, None, body, None).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct ClientBuilder {
    api_key: String,
    base_url: Result<Url, String>,
    timeout: Duration,
    max_retries: u8,
    max_response_bytes: usize,
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientBuilder")
            .field("api_key", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("max_response_bytes", &self.max_response_bytes)
            .finish()
    }
}

impl ClientBuilder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: parse_base_url(DEFAULT_BASE_URL),
            timeout: DEFAULT_TIMEOUT,
            max_retries: DEFAULT_MAX_RETRIES,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn base_url(mut self, base_url: impl AsRef<str>) -> Result<Self, Error> {
        self.base_url = parse_base_url(base_url.as_ref());
        if let Err(message) = &self.base_url {
            return Err(Error::Validation(message.clone()));
        }
        Ok(self)
    }

    pub fn timeout(mut self, timeout: Duration) -> Result<Self, Error> {
        if timeout.is_zero() {
            return Err(Error::Validation(
                "timeout must be greater than zero".into(),
            ));
        }
        self.timeout = timeout;
        Ok(self)
    }

    pub fn max_retries(mut self, max_retries: u8) -> Self {
        self.max_retries = max_retries.min(5);
        self
    }

    pub fn max_response_bytes(mut self, limit: usize) -> Result<Self, Error> {
        if limit == 0 {
            return Err(Error::Validation(
                "max_response_bytes must be greater than zero".into(),
            ));
        }
        self.max_response_bytes = limit;
        Ok(self)
    }

    pub fn build(self) -> Result<ViaPost, Error> {
        let key = self.api_key.trim();
        if key.is_empty() {
            return Err(Error::Validation("api_key must not be empty".into()));
        }
        let authorization = HeaderValue::from_str(&format!("Bearer {key}"))
            .map_err(|_| Error::Validation("api_key contains invalid header characters".into()))?;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, authorization);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&format!("viapost-rust/{VERSION}"))
                .map_err(|_| Error::Validation("invalid SDK user agent".into()))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers.clone())
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(Error::Transport)?;

        Ok(ViaPost {
            http,
            base_url: self.base_url.map_err(Error::Validation)?,
            headers,
            timeout: self.timeout,
            max_retries: self.max_retries,
            max_response_bytes: self.max_response_bytes,
        })
    }
}

fn parse_base_url(raw: &str) -> Result<Url, String> {
    let mut url = Url::parse(raw).map_err(|_| "base_url must be an absolute URL".to_owned())?;
    if url.username() != ""
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("base_url must not contain credentials, query, or fragment".into());
    }
    let secure = url.scheme() == "https";
    let loopback = url.scheme() == "http" && is_loopback(&url);
    if !secure && !loopback {
        return Err(
            "base_url must use HTTPS; HTTP is allowed only for loopback development".into(),
        );
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

fn is_loopback(url: &Url) -> bool {
    match url.host_str() {
        Some("localhost") => true,
        Some(host) => IpAddr::from_str(host).is_ok_and(|ip| ip.is_loopback()),
        None => false,
    }
}

async fn read_limited(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, Error> {
    let headers = response.headers();
    let encoded = headers
        .get(CONTENT_ENCODING)
        .is_some_and(|value| value != "identity");
    if !encoded {
        if let Some(length) = headers
            .get(CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
        {
            if length > limit {
                return Err(Error::ResponseTooLarge { limit });
            }
        }
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(Error::Transport)?;
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(Error::ResponseTooLarge { limit });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn is_retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn retry_delay(headers: &HeaderMap, attempt: u8) -> Duration {
    if let Some(value) = headers.get("retry-after").and_then(|v| v.to_str().ok()) {
        if let Ok(seconds) = value.parse::<u64>() {
            return Duration::from_secs(seconds);
        }
        if let Ok(target) = httpdate::parse_http_date(value) {
            if let Ok(delay) = target.duration_since(SystemTime::now()) {
                return delay;
            }
        }
    }
    Duration::from_millis(100_u64.saturating_mul(1_u64 << attempt.min(4))).min(MAX_BACKOFF_DELAY)
}

fn api_error(
    status: StatusCode,
    headers: &HeaderMap,
    bytes: &[u8],
    api_key: Option<&str>,
) -> Error {
    let body = serde_json::from_slice(bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes).into_owned()));
    let error = body.get("error");
    let code = error
        .and_then(|value| value.get("code"))
        .and_then(Value::as_str)
        .map(|value| redact(value, api_key));
    let message = error
        .and_then(|value| {
            value
                .as_str()
                .or_else(|| value.get("message").and_then(Value::as_str))
        })
        .unwrap_or("ViaPost API request failed")
        .to_owned();
    let message = redact(&message, api_key);
    let request_id = headers
        .get("x-request-id")
        .or_else(|| headers.get("x-correlation-id"))
        .and_then(|value| value.to_str().ok())
        .map(|value| redact(value, api_key))
        .or_else(|| {
            error
                .and_then(|value| value.get("request_id"))
                .and_then(Value::as_str)
                .map(|value| redact(value, api_key))
        });
    Error::Api {
        status: status.as_u16(),
        code,
        message,
        request_id,
    }
}

fn redact(value: &str, secret: Option<&str>) -> String {
    match secret.filter(|secret| !secret.is_empty()) {
        Some(secret) => value.replace(secret, "[REDACTED]"),
        None => value.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_is_honored_beyond_the_backoff_cap() {
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("30"));

        assert_eq!(retry_delay(&headers, 0), Duration::from_secs(30));
    }
}
