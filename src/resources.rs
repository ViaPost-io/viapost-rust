use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Method,
};
use url::Url;

use crate::models::*;
use crate::{Error, ViaPost};

fn path_parameter(name: &str, value: &str) -> Result<String, Error> {
    if value.is_empty() || matches!(value, "." | "..") {
        return Err(Error::Validation(format!(
            "{name} must be a non-empty safe path value"
        )));
    }
    Ok(percent_encode(value.as_bytes(), NON_ALPHANUMERIC).to_string())
}

fn validate_idempotency_key(value: &str) -> Result<HeaderValue, Error> {
    if value.is_empty()
        || value.len() > 255
        || !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
    {
        return Err(Error::Validation(
            "idempotency_key must contain 1 to 255 visible ASCII characters".into(),
        ));
    }
    HeaderValue::from_str(value)
        .map_err(|_| Error::Validation("idempotency_key is not header-safe".into()))
}

fn validate_send(request: &SendRequest) -> Result<(), Error> {
    if !(1..=50).contains(&request.to.len()) {
        return Err(Error::Validation(
            "to must contain between 1 and 50 recipients".into(),
        ));
    }
    if request.cc.len() > 50 || request.bcc.len() > 50 {
        return Err(Error::Validation(
            "cc and bcc must each contain at most 50 recipients".into(),
        ));
    }
    if request.variables.len() > 100 {
        return Err(Error::Validation(
            "variables must contain at most 100 properties".into(),
        ));
    }
    if request.attachments.len() > 10 {
        return Err(Error::Validation(
            "attachments must contain at most 10 items".into(),
        ));
    }
    Ok(())
}

fn validate_template_precondition(
    expected_version_id: Option<&String>,
    expected_updated_at: Option<&String>,
) -> Result<(), Error> {
    if expected_version_id.is_some() != expected_updated_at.is_some() {
        return Err(Error::Validation(
            "expected_version_id and expected_updated_at must be provided together".into(),
        ));
    }
    Ok(())
}

fn validate_template_draft(request: &UpdateTemplateDraftRequest) -> Result<(), Error> {
    if request.variables.len() > 100 {
        return Err(Error::Validation(
            "variables must contain at most 100 items".into(),
        ));
    }
    validate_template_precondition(
        request.expected_version_id.as_ref(),
        request.expected_updated_at.as_ref(),
    )
}

