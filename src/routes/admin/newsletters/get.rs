use actix_web::{http::header::ContentType, HttpResponse};
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use uuid::Uuid;

pub async fn get_newsletters_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut html_msg = String::new();
    for m in flash_messages.iter() {
        writeln!(html_msg, "<p><i>{}</i></i>", m.content()).unwrap();
    }
    let idempotency_key = Uuid::new_v4();
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Send a Newsletter</title>
</head>
<body>
    {html_msg}
    <form action="/admin/newsletters" method="post">
        <label>Title
            <input
            type="text"
            placeholder="Enter the Title"
            name="title"
            >
        </label>
        <br>
        <label>HTML
            <input
            type="text"
            placeholder="Enter content as HTML"
            name="html"
            >
        </label>
        <br>
        <label>TEXT
            <input
            type="password"
            placeholder="Enter content as Text"
            name="text"
            >
        </label>
        <br>
        <input hidden type="text" name="idempotency_key" value="{idempotency_key}">
        <button type="submit">Send Newsletter</button>
    </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>"#,
        ))
}
