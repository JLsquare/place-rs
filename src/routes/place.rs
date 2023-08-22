use std::sync::RwLock;

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use serde_derive::Deserialize;

use crate::models::appstate::AppState;
use crate::routes::utils::token_to_id;

#[derive(Deserialize)]
struct DrawInfo {
    x: u32,
    y: u32,
    color: u8,
}

#[get("/api/png")]
async fn get_png(appstate: web::Data<RwLock<AppState>>) -> impl Responder {
    let mut appstate = match appstate.write() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    match appstate.try_update() {
        Ok(_) => (),
        Err(err) => eprintln!("appstate error: {}", err),
    }

    HttpResponse::Ok()
        .content_type("image/png")
        .body(appstate.get_png())
}

#[get("/api/updates")]
async fn get_updates(appstate: web::Data<RwLock<AppState>>) -> impl Responder {
    let updates = match appstate.read() {
        Ok(appstate) => appstate.get_message_updates(),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    HttpResponse::Ok().json(updates)
}

#[post("/api/draw")]
async fn draw(
    appstate: web::Data<RwLock<AppState>>,
    info: web::Json<DrawInfo>,
    req: HttpRequest,
) -> impl Responder {
    let mut appstate = match appstate.write() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    let user_id = match token_to_id(req, appstate.jwt_secret().as_bytes()) {
        Ok(username) => username,
        Err(response) => return response,
    };

    let user = match appstate.get_user(user_id) {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("invalid user"),
    };

    let time = Utc::now().timestamp();

    if time - user.cooldown < appstate.cooldown() as i64 {
        return HttpResponse::BadRequest().body("cooldown");
    }

    if !user.verified {
        return HttpResponse::BadRequest().body("unverified");
    }

    appstate.draw(info.x as usize, info.y as usize, user_id, info.color);
    match appstate.try_update() {
        Ok(_) => (),
        Err(err) => eprintln!("appstate error : {}", err),
    }

    HttpResponse::Ok().json(appstate.cooldown())
}

#[get("/api/size")]
async fn get_size(appstate: web::Data<RwLock<AppState>>) -> impl Responder {
    match appstate.read() {
        Ok(appstate) => HttpResponse::Ok().json(appstate.get_size()),
        Err(err) => HttpResponse::InternalServerError().body(format!("appstate error : {}", err)),
    }
}

#[get("/api/username/{x}/{y}")]
async fn get_username(
    appstate: web::Data<RwLock<AppState>>,
    path: web::Path<(u32, u32)>,
) -> impl Responder {
    let (x, y) = path.into_inner();
    match appstate.read() {
        Ok(appstate) => {
            HttpResponse::Ok().body(appstate.get_username_from_pixel(x as usize, y as usize))
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("appstate error : {}", err)),
    }
}

#[get("/api/users/count")]
async fn get_users_count(appstate: web::Data<RwLock<AppState>>) -> impl Responder {
    match appstate.read() {
        Ok(appstate) => HttpResponse::Ok().json(appstate.user_length()),
        Err(err) => HttpResponse::InternalServerError().body(format!("appstate error : {}", err)),
    }
}

#[get("/api/users/connected")]
async fn get_users_connected(appstate: web::Data<RwLock<AppState>>) -> impl Responder {
    match appstate.read() {
        Ok(appstate) => HttpResponse::Ok().json(appstate.get_users_connected()),
        Err(err) => HttpResponse::InternalServerError().body(format!("appstate error : {}", err)),
    }
}

#[get("/api/profile/me")]
async fn get_profile(appstate: web::Data<RwLock<AppState>>, req: HttpRequest) -> impl Responder {
    let appstate = match appstate.read() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    let user_id = match token_to_id(req, appstate.jwt_secret().as_bytes()) {
        Ok(username) => username,
        Err(response) => return response,
    };

    let user = match appstate.get_user(user_id) {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("invalid user"),
    };

    HttpResponse::Ok().json(user)
}

#[get("/api/cooldown")]
async fn get_cooldown(appstate: web::Data<RwLock<AppState>>, req: HttpRequest) -> impl Responder {
    let appstate = match appstate.read() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    let user_id = match token_to_id(req, appstate.jwt_secret().as_bytes()) {
        Ok(username) => username,
        Err(response) => return response,
    };

    let user = match appstate.get_user(user_id) {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("invalid user"),
    };

    let time = Utc::now().timestamp();

    if time - user.cooldown > appstate.cooldown() as i64 {
        HttpResponse::Ok().json(0)
    } else {
        HttpResponse::Ok().json(appstate.cooldown() - (time - user.cooldown) as u16)
    }
}

#[get("/api/leaderboard")]
async fn get_leaderboard(appstate: web::Data<RwLock<AppState>>) -> impl Responder {
    let appstate = match appstate.read() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    HttpResponse::Ok().json(appstate.get_leaderboard())
}
