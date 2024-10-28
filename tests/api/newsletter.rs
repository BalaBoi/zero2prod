use crate::helpers::{get_confirmation_links, spawn_app, ConfirmationLinks, TestApp};
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = test_app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;

    Mock::given(path("/v3/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body as plain text",
            "html": "<p>Newsletter body as HTML</p>",
        }
    });
    let response = test_app.post_newsletters(&newsletter_request_body).await;

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let test_app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "random email newsletter text",
                    "html": "<p>Newsletter body as html</p>",
                }
            }),
            "missing the title",
        ),
        (
            serde_json::json!({
                "title": "Newsletter subject"
            }),
            "missing the content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = test_app.post_newsletters(&invalid_body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with a 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let test_app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(format!("{}/newsletters", test_app.address))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "html": "<p>Newsletter body as html</p>",
                "text": "Newsletter body as plain text",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let _mock_guard = Mock::given(path("/v3/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;
    test_app
        .post_subscription(body)
        .await
        .error_for_status()
        .unwrap();
    let email_request = test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();
    get_confirmation_links(&email_request, test_app.port)
}

async fn create_confirmed_subscriber(test_app: &TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(test_app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
