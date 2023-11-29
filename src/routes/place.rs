use std::sync::RwLock;

use actix_web::{get, post, web, HttpRequest, HttpResponse, Error, error};
use chrono::Utc;
use serde_derive::Deserialize;

use crate::database::Database;
use crate::models::appstate::AppState;
use crate::routes::utils::token_to_id;

#[derive(Deserialize)]
struct DrawInfo {
    x: u32,
    y: u32,
    color: u8,
}

#[get("/api/png")]
async fn get_png(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<Database>,
) -> Result<HttpResponse, Error> {
    let mut appstate = appstate.write()
        .map_err(|_| error::ErrorInternalServerError("appstate write error"))?;

    appstate.try_update(&database)
        .map_err(|err| eprintln!("appstate error: {}", err)).ok();

    Ok(HttpResponse::Ok().content_type("image/png").body(appstate.get_png().clone()))
}

#[get("/api/updates")]
async fn get_updates(appstate: web::Data<RwLock<AppState>>) -> Result<HttpResponse, Error> {
    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    Ok(HttpResponse::Ok().json(appstate.get_message_updates()))
}

#[post("/api/draw")]
async fn draw(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<Database>,
    info: web::Json<DrawInfo>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let mut appstate = appstate.write()
        .map_err(|err| error::ErrorInternalServerError(format!("appstate error: {}", err)))?;

    let user_id = token_to_id(req, appstate.jwt_secret().as_bytes())
        .map_err(|_| error::ErrorBadRequest("Failed to decode token"))?;

    let user = appstate.get_user(user_id)
        .ok_or(error::ErrorBadRequest("invalid user"))?;

    let time = Utc::now().timestamp();

    if info.x >= appstate.get_size().0 as u32 || info.y >= appstate.get_size().1 as u32 {
        return Err(error::ErrorBadRequest("invalid coordinates"));
    }

    if user.cooldown - time > 0 {
        return Err(error::ErrorBadRequest(format!("cooldown not over : {}s", user.cooldown - time)));
    }

    if !user.verified {
        return Err(error::ErrorBadRequest("unverified"));
    }

    appstate.draw(info.x as usize, info.y as usize, user_id, info.color)
        .map_err(|err| error::ErrorInternalServerError(format!("appstate error: {}", err)))?;

    appstate.try_update(&database)
        .map_err(|err| eprintln!("appstate error: {}", err)).ok();

    Ok(HttpResponse::Ok().json(appstate.cooldown()))
}

#[get("/api/size")]
async fn get_size(appstate: web::Data<RwLock<AppState>>) -> Result<HttpResponse, Error> {
    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    Ok(HttpResponse::Ok().json(appstate.get_size()))
}

#[get("/api/username/{x}/{y}")]
async fn get_username(
    appstate: web::Data<RwLock<AppState>>,
    path: web::Path<(u32, u32)>,
) -> Result<HttpResponse, Error> {
    let (x, y) = path.into_inner();

    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    if x >= appstate.get_size().0 as u32 || y >= appstate.get_size().1 as u32 {
        return Err(error::ErrorBadRequest("invalid coordinates"));
    }

    Ok(HttpResponse::Ok().body(appstate.get_username_from_pixel(x as usize, y as usize)))
}

#[get("/api/users/count")]
async fn get_users_count(appstate: web::Data<RwLock<AppState>>) -> Result<HttpResponse, Error> {
    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    Ok(HttpResponse::Ok().json(appstate.user_length()))
}

#[get("/api/users/connected")]
async fn get_users_connected(appstate: web::Data<RwLock<AppState>>) -> Result<HttpResponse, Error> {
    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    Ok(HttpResponse::Ok().json(appstate.get_users_connected()))
}

#[get("/api/leaderboard")]
async fn get_leaderboard(appstate: web::Data<RwLock<AppState>>) -> Result<HttpResponse, Error> {
    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    Ok(HttpResponse::Ok().json(appstate.get_leaderboard()))
}