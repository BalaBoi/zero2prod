use uuid::Uuid;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    let test_app = spawn_app().await;

    let response = test_app.get_change_password().await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_password() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    let body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_password,
        "new_password_check": &new_password,
    });

    let response = test_app.post_change_password(&body).await;

    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let incorrect_password_check = Uuid::new_v4().to_string();

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password,
        }))
        .await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": &test_app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &incorrect_password_check,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let test_app = spawn_app().await;

    let incorrect_current_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();

    test_app
        .post_login(&serde_json::json!({
            "username": &test_app.test_user.username,
            "password": &test_app.test_user.password,
        }))
        .await;

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": &incorrect_current_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>The current password is incorrect</i></p>"
    ));
}

#[tokio::test]
async fn logout_clears_session_state() {
    let test_app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &test_app.test_user.username,
        "password": &test_app.test_user.password,
    });

    let response = test_app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = test_app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", test_app.test_user.username)));

    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have succesfully logged out</i></p>"#));

    let response = test_app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn changing_password_works() {
    let test_app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let response = test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": test_app.test_user.password
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let response = test_app
        .post_change_password(&serde_json::json!({
            "current_password": test_app.test_user.password,
            "new_password": new_password,
            "new_password_check": new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");

    let html_page = test_app.get_change_password_html().await;
    assert!(html_page.contains("Your password has been changed"));

    let response = test_app.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    let html_page = test_app.get_login_html().await;
    assert!(html_page.contains("You have succesfully logged out"));

    let response = test_app
        .post_login(&serde_json::json!({
            "username": test_app.test_user.username,
            "password": new_password
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/dashboard");
}