#[derive(Debug, Clone, Copy)]
pub struct SendResource<'a> {
    client: &'a ViaPost,
}
impl<'a> SendResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }

    pub async fn create(
        &self,
        request: &SendRequest,
        idempotency_key: Option<&str>,
    ) -> Result<SendResult, Error> {
        validate_send(request)?;
        let headers = if let Some(key) = idempotency_key {
            let mut headers = HeaderMap::new();
            headers.insert(
                HeaderName::from_static("idempotency-key"),
                validate_idempotency_key(key)?,
            );
            Some(headers)
        } else {
            None
        };
        self.client
            .request(
                Method::POST,
                "/v1/send",
                None,
                Some(ViaPost::body(request)?),
                headers,
            )
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MessagesResource<'a> {
    client: &'a ViaPost,
}
impl<'a> MessagesResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }

    pub async fn list(&self, params: MessageListParams) -> Result<MessageList, Error> {
        self.client
            .request(
                Method::GET,
                "/v1/messages",
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
    pub async fn retrieve(&self, message_id: &str) -> Result<Message, Error> {
        let id = path_parameter("message_id", message_id)?;
        self.client
            .request(Method::GET, &format!("/v1/messages/{id}"), None, None, None)
            .await
    }
    pub async fn events(&self, message_id: &str) -> Result<MessageEventList, Error> {
        let id = path_parameter("message_id", message_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/messages/{id}/events"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn engagement(&self, params: DaysParams) -> Result<EngagementResponse, Error> {
        self.client
            .request(
                Method::GET,
                "/v1/messages/engagement",
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
    pub async fn metrics(&self, params: MetricsParams) -> Result<MetricsResponse, Error> {
        self.client
            .request(
                Method::GET,
                "/v1/messages/metrics",
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
    pub async fn timeseries(&self, params: DaysParams) -> Result<TimeseriesResponse, Error> {
        self.client
            .request(
                Method::GET,
                "/v1/messages/timeseries",
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DomainsResource<'a> {
    client: &'a ViaPost,
}
impl<'a> DomainsResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }
    pub async fn list(&self) -> Result<DomainList, Error> {
        self.client
            .request(Method::GET, "/v1/domains", None, None, None)
            .await
    }
    pub async fn create(
        &self,
        request: &CreateDomainRequest,
    ) -> Result<CreateDomainResponse, Error> {
        self.client
            .request(
                Method::POST,
                "/v1/domains",
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn retrieve(&self, domain_id: &str) -> Result<Domain, Error> {
        let id = path_parameter("domain_id", domain_id)?;
        self.client
            .request(Method::GET, &format!("/v1/domains/{id}"), None, None, None)
            .await
    }
    pub async fn delete(&self, domain_id: &str) -> Result<(), Error> {
        let id = path_parameter("domain_id", domain_id)?;
        self.client
            .request_empty(Method::DELETE, &format!("/v1/domains/{id}"), None)
            .await
    }
    pub async fn dns(&self, domain_id: &str) -> Result<DnsRecordList, Error> {
        let id = path_parameter("domain_id", domain_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/domains/{id}/dns"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn verify(&self, domain_id: &str) -> Result<Domain, Error> {
        let id = path_parameter("domain_id", domain_id)?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/domains/{id}/verify"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn rotate_dkim(&self, domain_id: &str) -> Result<RotateDkimResponse, Error> {
        let id = path_parameter("domain_id", domain_id)?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/domains/{id}/dkim/rotate"),
                None,
                None,
                None,
            )
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TemplatesResource<'a> {
    client: &'a ViaPost,
}
impl<'a> TemplatesResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }
    pub async fn list(&self, params: TemplateListParams) -> Result<TemplateList, Error> {
        self.client
            .request(
                Method::GET,
                "/v1/templates",
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
    pub async fn create(
        &self,
        request: &CreateTemplateRequest,
    ) -> Result<CreateTemplateResponse, Error> {
        self.client
            .request(
                Method::POST,
                "/v1/templates",
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn retrieve(&self, template_id: &str) -> Result<EmailTemplate, Error> {
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/templates/{id}"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn delete(&self, template_id: &str) -> Result<(), Error> {
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request_empty(Method::DELETE, &format!("/v1/templates/{id}"), None)
            .await
    }
    pub async fn archive(&self, template_id: &str) -> Result<(), Error> {
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request_empty(Method::POST, &format!("/v1/templates/{id}/archive"), None)
            .await
    }
    pub async fn create_asset(
        &self,
        template_id: &str,
        request: &CreateTemplateAssetRequest,
    ) -> Result<TemplateAssetPolicy, Error> {
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/templates/{id}/assets"),
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn update_draft(
        &self,
        template_id: &str,
        request: &UpdateTemplateDraftRequest,
    ) -> Result<UpdateTemplateDraftResponse, Error> {
        validate_template_draft(request)?;
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request(
                Method::PATCH,
                &format!("/v1/templates/{id}/draft"),
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn duplicate(&self, template_id: &str) -> Result<CreateTemplateResponse, Error> {
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/templates/{id}/duplicate"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn preview(
        &self,
        template_id: &str,
        request: &PreviewTemplateRequest,
    ) -> Result<PreviewTemplateResponse, Error> {
        if request.variables.len() > 100 {
            return Err(Error::Validation(
                "variables must contain at most 100 properties".into(),
            ));
        }
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/templates/{id}/preview"),
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn publish(
        &self,
        template_id: &str,
        request: Option<&TemplatePreconditionRequest>,
    ) -> Result<EmailTemplateVersion, Error> {
        let id = path_parameter("template_id", template_id)?;
        if let Some(request) = request {
            validate_template_precondition(
                request.expected_version_id.as_ref(),
                request.expected_updated_at.as_ref(),
            )?;
        }
        let body = request.map(ViaPost::body).transpose()?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/templates/{id}/publish"),
                None,
                body,
                None,
            )
            .await
    }
    pub async fn versions(&self, template_id: &str) -> Result<TemplateVersionList, Error> {
        let id = path_parameter("template_id", template_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/templates/{id}/versions"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn version(
        &self,
        template_id: &str,
        version_id: &str,
    ) -> Result<EmailTemplateVersion, Error> {
        let template_id = path_parameter("template_id", template_id)?;
        let version_id = path_parameter("version_id", version_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/templates/{template_id}/versions/{version_id}"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn revert(
        &self,
        template_id: &str,
        version_id: &str,
        request: Option<&TemplatePreconditionRequest>,
    ) -> Result<EmailTemplateVersion, Error> {
        let template_id = path_parameter("template_id", template_id)?;
        let version_id = path_parameter("version_id", version_id)?;
        if let Some(request) = request {
            validate_template_precondition(
                request.expected_version_id.as_ref(),
                request.expected_updated_at.as_ref(),
            )?;
        }
        let body = request.map(ViaPost::body).transpose()?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/templates/{template_id}/versions/{version_id}/revert"),
                None,
                body,
                None,
            )
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WebhooksResource<'a> {
    client: &'a ViaPost,
}
impl<'a> WebhooksResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }
    pub async fn list(&self) -> Result<WebhookList, Error> {
        self.client
            .request(Method::GET, "/v1/webhooks", None, None, None)
            .await
    }
    pub async fn create(
        &self,
        request: &CreateWebhookRequest,
    ) -> Result<CreateWebhookResponse, Error> {
        let url = Url::parse(&request.url)
            .map_err(|_| Error::Validation("webhook url must be an absolute HTTP(S) URL".into()))?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(Error::Validation(
                "webhook url must be an absolute HTTP(S) URL".into(),
            ));
        }
        if request.event_types.is_empty() {
            return Err(Error::Validation(
                "event_types must contain at least one item".into(),
            ));
        }
        self.client
            .request(
                Method::POST,
                "/v1/webhooks",
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn delete(&self, webhook_id: &str) -> Result<(), Error> {
        let id = path_parameter("webhook_id", webhook_id)?;
        self.client
            .request_empty(Method::DELETE, &format!("/v1/webhooks/{id}"), None)
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AutomationsResource<'a> {
    client: &'a ViaPost,
}
impl<'a> AutomationsResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }
    pub async fn list(&self, params: AutomationListParams) -> Result<AutomationList, Error> {
        self.client
            .request(
                Method::GET,
                "/v1/automations",
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
    pub async fn create(&self, request: &CreateAutomationRequest) -> Result<Automation, Error> {
        if request.name.is_empty() {
            return Err(Error::Validation("name must not be empty".into()));
        }
        self.client
            .request(
                Method::POST,
                "/v1/automations",
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn retrieve(&self, automation_id: &str) -> Result<Automation, Error> {
        let id = path_parameter("automation_id", automation_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/automations/{id}"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn update(
        &self,
        automation_id: &str,
        request: &RenameAutomationRequest,
    ) -> Result<Automation, Error> {
        if request.name.is_empty() {
            return Err(Error::Validation("name must not be empty".into()));
        }
        let id = path_parameter("automation_id", automation_id)?;
        self.client
            .request(
                Method::PATCH,
                &format!("/v1/automations/{id}"),
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn delete(&self, automation_id: &str) -> Result<(), Error> {
        let id = path_parameter("automation_id", automation_id)?;
        self.client
            .request_empty(Method::DELETE, &format!("/v1/automations/{id}"), None)
            .await
    }
    pub async fn activate(&self, automation_id: &str) -> Result<Automation, Error> {
        self.action(automation_id, "activate").await
    }
    pub async fn disable(&self, automation_id: &str) -> Result<Automation, Error> {
        self.action(automation_id, "disable").await
    }
    pub async fn duplicate(&self, automation_id: &str) -> Result<Automation, Error> {
        self.action(automation_id, "duplicate").await
    }
    async fn action(&self, automation_id: &str, action: &str) -> Result<Automation, Error> {
        let id = path_parameter("automation_id", automation_id)?;
        self.client
            .request(
                Method::POST,
                &format!("/v1/automations/{id}/{action}"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn update_draft(
        &self,
        automation_id: &str,
        request: &UpdateAutomationDraftRequest,
    ) -> Result<Automation, Error> {
        let id = path_parameter("automation_id", automation_id)?;
        self.client
            .request(
                Method::PATCH,
                &format!("/v1/automations/{id}/draft"),
                None,
                Some(ViaPost::body(request)?),
                None,
            )
            .await
    }
    pub async fn runs(
        &self,
        automation_id: &str,
        params: AutomationRunListParams,
    ) -> Result<AutomationRunList, Error> {
        if params
            .limit
            .is_some_and(|limit| !(1..=200).contains(&limit))
        {
            return Err(Error::Validation("limit must be between 1 and 200".into()));
        }
        let id = path_parameter("automation_id", automation_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/automations/{id}/runs"),
                Some(ViaPost::body(&params)?),
                None,
                None,
            )
            .await
    }
    pub async fn run(
        &self,
        automation_id: &str,
        run_id: &str,
    ) -> Result<AutomationRunDetail, Error> {
        let automation_id = path_parameter("automation_id", automation_id)?;
        let run_id = path_parameter("run_id", run_id)?;
        self.client
            .request(
                Method::GET,
                &format!("/v1/automations/{automation_id}/runs/{run_id}"),
                None,
                None,
                None,
            )
            .await
    }
    pub async fn cancel_run(&self, automation_id: &str, run_id: &str) -> Result<(), Error> {
        let automation_id = path_parameter("automation_id", automation_id)?;
        let run_id = path_parameter("run_id", run_id)?;
        self.client
            .request_empty(
                Method::POST,
                &format!("/v1/automations/{automation_id}/runs/{run_id}/cancel"),
                None,
            )
            .await
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UsageResource<'a> {
    client: &'a ViaPost,
}
impl<'a> UsageResource<'a> {
    pub(crate) fn new(client: &'a ViaPost) -> Self {
        Self { client }
    }
    pub async fn retrieve(&self) -> Result<MonthlyUsage, Error> {
        self.client
            .request(Method::GET, "/v1/usage", None, None, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_values_are_encoded_and_ambiguous_values_rejected() {
        assert_eq!(
            path_parameter("id", "part/../value ?#").unwrap(),
            "part%2F%2E%2E%2Fvalue%20%3F%23"
        );
        for value in ["", ".", ".."] {
            assert!(path_parameter("id", value).is_err());
        }
    }
}
