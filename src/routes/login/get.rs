use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut fm_html = String::new();
    
    for msg in flash_messages.iter() {
        writeln!(fm_html, "<p><i>{}</i></p>", msg.content()).unwrap();
    }
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">

<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>

<body>
    {fm_html}
    <form action="\login" method="post">
        <label>
            Username
            <input type="text" placeholder="Enter Username" name="username">
        </label>

        <label>
            Password
            <input type="password" placeholder="Enter Password" name="password">
        </label>

        <button type="submit">Login</button>
    </form>
</body>

</html>"#
        ))
}
