use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_admin_dashboard() {
    let test_app = spawn_app().await;

    let response = test_app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login");
}
