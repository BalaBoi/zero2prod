use serde::Deserialize;
use actix_web::{web, HttpResponse, Responder};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(form: web::Form<FormData>) -> impl Responder {
    HttpResponse::Ok().body(format!("Got email:{} and name:{} as the form data", form.email, form.name))
}
