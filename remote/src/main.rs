mod protocol;
mod server;
mod ws;

use actix::prelude::*;
use actix_web::{App, HttpResponse, HttpServer, Responder, web};

use crate::server::Registry;

const SERVER_PORT: u16 = 8080;
const DEVICE_WS_PATH: &str = "/ws/device";
const UI_WS_PATH: &str = "/ws/ui";
const HEALTH_PATH: &str = "/health";

async fn health() -> impl Responder {
    HttpResponse::Ok().body("ok")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let registry = Registry::new().start();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(registry.clone()))
            .route(DEVICE_WS_PATH, web::get().to(ws::device_ws))
            .route(UI_WS_PATH, web::get().to(ws::ui_ws))
            .route(HEALTH_PATH, web::get().to(health))
    })
    .bind(("0.0.0.0", SERVER_PORT))?
    .run()
    .await
}
