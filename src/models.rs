use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON object used by extensible `ViaPost` graph/content fields.
pub type JsonObject = serde_json::Map<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EmailStream {
    #[default]
    Transactional,
    Marketing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    InvalidAddress,
    Suppressed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainStatus {
    Pending,
    Verified,
    Failed,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DnsPurpose {
    Dkim,
    Spf,
    Dmarc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateVersionStatus {
    Draft,
    Published,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TemplateVariableType {
    String,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutomationStatus {
    Disabled,
    Enabled,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AutomationRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    pub filename: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendRequest {
    pub from: String,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cc: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bcc: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "is_default_stream")]
    pub stream: EmailStream,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "JsonObject::is_empty")]
    pub metadata: JsonObject,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(default, skip_serializing_if = "JsonObject::is_empty")]
    pub variables: JsonObject,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,
}

fn is_default_stream(stream: &EmailStream) -> bool {
    *stream == EmailStream::Transactional
}

impl SendRequest {
    pub fn new(from: impl Into<String>, to: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            from: from.into(),
            to: to.into_iter().map(Into::into).collect(),
            from_name: None,
            reply_to: None,
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: None,
            html: None,
            text: None,
            stream: EmailStream::default(),
            tags: Vec::new(),
            metadata: JsonObject::new(),
            scheduled_at: None,
            template_id: None,
            variables: JsonObject::new(),
            attachments: Vec::new(),
        }
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AcceptedMessage {
    pub message_id: String,
    pub to: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RejectedMessage {
    pub to: String,
    pub reason: RejectionReason,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SendResult {
    #[serde(default, deserialize_with = "null_to_default")]
    pub accepted: Vec<AcceptedMessage>,
    #[serde(default, deserialize_with = "null_to_default")]
    pub rejected: Vec<RejectedMessage>,
}

fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}

#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub id: String,
    pub status: String,
    pub stream: EmailStream,
    pub from_address: String,
    pub to_address: String,
    pub subject: Option<String>,
    pub recipient_domain: String,
    pub api_key_id: Option<String>,
    pub created_at: String,
    pub scheduled_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub queued_at: Option<String>,
    pub sent_at: Option<String>,
    pub delivered_at: Option<String>,
    pub failed_at: Option<String>,
    pub suppressed_at: Option<String>,
    pub first_opened_at: Option<String>,
    pub first_clicked_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct MessageDetail {
    #[serde(flatten)]
    pub message: Message,
    pub body_html: Option<String>,
    pub body_plain: Option<String>,
    pub content_status: Option<String>,
    pub raw_message_api_path: Option<String>,
    pub content_variant: Option<String>,
}

impl fmt::Debug for MessageDetail {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessageDetail")
            .field("message", &self.message)
            .field("body_html", &self.body_html.as_ref().map(|_| "[REDACTED]"))
            .field(
                "body_plain",
                &self.body_plain.as_ref().map(|_| "[REDACTED]"),
            )
            .field("content_status", &self.content_status)
            .field("raw_message_api_path", &self.raw_message_api_path)
            .field("content_variant", &self.content_variant)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageList {
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub occurred_at: String,
    pub recipient: Option<String>,
    pub smtp_code: Option<i64>,
    pub enhanced_code: Option<String>,
    pub diagnostic: Option<String>,
    pub mx_host: Option<String>,
    pub click_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageEventList {
    pub events: Vec<MessageEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EngagementResponse {
    pub since: String,
    pub delivered: u64,
    pub opened: u64,
    pub clicked: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageTimeseriesDay {
    pub date: String,
    pub queued: u64,
    pub processing: u64,
    pub sent: u64,
    pub delivered: u64,
    pub deferred: u64,
    pub bounced: u64,
    pub failed: u64,
    pub rejected: u64,
    pub complained: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeseriesResponse {
    pub since: String,
    pub days: Vec<MessageTimeseriesDay>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSummary {
    pub total: u64,
    pub delivered: u64,
    pub opened: u64,
    pub clicked: u64,
    pub bounced: u64,
    pub complained: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsTimeseriesDay {
    pub date: String,
    pub delivered: u64,
    pub open: u64,
    pub click: u64,
    pub soft_bounce: u64,
    pub hard_bounce: u64,
    pub complaint: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainMetrics {
    pub domain_id: String,
    pub domain_name: String,
    pub sent: u64,
    pub delivered: u64,
    pub opened: u64,
    pub clicked: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetricsResponse {
    pub since: String,
    pub until: String,
    pub current: MetricsSummary,
    pub previous: MetricsSummary,
    pub timeseries: Vec<MetricsTimeseriesDay>,
    pub by_domain: Vec<DomainMetrics>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MessageListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DaysParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MetricsParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Domain {
    pub id: String,
    pub name: String,
    pub status: DomainStatus,
    pub spf_verified: bool,
    pub dkim_verified: bool,
    pub dmarc_verified: bool,
    pub return_path_subdomain: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainList {
    pub domains: Vec<Domain>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateDomainRequest {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DnsRecord {
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    pub value: String,
    pub purpose: DnsPurpose,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DnsRecordList {
    pub dns_records: Vec<DnsRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDomainResponse {
    pub domain: Domain,
    pub dns_records: Vec<DnsRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RotateDkimResponse {
    pub selector: String,
    pub public_key: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    pub name: String,
    pub var_type: TemplateVariableType,
    pub fallback_value: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailTemplate {
    pub id: String,
    pub name: String,
    pub current_draft_version_id: Option<String>,
    pub published_version_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailTemplateVersion {
    pub id: String,
    pub version_number: u64,
    pub status: TemplateVersionStatus,
    pub subject: Option<String>,
    pub content_json: JsonObject,
    pub compiled_html: Option<String>,
    pub compiled_text: Option<String>,
    pub published_at: Option<String>,
    pub variables: Vec<TemplateVariable>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateList {
    pub templates: Vec<EmailTemplate>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TemplateListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateTemplateRequest {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTemplateResponse {
    pub template: EmailTemplate,
    pub draft: EmailTemplateVersion,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TemplatePreconditionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateTemplateDraftRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    pub content_json: JsonObject,
    pub variables: Vec<TemplateVariable>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_version_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTemplateDraftResponse {
    pub draft: EmailTemplateVersion,
    pub preview_html: String,
    pub preview_text: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct PreviewTemplateRequest {
    #[serde(default, skip_serializing_if = "JsonObject::is_empty")]
    pub variables: JsonObject,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewTemplateResponse {
    pub subject: String,
    pub html: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateVersionList {
    pub versions: Vec<EmailTemplateVersion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateTemplateAssetRequest {
    pub filename: String,
    pub content_type: String,
}

#[derive(Clone, Deserialize)]
pub struct TemplateAssetPolicy {
    pub upload_url: String,
    pub upload_fields: BTreeMap<String, String>,
    pub asset_url: String,
}

impl fmt::Debug for TemplateAssetPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TemplateAssetPolicy")
            .field("upload_url", &"[REDACTED]")
            .field("upload_fields", &"[REDACTED]")
            .field("asset_url", &self.asset_url)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct WebhookEndpoint {
    pub id: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub enabled: bool,
    pub max_attempts: u64,
    pub consecutive_failures: u64,
    pub disabled_at: Option<String>,
    pub secret_rotated_at: Option<String>,
    pub version: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl fmt::Debug for WebhookEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WebhookEndpoint")
            .field("id", &self.id)
            .field("url", &"[REDACTED]")
            .field("event_types", &self.event_types)
            .field("enabled", &self.enabled)
            .field("max_attempts", &self.max_attempts)
            .field("consecutive_failures", &self.consecutive_failures)
            .field("disabled_at", &self.disabled_at)
            .field("secret_rotated_at", &self.secret_rotated_at)
            .field("version", &self.version)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookList {
    pub webhooks: Vec<WebhookEndpoint>,
}

#[derive(Clone, Serialize)]
pub struct CreateWebhookRequest {
    pub url: String,
    pub event_types: Vec<String>,
}

impl fmt::Debug for CreateWebhookRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateWebhookRequest")
            .field("url", &"[REDACTED]")
            .field("event_types", &self.event_types)
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct CreateWebhookResponse {
    pub endpoint: WebhookEndpoint,
    pub secret: String,
}

impl fmt::Debug for CreateWebhookResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreateWebhookResponse")
            .field("endpoint", &self.endpoint)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateWebhookRequest {
    pub expected_version: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct WebhookDeliveryListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookDeliverySummary {
    pub delivery_id: String,
    pub event_type: String,
    pub status: String,
    pub attempt_count: u64,
    pub created_at: String,
    pub updated_at: String,
    pub next_retry_at: Option<String>,
    pub delivered_at: Option<String>,
    pub last_response_code: Option<u16>,
    pub last_duration_ms: Option<u64>,
    pub is_test: bool,
    pub replay_of_delivery_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookDeliveryPage {
    pub data: Vec<WebhookDeliverySummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookDeliveryAttempt {
    pub attempt: u64,
    pub status: String,
    pub response_code: Option<u16>,
    pub duration_ms: Option<u64>,
    pub attempted_at: String,
    pub next_retry_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPayloadRedacted {
    pub event_type: Option<String>,
    pub message_id: Option<String>,
    pub inbound_message_id: Option<String>,
    pub occurred_at: Option<String>,
    pub test: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookDeliveryDetail {
    #[serde(flatten)]
    pub summary: WebhookDeliverySummary,
    pub payload_redacted: WebhookPayloadRedacted,
    pub attempts: Vec<WebhookDeliveryAttempt>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookTestAccepted {
    pub delivery_id: String,
    pub status: String,
    pub created_at: String,
    pub is_test: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookReplayAccepted {
    pub delivery_id: String,
    pub status: String,
    pub created_at: String,
    pub source_delivery_id: String,
}

#[derive(Clone, Deserialize)]
pub struct RotateWebhookSecretResponse {
    pub endpoint: WebhookEndpoint,
    pub secret: Option<String>,
    pub rotated_at: String,
}

/// Result of an atomic CSV contact import.
#[derive(Debug, Clone, Deserialize)]
pub struct ContactImportResult {
    pub total: u64,
    pub created: u64,
    pub skipped: u64,
    pub duplicates: u64,
}

/// A contact returned in a segment preview. Contact fields can contain PII;
/// callers should avoid logging this value wholesale.
#[derive(Clone, Deserialize)]
pub struct Contact {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub subscribed: bool,
    pub properties: JsonObject,
    pub created_at: String,
    pub updated_at: String,
}

impl fmt::Debug for Contact {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Contact")
            .field("id", &self.id)
            .field("email", &"[REDACTED]")
            .field(
                "first_name",
                &self.first_name.as_ref().map(|_| "[REDACTED]"),
            )
            .field("last_name", &self.last_name.as_ref().map(|_| "[REDACTED]"))
            .field("subscribed", &self.subscribed)
            .field("properties", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct InboundDomainConfiguration {
    pub recipient_domain: String,
    pub status: String,
    pub mx: InboundMXConfiguration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InboundMXConfiguration {
    pub host: String,
    pub priority: u16,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainHealth {
    pub domain_id: String,
    pub domain_name: String,
    pub domain_status: DomainStatus,
    pub score: Option<u8>,
    pub status: String,
    pub calculation_version: String,
    pub evaluated_at: String,
    pub dns_checked_at: Option<String>,
    pub window: DomainHealthWindow,
    pub minimum_sample_size: u64,
    pub sample_size: u64,
    pub checks: DomainHealthChecks,
    pub recommendations: Vec<DomainHealthRecommendation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainHealthWindow {
    pub start: String,
    pub end: String,
    pub days: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainHealthChecks {
    pub spf: DomainHealthDNSCheck,
    pub dkim: DomainHealthDNSCheck,
    pub dmarc: DomainHealthDNSCheck,
    pub delivery_rate: DomainHealthRateCheck,
    pub bounce_rate: DomainHealthRateCheck,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainHealthDNSCheck {
    pub verified: bool,
    pub status: String,
    pub points: u8,
    pub max_points: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainHealthRateCheck {
    pub numerator: u64,
    pub denominator: u64,
    pub rate_basis_points: Option<u16>,
    pub status: String,
    pub points: Option<u8>,
    pub max_points: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainHealthRecommendation {
    pub code: String,
    pub check: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct MessageTimelineParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct MessageTimelineEvent {
    pub id: String,
    pub message_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub occurred_at: String,
    pub recipient: Option<String>,
    pub smtp_code: Option<i64>,
    pub enhanced_code: Option<String>,
    pub diagnostic: Option<String>,
    pub mx_host: Option<String>,
    pub click_url: Option<String>,
}

impl fmt::Debug for MessageTimelineEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessageTimelineEvent")
            .field("id", &self.id)
            .field("message_id", &self.message_id)
            .field("event_type", &self.event_type)
            .field("occurred_at", &self.occurred_at)
            .field("recipient", &self.recipient.as_ref().map(|_| "[REDACTED]"))
            .field("smtp_code", &self.smtp_code)
            .field("enhanced_code", &self.enhanced_code)
            .field(
                "diagnostic",
                &self.diagnostic.as_ref().map(|_| "[REDACTED]"),
            )
            .field("mx_host", &self.mx_host.as_ref().map(|_| "[REDACTED]"))
            .field("click_url", &self.click_url.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct MessageTimelinePage {
    pub data: Vec<MessageTimelineEvent>,
    pub next_cursor: Option<String>,
}

impl fmt::Debug for MessageTimelinePage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MessageTimelinePage")
            .field("data", &"[REDACTED]")
            .field("next_cursor", &self.next_cursor)
            .finish()
    }
}

/// Dynamic segment rules are recursive and intentionally represented as JSON.
/// This preserves all current contract variants without silently discarding a
/// predicate added by the API; the service remains the semantic validator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SegmentDefinition(pub Value);

#[derive(Debug, Clone, Serialize)]
pub struct SegmentPreviewRequest {
    pub definition: SegmentDefinition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u8>,
}

#[derive(Clone, Deserialize)]
pub struct SegmentPreview {
    pub contact_count: u64,
    pub data: Vec<Contact>,
}

impl fmt::Debug for SegmentPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SegmentPreview")
            .field("contact_count", &self.contact_count)
            .field("data", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchSendMessage {
    pub idempotency_key: String,
    pub request: SendRequest,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchSendRequest {
    pub messages: Vec<BatchSendMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchSendError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchSendResultItem {
    pub index: u64,
    pub accepted: Option<Vec<AcceptedMessage>>,
    pub rejected: Option<Vec<RejectedMessage>>,
    pub error: Option<BatchSendError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchSendResult {
    pub results: Vec<BatchSendResultItem>,
}

impl fmt::Debug for RotateWebhookSecretResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RotateWebhookSecretResponse")
            .field("endpoint", &self.endpoint)
            .field("secret", &self.secret.as_ref().map(|_| "[REDACTED]"))
            .field("rotated_at", &self.rotated_at)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Automation {
    pub id: String,
    pub name: String,
    pub status: AutomationStatus,
    pub graph: JsonObject,
    pub current_version_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutomationList {
    pub data: Vec<Automation>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AutomationListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AutomationStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateAutomationRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenameAutomationRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateAutomationDraftRequest {
    pub graph: JsonObject,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutomationRun {
    pub id: String,
    pub automation_id: String,
    pub automation_version_id: String,
    pub contact_id: String,
    pub trigger_event_id: String,
    pub status: AutomationRunStatus,
    pub graph_snapshot: JsonObject,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutomationRunStep {
    pub id: String,
    pub step_key: String,
    pub step_type: String,
    pub status: String,
    pub attempts: u64,
    pub branch: Option<String>,
    pub error: Option<String>,
    pub scheduled_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutomationRunList {
    pub data: Vec<AutomationRun>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutomationRunDetail {
    pub run: AutomationRun,
    pub steps: Vec<AutomationRunStep>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AutomationRunListParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AutomationRunStatus>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsagePeriod {
    pub start: String,
    pub end: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonthlyUsage {
    pub period: UsagePeriod,
    pub used: u64,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub unlimited: bool,
}
