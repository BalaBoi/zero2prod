use std::time::Duration;

use crate::helpers::{
    assert_is_redirect_to, get_confirmation_links, spawn_app, ConfirmationLinks, TestApp,
};
use fake::{
    faker::{internet::en::SafeEmail, name::en::Name},
    Fake,
};
use uuid::Uuid;
use wiremock::{
    matchers::{any, method, path},
    Mock, ResponseTemplate,
};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let test_app = spawn_app().await;
    create_unconfirmed_subscriber(&test_app).await;
    test_app.test_user_login().await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0) //no email should be sent to an unconfirmed subscriber
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });
    let response = test_app
        .post_publish_newsletters(&newsletter_request_body)
        .await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted - emails will go out shortly"));
    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.test_user_login().await;

    Mock::given(path("/v3/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as HTML</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });
    let response = test_app
        .post_publish_newsletters(&newsletter_request_body)
        .await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted - emails will go out shortly"));
    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let test_app = spawn_app().await;

    test_app.test_user_login().await;

    let test_cases = vec![
        (
            serde_json::json!({
                "text": "random email newsletter text",
                "html": "<p>Newsletter body as html</p>",
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
        let response = test_app.post_publish_newsletters(&invalid_body).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with a 400 Bad Request when the payload was {}",
            error_message
        );
    }
    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_newsletter_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_publish_newsletter().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let test_app = spawn_app().await;

    let username = Uuid::new_v4();
    let password = Uuid::new_v4();

    let login_body = serde_json::json!({
        "username": &username,
        "password": &password,
    });
    let response = test_app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/login");

    let body = serde_json::json!({
        "title": "Newsletter title",
        "text": "newsletter body as text",
        "html": "<p>Newsletter body as html</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });
    let response = test_app.post_publish_newsletters(&body).await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let test_app = spawn_app().await;

    let password = Uuid::new_v4();
    assert_ne!(test_app.test_user.password, password.to_string());

    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &password,
    });
    let response = test_app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/login");
    let body = serde_json::json!({
        "title": "Newsletter title",
        "text": "newsletter body as text",
        "html": "<p>Newsletter body as html</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });
    let response = test_app.post_publish_newsletters(&body).await;
    assert_is_redirect_to(&response, "/login");
}

async fn create_unconfirmed_subscriber(test_app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email
    }))
    .unwrap();
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

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.test_user_login().await;

    Mock::given(path("/v3/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1) //Only send a single api request to email server as requests are retries
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as html</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });

    let response = test_app
        .post_publish_newsletters(&newsletter_request_body)
        .await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted - emails will go out shortly"));

    let response = test_app
        .post_publish_newsletters(&newsletter_request_body)
        .await;
    assert_is_redirect_to(&response, "/admin/newsletters");

    let html_page = test_app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter issue has been accepted - emails will go out shortly"));
    test_app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let test_app = spawn_app().await;
    create_confirmed_subscriber(&test_app).await;
    test_app.test_user_login().await;

    Mock::given(path("v3/mail/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text": "Newsletter body as plain text",
        "html": "<p>Newsletter body as html</p>",
        "idempotency_key": Uuid::new_v4().to_string()
    });
    let response1 = test_app.post_publish_newsletters(&newsletter_request_body);
    let response2 = test_app.post_publish_newsletters(&newsletter_request_body);

    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
    test_app.dispatch_all_pending_emails().await;
}

// fn when_sending_an_email() -> MockBuilder {
//     Mock::given(path("/v3/mail/send")).and(method("POST"))
// }

// #[tokio::test]
// async fn transient_errors_do_not_cause_duplicate_deliveries_on_retries() {
//     let test_app = spawn_app().await;
//     let newsletter_request_body = serde_json::json!({
//         "title": "Newsletter title",
//         "text": "Newsletter body as plain text",
//         "html": "<p>Newsletter body as html</p>",
//         "idempotency_key": Uuid::new_v4().to_string()
//     });

//     create_confirmed_subscriber(&test_app).await;
//     create_confirmed_subscriber(&test_app).await;
//     test_app.test_user_login().await;

//     when_sending_an_email()
//         .respond_with(ResponseTemplate::new(200))
//         .up_to_n_times(1)
//         .expect(1)
//         .mount(&test_app.email_server)
//         .await;

//     when_sending_an_email()
//         .respond_with(ResponseTemplate::new(500))
//         .up_to_n_times(1)
//         .expect(1)
//         .mount(&test_app.email_server)
//         .await;

//     let response = test_app
//         .post_publish_newsletters(&newsletter_request_body)
//         .await;
//     assert_eq!(response.status().as_u16(), 500);

//     when_sending_an_email()
//         .respond_with(ResponseTemplate::new(200))
//         .expect(1)
//         .named("Delivery retry")
//         .mount(&test_app.email_server)
//         .await;

//     let response = test_app
//         .post_publish_newsletters(&newsletter_request_body)
//         .await;
//     assert_eq!(response.status().as_u16(), 303);
// }
