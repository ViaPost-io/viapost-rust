use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use viapost::{
    ClientBuilder, CreateWebhookRequest, CreateWebhookResponse, Error, MessageListParams,
    MetricsParams, SendRequest, TemplateAssetPolicy, TemplatePreconditionRequest, ViaPost,
    WebhookEndpoint,
};

fn server(responses: Vec<&'static str>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    thread::spawn(move || {
        while let Some(response) = responses.lock().unwrap().pop_front() {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = vec![0_u8; 32 * 1024];
            let read = stream.read(&mut buffer).unwrap();
            captured
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&buffer[..read]).into_owned());
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    (format!("http://{address}"), requests)
}

#[tokio::test]
async fn send_forwards_idempotency_key_and_never_retries_mutations() {
    let (base_url, requests) = server(vec![
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: 23\r\nConnection: close\r\n\r\n{\"error\":\"unavailable\"}",
    ]);
    let client = ViaPost::builder("vp_test_secret")
        .base_url(base_url)
        .unwrap()
        .max_retries(2)
        .build()
        .unwrap();
    let request = SendRequest::new("hello@example.com", ["person@example.com"]);
    let error = client
        .send()
        .create(&request, Some("order-123"))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Api { status: 503, .. }));
    let captured = requests.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert!(captured[0]
        .to_ascii_lowercase()
        .contains("idempotency-key: order-123"));
    assert!(captured[0]
        .to_ascii_lowercase()
        .contains("authorization: bearer vp_test_secret"));
}

