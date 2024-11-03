use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure_but_not_there_on_refresh() {
    let test_app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "randomdfgb",
        "password": "kegmj;eotgh",
    });

    let response = test_app.post_login(&login_body).await;

    assert_is_redirect_to(&response, "/login");

    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = test_app.get_login_html().await;
    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let test_app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &test_app.test_user.password,
    });

    let response = test_app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_body = test_app.get_admin_dashboard_html().await;
    assert!(html_body.contains(&format!("Welcome {}", test_app.test_user.username)));
}
