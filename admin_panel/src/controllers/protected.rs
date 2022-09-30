use std::sync::Arc;

use axum::{response::Html, Extension, Json};
use serde_json::{json, Value};
use tokio::sync::oneshot::channel;

use crate::{
    error::ProxyMessageError,
    models::{self, auth::Claims},
    server::proxy_service::ProxyMessage,
    Context,
};

static HOME_PAGE: &str = include_str!("../../static/home.html");

pub async fn home(_claims: Claims) -> Html<String> {
    Html::from(HOME_PAGE.to_owned())
}

pub async fn ban_user(
    Json(ban): Json<models::protected::Ban>,
    Extension(context): Extension<Arc<Context>>,
    _claims: Claims,
) -> Result<Json<Value>, ProxyMessageError> {
    let message = ProxyMessage::Ban {
        nickname: ban.nickname,
        reason: ban.reason,
    };
    send_message(context, message).await
}

pub async fn pardon(
    Json(pardon): Json<models::protected::Pardon>,
    Extension(context): Extension<Arc<Context>>,
    _claims: Claims,
) -> Result<Json<Value>, ProxyMessageError> {
    let message = ProxyMessage::Pardon {
        nickname: pardon.nickname,
    };
    send_message(context, message).await
}

pub async fn kick_user(
    Json(kick): Json<models::protected::Kick>,
    Extension(context): Extension<Arc<Context>>,
    _claims: Claims,
) -> Result<Json<Value>, ProxyMessageError> {
    let message = ProxyMessage::Kick {
        nickname: kick.nickname,
        reason: kick.reason,
    };
    send_message(context, message).await
}

pub async fn whitelist_add(
    Json(whitelist_add): Json<models::protected::WhitelistAdd>,
    Extension(context): Extension<Arc<Context>>,
    _claims: Claims,
) -> Result<Json<Value>, ProxyMessageError> {
    let message = ProxyMessage::WhitelistAdd {
        nickname: whitelist_add.nickname,
    };
    send_message(context, message).await
}

pub async fn whitelist_remove(
    Json(whitelist_remove): Json<models::protected::WhitelistRemove>,
    Extension(context): Extension<Arc<Context>>,
    _claims: Claims,
) -> Result<Json<Value>, ProxyMessageError> {
    let message = ProxyMessage::WhitelistRemove {
        nickname: whitelist_remove.nickname,
    };
    send_message(context, message).await
}

pub async fn server_status(
    Extension(context): Extension<Arc<Context>>,
    _claims: Claims,
) -> Result<Json<Value>, ProxyMessageError> {
    let message = ProxyMessage::Ping;
    send_message(context, message).await
}

async fn send_message(
    context: Arc<Context>,
    message: ProxyMessage,
) -> Result<Json<Value>, ProxyMessageError> {
    let (rx, tx) = channel();

    context.tx.send((message, rx)).await?;

    let response = tx.await?;
    Ok(Json(json!({ "success": true, "response": response })))
}
