use std::sync::Arc;

use axum::{
    response::{Html, Redirect},
    Extension, Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use cookie::{
    time::{Duration, OffsetDateTime},
    SameSite,
};
use jsonwebtoken::{encode, Header};

use crate::{
    error::LoginError,
    models::{self, auth::Claims},
    Context,
};

static LOGIN_FORM: &str = include_str!("../../static/login.html");

pub async fn login() -> Html<String> {
    Html::from(LOGIN_FORM.to_owned())
}

pub async fn login_post(
    Form(credntials): Form<models::auth::LoginData>,
    Extension(context): Extension<Arc<Context>>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), LoginError> {
    let in_username = credntials.username.to_lowercase();
    let ref_username = context.username.to_lowercase();
    if in_username != ref_username {
        return Err(LoginError::WrongCredentials);
    }

    if credntials.password != context.password {
        return Err(LoginError::WrongCredentials);
    }

    let max_age = Duration::minutes(10);
    let expiration = OffsetDateTime::now_utc() + max_age;

    let claims = Claims {
        loged_in: true,
        exp: expiration.unix_timestamp(),
    };

    let token = encode(&Header::default(), &claims, &context.keys.encoding)
        .map_err(|_| LoginError::TokenCreation)?;

    let mut jwt = Cookie::new("jwt", token);
    jwt.set_http_only(true);
    jwt.set_secure(true);
    jwt.set_max_age(max_age);
    jwt.set_same_site(SameSite::Strict);
    let cookie_jar = jar.add(jwt);

    Ok((cookie_jar, Redirect::to("/home")))
}
