use std::{error::Error, io};

use axum::{http::StatusCode, response::IntoResponse, Json};
use log::SetLoggerError;
use log4rs::config::runtime::ConfigErrors;
use serde_json::json;
use thiserror::Error;
use tokio::sync::{mpsc::error::SendError, oneshot};

use crate::server::proxy_service::{ProxyMessage, ProxyResponse};

#[derive(Error, Debug)]
pub enum LogInitError {
    #[error("Couldn't initialize logger config: {0}")]
    Config(#[from] ConfigErrors),
    #[error("Couldn't create logger: {0}")]
    Create(#[from] SetLoggerError),
}

#[derive(Error, Debug)]
pub enum LoginError {
    #[error("Incorrect username and/or password")]
    WrongCredentials,
    #[error("Couldn't create token")]
    TokenCreation,
    #[error("Context wasn't loaded for some reason")]
    ContextNotLoaded,
    #[error("Invalid token")]
    InvalidToken,
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            LoginError::WrongCredentials => StatusCode::UNAUTHORIZED,
            LoginError::TokenCreation => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::ContextNotLoaded => StatusCode::INTERNAL_SERVER_ERROR,
            LoginError::InvalidToken => StatusCode::UNAUTHORIZED,
        };
        let msg = format!("{}", self);

        (status, Json(json!({ "success": false, "error": msg }))).into_response()
    }
}

#[derive(Error, Debug)]
pub enum OnlinePollerError {
    #[error("Couldn't create the watcher: {0}")]
    Create(#[from] io::Error),
}

impl IntoResponse for OnlinePollerError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            OnlinePollerError::Create(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let msg = format!("{}", self);

        (status, Json(json!({ "success": false, "error": msg }))).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ProxyMessageError {
    #[error("Couldn't send message to Minecraft server")]
    ChannelClosed,
    #[error("Couldn't receive response from Minecraft server")]
    IncomingChannelClosed,
}

impl From<SendError<(ProxyMessage, oneshot::Sender<ProxyResponse>)>> for ProxyMessageError {
    fn from(_: SendError<(ProxyMessage, oneshot::Sender<ProxyResponse>)>) -> Self {
        Self::ChannelClosed
    }
}

impl From<oneshot::error::RecvError> for ProxyMessageError {
    fn from(_: oneshot::error::RecvError) -> Self {
        Self::IncomingChannelClosed
    }
}

impl IntoResponse for ProxyMessageError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let msg = format!("{}", self);

        (status, Json(json!({ "success": false, "error": msg }))).into_response()
    }
}

#[derive(Error, Debug)]
pub enum ProxyResponseError {
    #[error("Response channel from the proxy server to the web layer is closed")]
    ResponseChannelClosed,
    #[error("Incoming channel from the web layer is closed")]
    IncomingChannelClosed,
    #[error("Sending messages to the server too often")]
    Spam,
    #[error("Can't connect to MC server: {0}")]
    McServerConnect(Box<dyn Error>),
    #[error("Couldn't authenticate to MC server: {0}")]
    McServerAuth(Box<dyn Error>),
    #[error("Couldn't send command to MC server: {0}")]
    McServerCommand(Box<dyn Error>),
    #[error("Couldn't shutdown MC server: {0}")]
    McShutdown(io::Error),
}

impl From<ProxyResponse> for ProxyResponseError {
    fn from(_: ProxyResponse) -> Self {
        Self::ResponseChannelClosed
    }
}