#[tokio::test]
async fn get_retries_retryable_status_then_succeeds() {
    let (base_url, requests) = server(vec![
        "HTTP/1.1 503 Service Unavailable\r\nRetry-After: 0\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"messages\":[]}",
    ]);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .max_retries(1)
        .build()
        .unwrap();

    let result = client
        .messages()
        .list(MessageListParams::default())
        .await
        .unwrap();
    assert!(result.messages.is_empty());
    assert_eq!(requests.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn rejects_oversized_decoded_response() {
    let body = "x".repeat(64);
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
    let leaked: &'static str = Box::leak(response.into_boxed_str());
    let (base_url, _) = server(vec![leaked]);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .max_response_bytes(32)
        .unwrap()
        .build()
        .unwrap();

    assert!(matches!(
        client.usage().retrieve().await.unwrap_err(),
        Error::ResponseTooLarge { limit: 32 }
    ));
}

#[test]
fn security_defaults_validate_transport_and_redact_api_key() {
    assert!(ViaPost::builder("secret")
        .base_url("http://example.com")
        .is_err());
    assert!(ViaPost::builder("secret")
        .base_url("http://127.0.0.1:8080")
        .is_ok());
    assert!(ViaPost::builder("secret")
        .base_url("https://api.viapost.io")
        .is_ok());
    assert!(ClientBuilder::new("line\nbreak").build().is_err());
    let debug = format!("{:?}", ViaPost::new("vp_live_super_secret").unwrap());
    assert!(!debug.contains("vp_live_super_secret"));
    let builder_debug = format!("{:?}", ViaPost::builder("vp_live_super_secret"));
    assert!(!builder_debug.contains("vp_live_super_secret"));
}

#[test]
fn one_time_credentials_are_redacted_from_debug_output() {
    let webhook_secret = "whsec_never_log_this";
    let webhook = CreateWebhookResponse {
        endpoint: WebhookEndpoint {
            id: "webhook-id".to_owned(),
            url: "https://example.test/webhook".to_owned(),
            event_types: vec!["message.delivered".to_owned()],
            enabled: true,
            max_attempts: 3,
            created_at: "2026-09-11T00:00:00Z".to_owned(),
        },
        secret: webhook_secret.to_owned(),
    };
    let mut upload_fields = BTreeMap::new();
    upload_fields.insert("policy".to_owned(), "signed-policy".to_owned());
    let asset = TemplateAssetPolicy {
        upload_url: "https://storage.test/upload?signature=signed-url".to_owned(),
        upload_fields,
        asset_url: "https://cdn.test/public.png".to_owned(),
    };

    let webhook_debug = format!("{webhook:?}");
    let asset_debug = format!("{asset:?}");
    assert!(!webhook_debug.contains(webhook_secret));
    assert!(webhook_debug.contains("[REDACTED]"));
    assert!(!asset_debug.contains("signed-policy"));
    assert!(!asset_debug.contains("signed-url"));
}

#[tokio::test]
async fn api_errors_do_not_echo_the_api_key_or_raw_body_in_debug() {
    let secret = "vp_live_never_log_this";
    let body = format!(
        "{{\"error\":{{\"code\":\"bad_request\",\"message\":\"bad {secret}\"}},\"private\":\"{secret}\"}}"
    );
    let response = format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let leaked: &'static str = Box::leak(response.into_boxed_str());
    let (base_url, _) = server(vec![leaked]);
    let client = ViaPost::builder(secret)
        .base_url(base_url)
        .unwrap()
        .build()
        .unwrap();

    let error = client
        .messages()
        .list(MessageListParams::default())
        .await
        .unwrap_err();
    assert!(!format!("{error}").contains(secret));
    assert!(!format!("{error:?}").contains(secret));
    assert!(!format!("{error:?}").contains("private"));
}

#[tokio::test]
async fn message_metrics_forwards_optional_domain_filter() {
    let (base_url, requests) = server(vec![
        "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    ]);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .build()
        .unwrap();

    let result = client
        .messages()
        .metrics(MetricsParams {
            days: Some(7),
            domain_id: Some("11111111-1111-1111-1111-111111111111".to_owned()),
        })
        .await;

    assert!(matches!(result, Err(Error::Api { status: 400, .. })));
    let captured = requests.lock().unwrap();
    assert!(captured[0].contains("days=7"));
    assert!(captured[0].contains("domain_id=11111111-1111-1111-1111-111111111111"));
}

#[tokio::test]
async fn timeout_covers_retries_and_backoff_as_one_operation() {
    let (base_url, requests) = server(vec![
        "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
    ]);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .timeout(Duration::from_millis(25))
        .unwrap()
        .max_retries(1)
        .build()
        .unwrap();

    let error = client
        .messages()
        .list(MessageListParams::default())
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Timeout { .. }));
    assert_eq!(requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn validates_contract_constraints_before_network() {
    let request = SendRequest::new("from@example.com", std::iter::empty::<String>());
    let client = ViaPost::new("vp_test").unwrap();
    assert!(matches!(
        client.send().create(&request, None).await,
        Err(Error::Validation(_))
    ));

    let request = SendRequest::new("from@example.com", ["to@example.com"]);
    assert!(matches!(
        client.send().create(&request, Some("bad key")).await,
        Err(Error::Validation(_))
    ));

    let mut request = SendRequest::new("from@example.com", ["to@example.com"]);
    request.cc = vec!["cc@example.com".to_owned(); 51];
    assert!(matches!(
        client.send().create(&request, None).await,
        Err(Error::Validation(_))
    ));

    let webhook = CreateWebhookRequest {
        url: "file:///tmp/hook".to_owned(),
        event_types: vec!["delivered".to_owned()],
    };
    assert!(matches!(
        client.webhooks().create(&webhook).await,
        Err(Error::Validation(_))
    ));

    let incomplete_precondition = TemplatePreconditionRequest {
        expected_version_id: Some("11111111-1111-1111-1111-111111111111".to_owned()),
        expected_updated_at: None,
    };
    assert!(matches!(
        client
            .templates()
            .publish("11111111-1111-1111-1111-111111111111", Some(&incomplete_precondition))
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .templates()
            .revert(
                "11111111-1111-1111-1111-111111111111",
                "22222222-2222-2222-2222-222222222222",
                Some(&incomplete_precondition),
            )
            .await,
        Err(Error::Validation(_))
    ));
}
