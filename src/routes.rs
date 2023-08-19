use std::env;
use std::sync::RwLock;
use actix_web::{get, HttpRequest, HttpResponse, post, Responder, web};
use chrono::{Duration, Utc};
use serde_derive::Deserialize;
use crate::appstate::AppState;
use jsonwebtoken::{Algorithm, encode, EncodingKey, Header};
use rand::Rng;
use regex::Regex;
use crate::database;
use crate::utils::{Claims, token_to_username};

#[derive(Deserialize)]
struct DrawInfo {
    x: u32,
    y: u32,
    color: u8,
}

#[derive(Deserialize)]
struct LoginInfo {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct SignupInfo {
    username: String,
    password: String,
    email: String,
}

#[get("/api/pixels")]
async fn get_grid(data: web::Data<RwLock<AppState>>) -> impl Responder {
    let pixels_color = match data.write() {
        Ok(mut data) => data.get_pixels_color(),
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    HttpResponse::Ok()
        .append_header(("Content-Encoding", "gzip"))
        .body(pixels_color)
}

#[post("/api/draw")]
async fn draw(
    data: web::Data<RwLock<AppState>>,
    info: web::Json<DrawInfo>,
    req: HttpRequest,
) -> impl Responder {
    let cooldown = env::var("COOLDOWN_SEC")
        .expect("COOLDOWN must be set")
        .parse::<i64>()
        .expect("COOLDOWN should be a valid u64");

    let username = match token_to_username(req) {
        Ok(username) => username,
        Err(response) => return response,
    };

    let mut data = match data.write() {
        Ok(data) => data,
        Err(err) => return HttpResponse::InternalServerError().body(format!("error: {}", err)),
    };

    let database = match database::Database::new() {
        Ok(database) => database,
        Err(err) => return HttpResponse::InternalServerError().body(format!("error: {}", err)),
    };

    match database.draw(info.x, info.y, info.color, &username) {
        Ok(_) => (),
        Err(err) => return HttpResponse::InternalServerError().body(format!("error: {}", err)),
    }

    data.draw(info.x as usize, info.y as usize, &username, info.color);

    HttpResponse::Ok().json(cooldown)
}

#[post("/api/login")]
async fn login(info: web::Json<LoginInfo>) -> impl Responder {
    let database = match database::Database::new() {
        Ok(database) => database,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    match database.login(&info.username, &info.password) {
        Ok(true) => (),
        Ok(false) => return HttpResponse::Unauthorized().body("invalid credentials"),
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    let claims = Claims {
        username: info.username.clone(),
        exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
    };

    let key = "temp_secret_key".as_bytes();

    match encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(key),
    ) {
        Ok(token) => HttpResponse::Ok().body(token),
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    }
}

#[post("/api/signup")]
async fn signup(
    info: web::Json<SignupInfo>,
    data: web::Data<RwLock<AppState>>,
) -> impl Responder {
    let email_regex = match Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$") {
        Ok(email_regex) => email_regex,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };
    let ubs_regex = match Regex::new(r"^[a-z0-9.]+@(etud\.)?univ-ubs\.fr$") {
        Ok(ubs_regex) => ubs_regex,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    if !email_regex.is_match(&info.email) {
        return HttpResponse::BadRequest().body("Invalid email format");
    }

    if !ubs_regex.is_match(&info.email) {
        return HttpResponse::BadRequest().body("Invalid email domain");
    }

    let database = match database::Database::new() {
        Ok(database) => database,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    let verification_code = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect::<String>();

    match database.signup(&info.username, &info.password, &info.email, &verification_code) {
        Ok(_) => (),
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    }

    let data = match data.read() {
        Ok(data) => data,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    data.send_verification_mail(&info.email, &verification_code);

    HttpResponse::Ok().body("ok")
}

#[get("/api/leaderboard")]
async fn get_leaderboard() -> impl Responder {
    let database = match database::Database::new() {
        Ok(database) => database,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    match database.get_leaderboard() {
        Ok(get_leaderboard) => HttpResponse::Ok().json(get_leaderboard),
        Err(_) => HttpResponse::InternalServerError().body("error"),
    }
}

#[get("/api/verify/{token}")]
async fn verify(token: web::Path<String>) -> impl Responder {
    let database = match database::Database::new() {
        Ok(database) => database,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    match database.verify(&token) {
        Ok(_) => HttpResponse::Ok().body("Account verified"),
        Err(_) => HttpResponse::InternalServerError().body("error"),
    }
}

#[get("/api/cooldown")]
async fn get_cooldown(
    req: HttpRequest,
) -> impl Responder {
    let cooldown = env::var("COOLDOWN_SEC")
        .expect("COOLDOWN must be set")
        .parse::<i64>()
        .expect("COOLDOWN should be a valid u64");

    let username = match token_to_username(req) {
        Ok(username) => username,
        Err(response) => return response,
    };

    let database = match database::Database::new() {
        Ok(database) => database,
        Err(_) => return HttpResponse::InternalServerError().body("error"),
    };

    match database.get_cooldown(&username) {
        Ok(get_cooldown) => HttpResponse::Ok().json(get_cooldown + cooldown),
        Err(_) => HttpResponse::InternalServerError().body("error"),
    }
}

#[get("/api/size")]
async fn get_size(
    data: web::Data<RwLock<AppState>>,
) -> impl Responder {
    match data.read() {
        Ok(data) => HttpResponse::Ok().json(data.get_size()),
        Err(_) => HttpResponse::InternalServerError().body("error"),
    }
}