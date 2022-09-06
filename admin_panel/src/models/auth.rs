use std::sync::Arc;

use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    headers::Cookie,
    Extension, TypedHeader,
};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::{error::LoginError, Context};

#[derive(Deserialize)]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Claims {
    pub loged_in: bool,
    pub exp: i64,
}

#[async_trait]
impl<B> FromRequest<B> for Claims
where
    B: Send,
{
    type Rejection = LoginError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let ctx: Extension<Arc<Context>> = Extension::from_request(req)
            .await
            .map_err(|_| LoginError::ContextNotLoaded)?;

        let TypedHeader(cookies) = TypedHeader::<Cookie>::from_request(req)
            .await
            .map_err(|_| LoginError::InvalidToken)?;

        let token = cookies.get("jwt").ok_or(LoginError::InvalidToken)?;

        let data = decode(token, &ctx.keys.decoding, &Validation::default())
            .map_err(|_| LoginError::InvalidToken)?;
        Ok(data.claims)
    }
}
