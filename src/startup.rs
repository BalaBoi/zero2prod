use std::net::TcpListener;
use actix_web::{HttpServer, App, web};
use actix_web::dev::Server;
use sqlx::PgConnection;
use crate::routes::{health_check, subscribe};

pub fn run(listener: TcpListener, database_connection: PgConnection) -> Result<Server, std::io::Error> {
    //creates an arc so that all the workers of the server can access the database
    let connection = web::Data::new(database_connection);
    let server = HttpServer::new(move|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
