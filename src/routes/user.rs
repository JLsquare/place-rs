use std::sync::RwLock;

use actix_web::{get, post, web, HttpRequest, HttpResponse, Error, error};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rand::Rng;
use serde_derive::Deserialize;

use crate::database;
use crate::models::appstate::AppState;
use crate::models::user::User;
use crate::routes::utils::{token_to_id, Claims};

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

#[derive(Deserialize)]
pub struct ProfileEdit {
    pub username: String,
    pub password: String,
    pub current_password: String,
}

#[post("/api/login")]
async fn login(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    info: web::Json<LoginInfo>,
) -> Result<HttpResponse, Error> {
    let user_id = database
        .login(&info.username, &info.password)
        .map_err(|_| error::ErrorInternalServerError("database error"))?
        .ok_or_else(|| error::ErrorUnauthorized("invalid credentials"))?;

    let claims = Claims {
        id: user_id,
        exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
    };

    let appstate = appstate
        .read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    let token = encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(appstate.jwt_secret().as_bytes()),
    )
        .map_err(|_| error::ErrorInternalServerError("token encoding error"))?;

    Ok(HttpResponse::Ok().body(token))
}

#[post("/api/signup")]
async fn signup(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    info: web::Json<SignupInfo>,
) -> Result<HttpResponse, Error> {
    let mut appstate = appstate
        .write()
        .map_err(|_| error::ErrorInternalServerError("appstate write error"))?;

    if !appstate.email_regex().is_match(&info.email) {
        return Err(error::ErrorBadRequest("Invalid email format"));
    }

    let mut ubs_id = 0;

    if appstate.check_ubs_email() {
        if !appstate.ubs_regex().is_match(&info.email) {
            return Err(error::ErrorBadRequest("Not a UBS email"));
        }

        ubs_id = appstate
            .extract_id_regex()
            .captures(&info.email)
            .and_then(|captures| captures.get(1))
            .and_then(|capture| {
                let id_str = &capture.as_str()[1..];
                id_str.parse::<u32>().ok()
            })
            .ok_or_else(|| error::ErrorBadRequest("Invalid email format"))?;

        let is_id_registered = database
            .check_ubs_id(ubs_id)
            .map_err(|_| error::ErrorInternalServerError("database error"))?;

        if is_id_registered {
            return Err(error::ErrorBadRequest("UBS id already registered"));
        }
    }

    if info.username.len() < 3 || info.username.len() > 15 {
        return Err(error::ErrorBadRequest("username must be between 3 and 15 characters"));
    }

    if info.password.len() < 8 || info.password.len() > 128 {
        return Err(error::ErrorBadRequest("password must be between 8 and 128 characters"));
    }

    if appstate.is_username_taken(&info.username) {
        return Err(error::ErrorConflict("username taken"));
    }

    let verification_code = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect::<String>();

    let user_id = database
        .signup(&info.username, &info.password, &info.email, &verification_code, ubs_id)
        .map_err(|_| error::ErrorInternalServerError("database error"))?;

    let user = User::new(info.username.clone(), 0, false);

    appstate.insert_user(user_id, user);
    appstate.send_verification_mail(&info.email, &verification_code)
        .map_err(|_| error::ErrorInternalServerError("email error"))?;

    Ok(HttpResponse::Ok().body("ok"))
}

#[get("/api/ubs")]
async fn ubs(appstate: web::Data<RwLock<AppState>>) -> Result<HttpResponse, Error> {
    let appstate = appstate
        .read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    Ok(HttpResponse::Ok().json(appstate.check_ubs_email()))
}

#[get("/api/verify/{token}")]
async fn verify(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    token: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let user_id = database
        .verify(&token)
        .map_err(|_| error::ErrorInternalServerError("database error"))?;

    let mut appstate = appstate
        .write()
        .map_err(|_| error::ErrorInternalServerError("appstate write error"))?;

    let user = appstate
        .get_user_mut(user_id)
        .ok_or_else(|| error::ErrorBadRequest("invalid user"))?;

    user.verified = true;

    Ok(HttpResponse::Ok().body("Account verified"))
}

#[get("/api/profile/me")]
async fn get_profile(appstate: web::Data<RwLock<AppState>>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let appstate = appstate.read()
        .map_err(|_| error::ErrorInternalServerError("appstate read error"))?;

    let user_id = token_to_id(req, appstate.jwt_secret().as_bytes())?;
    let user = appstate.get_user(user_id).ok_or_else(|| error::ErrorBadRequest("invalid user"))?;

    Ok(HttpResponse::Ok().json(user))
}

#[post("/api/profile/edit")]
async fn edit_profile(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    info: web::Json<ProfileEdit>,
    req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let mut appstate = appstate
        .write()
        .map_err(|_| error::ErrorInternalServerError("appstate write error"))?;

    let user_id = token_to_id(req, appstate.jwt_secret().as_bytes())?;

    if info.username.len() < 3 || info.username.len() > 15 {
        return Err(error::ErrorBadRequest("username must be between 3 and 15 characters"));
    }

    let user = appstate
        .get_user(user_id)
        .ok_or_else(|| error::ErrorBadRequest("invalid user"))?;

    if user.username != info.username && appstate.is_username_taken(&info.username) {
        return Err(error::ErrorBadRequest("username taken"));
    }

    let is_valid_password = database
        .check_password(user_id, &info.current_password)
        .map_err(|_| error::ErrorInternalServerError("database error"))?;

    if !is_valid_password {
        return Err(error::ErrorBadRequest("invalid credentials"));
    }

    let user = appstate
        .get_user_mut(user_id)
        .ok_or_else(|| error::ErrorBadRequest("invalid user"))?;

    user.username = info.username.clone();

    database
        .edit_profile(user_id, &info.into_inner())
        .map_err(|_| error::ErrorBadRequest("database error"))?;

    Ok(HttpResponse::Ok().body("ok"))
}