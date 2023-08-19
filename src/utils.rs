use actix_web::{HttpRequest, HttpResponse};
use jsonwebtoken::{Algorithm, decode, DecodingKey, Validation};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Claims {
    pub username: String,
    pub exp: usize,
}

pub fn token_to_username(req: HttpRequest) -> Result<String, HttpResponse> {
    let header = match req.headers().get("Authorization") {
        Some(header) => header,
        None => return Err(HttpResponse::Unauthorized().body("invalid token")),
    };

    let token = match header.to_str() {
        Ok(token) => token,
        Err(_) => return Err(HttpResponse::Unauthorized().body("invalid token")),
    };
    match decode::<Claims>(
        token,
        &DecodingKey::from_secret("temp_secret_key".as_bytes()),
        &Validation::new(Algorithm::HS512),
    ) {
        Ok(token) => Ok(token.claims.username),
        Err(_) => Err(HttpResponse::Unauthorized().body("invalid token")),
    }
}