mod database;
mod models;
mod routes;
mod websocket;

use crate::database::Database;
use crate::models::appstate::AppState;
use crate::routes::place::{
    draw, get_leaderboard, get_png, get_size, get_updates, get_username, get_users_connected,
    get_users_count,
};
use crate::routes::user::{edit_profile, get_profile, login, signup, verify};
use crate::websocket::ws_index;
use actix_cors::Cors;
use actix_files::Files;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use std::sync::RwLock;
use std::{env, io};

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenv().ok();

    let width: usize = env::var("WIDTH")
        .expect("WIDTH must be set")
        .parse()
        .expect("WIDTH should be a valid usize");

    let height: usize = env::var("HEIGHT")
        .expect("HEIGHT must be set")
        .parse()
        .expect("HEIGHT should be a valid usize");

    let bind_address = env::var("BIND_ADDRESS").expect("BIND_ADDRESS must be set");

    let port: u16 = env::var("PORT")
        .expect("PORT must be set")
        .parse()
        .expect("PORT should be a valid u16");

    let per_second: u64 = env::var("RATE_LIMIT_SEC")
        .expect("PER_SECOND must be set")
        .parse()
        .expect("PER_SECOND should be a valid u64");

    let burst_size: u32 = env::var("RATE_LIMIT_SIZE")
        .expect("BURST_SIZE must be set")
        .parse()
        .expect("BURST_SIZE should be a valid u32");

    let database = Database::new().expect("Error connecting to database");

    database.create_tables().expect("Error creating tables");

    let mut appstate = AppState::new(width, height, &database).expect("Error creating appstate");

    let mut database = web::Data::new(database);

    appstate
        .try_update(&mut database)
        .expect("Error updating appstate");

    let appstate = web::Data::new(RwLock::new(appstate));

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(per_second)
        .burst_size(burst_size)
        .finish()
        .expect("Error creating governor config");

    HttpServer::new(move || {
        let app = App::new()
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .wrap(Governor::new(&governor_conf))
            .app_data(appstate.clone())
            .app_data(database.clone())
            .service(get_png)
            .service(get_updates)
            .service(draw)
            .service(login)
            .service(signup)
            .service(get_leaderboard)
            .service(verify)
            .service(ws_index)
            .service(get_size)
            .service(get_profile)
            .service(edit_profile)
            .service(get_users_count)
            .service(get_users_connected)
            .service(get_username)
            .service(Files::new("/", "/var/www/html/").index_file("index.html"));

        app
    })
    .bind((bind_address, port))?
    .run()
    .await
}
