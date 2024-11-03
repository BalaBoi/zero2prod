use std::fmt::{Debug, Display};

use actix_web::{http::header::LOCATION, HttpResponse};

pub fn e500<T>(error: T) -> actix_web::Error
where
    T: Debug + Display + 'static,
{
    actix_web::error::ErrorInternalServerError(error)
}

pub fn see_other(location: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}
