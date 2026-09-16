use std::{
    collections::{BTreeMap, VecDeque},
    io::{Read, Write},
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use viapost::{
    ClientBuilder, CreateWebhookRequest, CreateWebhookResponse, EmailStream, Error, Message,
    MessageDetail, MessageListParams, MetricsParams, SendRequest, TemplateAssetPolicy,
    TemplatePreconditionRequest, UpdateWebhookRequest, ViaPost, WebhookDeliveryListParams,
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
            event_types: vec!["delivered".to_owned()],
            enabled: true,
            max_attempts: 3,
            consecutive_failures: 0,
            disabled_at: None,
            secret_rotated_at: None,
            version: 1,
            created_at: "2026-09-11T00:00:00Z".to_owned(),
            updated_at: "2026-09-11T00:00:00Z".to_owned(),
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

    let detail = MessageDetail {
        message: Message {
            id: "message-id".to_owned(),
            status: "delivered".to_owned(),
            stream: EmailStream::Transactional,
            from_address: "hello@example.com".to_owned(),
            to_address: "person@example.com".to_owned(),
            subject: Some("Hello".to_owned()),
            recipient_domain: "example.com".to_owned(),
            api_key_id: None,
            created_at: "2026-09-16T00:00:00Z".to_owned(),
            queued_at: None,
            sent_at: None,
            delivered_at: None,
            failed_at: None,
            first_opened_at: None,
            first_clicked_at: None,
            last_error: None,
        },
        body_html: Some("<p>private body</p>".to_owned()),
        body_plain: Some("private body".to_owned()),
        content_status: Some("available".to_owned()),
        raw_message_api_path: Some("/v1/messages/message-id/raw".to_owned()),
        content_variant: Some("submitted".to_owned()),
    };
    let detail_debug = format!("{detail:?}");
    assert!(!detail_debug.contains("private body"));

    let endpoint_debug = format!("{:?}", webhook.endpoint);
    assert!(!endpoint_debug.contains("example.test/webhook"));

    let create_request = CreateWebhookRequest {
        url: "https://example.test/webhook?token=never-log-this".to_owned(),
        event_types: vec!["delivered".to_owned()],
    };
    let request_debug = format!("{create_request:?}");
    assert!(!request_debug.contains("example.test/webhook"));
    assert!(!request_debug.contains("never-log-this"));
    assert!(request_debug.contains("[REDACTED]"));
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

    let insecure_webhook = CreateWebhookRequest {
        url: "http://example.com/hook".to_owned(),
        event_types: vec!["delivered".to_owned()],
    };
    assert!(matches!(
        client.webhooks().create(&insecure_webhook).await,
        Err(Error::Validation(_))
    ));

    for private_url in [
        "https://localhost/hook",
        "https://localhost./hook",
        "https://api.localhost/hook",
        "https://127.0.0.1/hook",
        "https://10.0.0.1/hook",
        "https://169.254.1.1/hook",
        "https://[::1]/hook",
        "https://[fc00::1]/hook",
    ] {
        let webhook = CreateWebhookRequest {
            url: private_url.to_owned(),
            event_types: vec!["delivered".to_owned()],
        };
        assert!(matches!(
            client.webhooks().create(&webhook).await,
            Err(Error::Validation(_))
        ));
    }

    let incomplete_precondition = TemplatePreconditionRequest {
        expected_version_id: Some("11111111-1111-1111-1111-111111111111".to_owned()),
        expected_updated_at: None,
    };
    assert!(matches!(
        client
            .templates()
            .publish(
                "11111111-1111-1111-1111-111111111111",
                Some(&incomplete_precondition),
            )
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

#[tokio::test]
async fn message_retrieve_returns_content_aware_detail() {
    let body = r#"{"id":"11111111-1111-1111-1111-111111111111","status":"delivered","stream":"transactional","from_address":"hello@example.com","to_address":"person@example.com","subject":"Hello","recipient_domain":"example.com","api_key_id":null,"created_at":"2026-09-16T00:00:00Z","queued_at":null,"sent_at":null,"delivered_at":"2026-09-16T00:00:01Z","failed_at":null,"first_opened_at":null,"first_clicked_at":null,"last_error":null,"body_html":"<p>Hello</p>","body_plain":"Hello","content_status":"available","raw_message_api_path":"/v1/messages/11111111-1111-1111-1111-111111111111/raw","content_variant":"submitted"}"#;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let leaked: &'static str = Box::leak(response.into_boxed_str());
    let (base_url, requests) = server(vec![leaked]);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .build()
        .unwrap();

    let detail = client
        .messages()
        .retrieve("11111111-1111-1111-1111-111111111111")
        .await
        .unwrap();

    assert_eq!(detail.message.status, "delivered");
    assert_eq!(detail.body_plain.as_deref(), Some("Hello"));
    assert_eq!(detail.content_status.as_deref(), Some("available"));
    assert!(detail
        .raw_message_api_path
        .as_deref()
        .is_some_and(|path| path.ends_with("/raw")));
    let captured = requests.lock().unwrap();
    assert!(captured[0].starts_with("GET /v1/messages/"));
    assert!(captured[0].contains("11111111%2D1111%2D1111%2D1111%2D111111111111"));
}

#[tokio::test]
async fn message_raw_download_returns_the_original_rfc5322_bytes() {
    let body = "From: hello@example.com\r\nTo: person@example.com\r\n\r\nHello\r\n";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: message/rfc822\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let leaked: &'static str = Box::leak(response.into_boxed_str());
    let (base_url, requests) = server(vec![leaked]);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .build()
        .unwrap();

    let raw = client
        .messages()
        .raw("11111111-1111-1111-1111-111111111111")
        .await
        .unwrap();

    assert_eq!(raw, body.as_bytes());
    let captured = requests.lock().unwrap();
    assert!(captured[0].starts_with("GET /v1/messages/"));
    assert!(captured[0].contains("/raw "));
    assert!(captured[0]
        .to_ascii_lowercase()
        .contains("accept: message/rfc822"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn webhook_management_and_delivery_operations_follow_the_contract() {
    let endpoint = r#"{"id":"11111111-1111-1111-1111-111111111111","url":"https://example.com/hook","event_types":["delivered"],"enabled":true,"max_attempts":8,"consecutive_failures":0,"disabled_at":null,"secret_rotated_at":null,"version":2,"created_at":"2026-09-16T00:00:00Z","updated_at":"2026-09-16T00:01:00Z"}"#;
    let delivery_page = r#"{"data":[],"next_cursor":"next-page"}"#;
    let delivery = r#"{"delivery_id":"22222222-2222-2222-2222-222222222222","event_type":"delivered","status":"delivered","attempt_count":1,"created_at":"2026-09-16T00:00:00Z","updated_at":"2026-09-16T00:00:01Z","next_retry_at":null,"delivered_at":"2026-09-16T00:00:01Z","last_response_code":204,"last_duration_ms":15,"is_test":false,"replay_of_delivery_id":null,"payload_redacted":{"event_type":"delivered","message_id":"33333333-3333-3333-3333-333333333333","occurred_at":"2026-09-16T00:00:00Z"},"attempts":[{"attempt":1,"status":"delivered","response_code":204,"duration_ms":15,"attempted_at":"2026-09-16T00:00:01Z","next_retry_at":null}]}"#;
    let test_accepted = r#"{"delivery_id":"44444444-4444-4444-4444-444444444444","status":"queued","created_at":"2026-09-16T00:02:00Z","is_test":true}"#;
    let replay_accepted = r#"{"delivery_id":"55555555-5555-5555-5555-555555555555","status":"queued","created_at":"2026-09-16T00:03:00Z","source_delivery_id":"22222222-2222-2222-2222-222222222222"}"#;
    let rotation = format!(
        "{{\"endpoint\":{endpoint},\"secret\":\"{}\",\"rotated_at\":\"2026-09-16T00:04:00Z\"}}",
        "s".repeat(43)
    );
    let responses: Vec<&'static str> = [
        endpoint.to_owned(),
        delivery_page.to_owned(),
        delivery.to_owned(),
        test_accepted.to_owned(),
        replay_accepted.to_owned(),
        rotation,
    ]
    .into_iter()
    .map(|body| {
        Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str(),
        ) as &'static str
    })
    .collect();
    let (base_url, requests) = server(responses);
    let client = ViaPost::builder("vp_test")
        .base_url(base_url)
        .unwrap()
        .build()
        .unwrap();
    let webhook_id = "11111111-1111-1111-1111-111111111111";
    let delivery_id = "22222222-2222-2222-2222-222222222222";

    let updated = client
        .webhooks()
        .update(
            webhook_id,
            &UpdateWebhookRequest {
                expected_version: 1,
                enabled: None,
                event_types: None,
                max_attempts: Some(8),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.version, 2);

    let page = client
        .webhooks()
        .deliveries(
            webhook_id,
            WebhookDeliveryListParams {
                cursor: Some("opaque cursor".to_owned()),
                limit: Some(25),
                status: Some("delivered".to_owned()),
                event_type: Some("delivered".to_owned()),
            },
        )
        .await
        .unwrap();
    assert_eq!(page.next_cursor.as_deref(), Some("next-page"));

    let detail = client
        .webhooks()
        .delivery(webhook_id, delivery_id)
        .await
        .unwrap();
    assert_eq!(detail.summary.last_response_code, Some(204));
    assert_eq!(detail.attempts.len(), 1);

    let accepted = client
        .webhooks()
        .test(webhook_id, "test-operation-1")
        .await
        .unwrap();
    assert!(accepted.is_test);

    let replayed = client
        .webhooks()
        .replay(webhook_id, delivery_id, "replay-operation-1")
        .await
        .unwrap();
    assert_eq!(replayed.source_delivery_id, delivery_id);

    let rotated = client
        .webhooks()
        .rotate_secret(webhook_id, "rotate-operation-1")
        .await
        .unwrap();
    assert_eq!(rotated.secret.as_deref().map(str::len), Some(43));
    assert!(!format!("{rotated:?}").contains(&"s".repeat(43)));

    let captured = requests.lock().unwrap();
    assert!(captured[0].starts_with("PATCH /v1/webhooks/"));
    assert!(captured[0].contains("11111111%2D1111%2D1111%2D1111%2D111111111111"));
    assert!(captured[0].contains("\"expected_version\":1"));
    assert!(captured[1].contains("cursor=opaque+cursor"));
    assert!(captured[1].contains("limit=25"));
    assert!(captured[2].starts_with("GET /v1/webhooks/"));
    assert!(captured[2].contains("/deliveries/22222222%2D2222%2D2222%2D2222%2D222222222222"));
    assert!(captured[3].starts_with("POST /v1/webhooks/"));
    assert!(captured[3].contains("/test "));
    assert!(captured[4].starts_with("POST /v1/webhooks/"));
    assert!(captured[4].contains("/deliveries/22222222%2D2222%2D2222%2D2222%2D222222222222/replay"));
    assert!(captured[5].starts_with("POST /v1/webhooks/"));
    assert!(captured[5].contains("/secret/rotate "));
    for (request, key) in [
        (&captured[3], "test-operation-1"),
        (&captured[4], "replay-operation-1"),
        (&captured[5], "rotate-operation-1"),
    ] {
        assert!(request
            .to_ascii_lowercase()
            .contains(&format!("idempotency-key: {key}")));
        assert!(request.ends_with("{}"));
    }
}

#[tokio::test]
async fn webhook_update_rejects_noop_and_invalid_constraints_before_network() {
    let client = ViaPost::new("vp_test").unwrap();
    let noop = UpdateWebhookRequest {
        expected_version: 1,
        enabled: None,
        event_types: None,
        max_attempts: None,
    };
    assert!(matches!(
        client.webhooks().update("webhook-id", &noop).await,
        Err(Error::Validation(_))
    ));

    let invalid = UpdateWebhookRequest {
        expected_version: 0,
        enabled: Some(true),
        event_types: None,
        max_attempts: None,
    };
    assert!(matches!(
        client.webhooks().update("webhook-id", &invalid).await,
        Err(Error::Validation(_))
    ));
}
