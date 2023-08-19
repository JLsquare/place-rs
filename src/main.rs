mod database;
mod appstate;
mod websocket;
mod routes;
mod utils;

use std::env;
use std::sync::RwLock;
use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use actix_files::Files;
use dotenv::dotenv;
use crate::appstate::AppState;
use crate::database::Database;
use crate::routes::{draw, get_cooldown, get_grid, get_leaderboard, get_size, login, signup, verify};
use crate::websocket::ws_index;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let width: usize = env::var("WIDTH")
        .expect("WIDTH must be set")
        .parse()
        .expect("WIDTH should be a valid usize");

    let height: usize = env::var("HEIGHT")
        .expect("HEIGHT must be set")
        .parse()
        .expect("HEIGHT should be a valid usize");

    let bind_address = env::var("BIND_ADDRESS")
        .expect("BIND_ADDRESS must be set");

    let port: u16 = env::var("PORT")
        .expect("PORT must be set")
        .parse()
        .expect("PORT should be a valid u16");


    let data = web::Data::new(RwLock::new(AppState::new(width, height)));
    let database = Database::new()
        .expect("Error connecting to database");
    database.create_tables()
        .expect("Error creating tables");

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .app_data(data.clone())
            .service(get_grid)
            .service(draw)
            .service(login)
            .service(signup)
            .service(get_leaderboard)
            .service(verify)
            .service(ws_index)
            .service(get_cooldown)
            .service(get_size)
            .service(Files::new("/", "public").index_file("index.html"))
    })
        .bind((bind_address, port))?
        .run()
        .await
}