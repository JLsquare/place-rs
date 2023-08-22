use std::sync::RwLock;

use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
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
) -> impl Responder {
    let user_id = match database.login(&info.username, &info.password) {
        Ok(Some(id)) => id,
        Ok(None) => return HttpResponse::Unauthorized().body("invalid credentials"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("database error : {}", err))
        }
    };

    let claims = Claims {
        id: user_id,
        exp: (Utc::now() + Duration::days(7)).timestamp() as usize,
    };

    let appstate = match appstate.read() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    match encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(appstate.jwt_secret().as_bytes()),
    ) {
        Ok(token) => HttpResponse::Ok().body(token),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("token encoding error : {}", err))
        }
    }
}

#[post("/api/signup")]
async fn signup(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    info: web::Json<SignupInfo>,
) -> impl Responder {
    let mut appstate = match appstate.write() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    if !appstate.email_regex().is_match(&info.email) {
        return HttpResponse::BadRequest().body("Invalid email format");
    }

    if !appstate.ubs_regex().is_match(&info.email) {
        return HttpResponse::BadRequest().body("Invalid email domain");
    }

    if info.username.len() < 3 || info.username.len() > 15 {
        return HttpResponse::BadRequest().body("username must be between 3 and 15 characters");
    }

    if info.password.len() < 8 || info.password.len() > 128 {
        return HttpResponse::BadRequest().body("password must be between 8 and 128 characters");
    }

    let verification_code = rand::thread_rng()
        .sample_iter(rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect::<String>();

    let user_id = match database.signup(
        &info.username,
        &info.password,
        &info.email,
        &verification_code,
    ) {
        Ok(id) => id,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("database error : {}", err))
        }
    };

    let user = User::new(info.username.clone(), 0, false);

    appstate.insert_user(user_id, user);
    appstate.send_verification_mail(&info.email, &verification_code);

    HttpResponse::Ok().body("ok")
}

#[get("/api/verify/{token}")]
async fn verify(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    token: web::Path<String>,
) -> impl Responder {
    let user_id = match database.verify(&token) {
        Ok(id) => id,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("database error : {}", err))
        }
    };

    let mut appstate = match appstate.write() {
        Ok(appstate) => appstate,
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("appstate error : {}", err))
        }
    };

    match appstate.get_user_mut(user_id) {
        Some(user) => user.verified = true,
        None => return HttpResponse::InternalServerError().body("appstate error : no such user"),
    };

    HttpResponse::Ok().body("Account verified")
}

#[post("/api/profile/edit")]
async fn edit_profile(
    appstate: web::Data<RwLock<AppState>>,
    database: web::Data<database::Database>,
    info: web::Json<ProfileEdit>,
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

    if info.username.len() < 3 || info.username.len() > 15 {
        return HttpResponse::BadRequest().body("username must be between 3 and 15 characters");
    }

    if appstate.is_username_taken(&info.username) {
        return HttpResponse::BadRequest().body("username taken");
    }

    match database.check_password(user_id, &info.current_password) {
        Ok(true) => (),
        Ok(false) => return HttpResponse::Unauthorized().body("invalid credentials"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("database error : {}", err))
        }
    };

    let user = match appstate.get_user_mut(user_id) {
        Some(user) => user,
        None => return HttpResponse::BadRequest().body("invalid user"),
    };

    user.username = info.username.clone();

    match database.edit_profile(user_id, &info.into_inner()) {
        Ok(_) => (),
        Err(err) => return HttpResponse::InternalServerError().body(format!("error: {}", err)),
    };

    HttpResponse::Ok().body("ok")
}
