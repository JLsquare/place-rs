use actix_web::{HttpRequest, HttpResponse};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Claims {
    pub id: u16,
    pub exp: usize,
}

pub fn token_to_id(req: HttpRequest, key: &[u8]) -> Result<u16, HttpResponse> {
    let header = match req.headers().get("Authorization") {
        Some(header) => header,
        None => return Err(HttpResponse::Unauthorized().body("No field `Authorization`")),
    };

    let token = match header.to_str() {
        Ok(token) => token,
        Err(_) => return Err(HttpResponse::Unauthorized().body("token str error")),
    };

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(key),
        &Validation::new(Algorithm::HS512),
    ) {
        Ok(token) => Ok(token.claims.id),
        Err(_) => Err(HttpResponse::Unauthorized().body("invalid token")),
    }
}
