use std::time::Duration;

use log::{debug, error, info, warn};
use minecraft_client_rs::Client;
use serde::Serialize;
use tokio::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        oneshot,
    },
    time::{sleep, timeout_at, Instant},
};

use crate::error::ProxyResponseError;

use super::online_poller::OnlinePoller;

#[derive(Debug)]
pub enum ProxyMessage {
    Ban {
        nickname: String,
        reason: Option<String>,
    },
    Pardon {
        nickname: String,
    },
    Kick {
        nickname: String,
        reason: Option<String>,
    },
    WhitelistAdd {
        nickname: String,
    },
    WhitelistRemove {
        nickname: String,
    },
    OpAdd {
        nickname: String,
    },
    DeOp {
        nickname: String,
    },
    Ping,
}

#[derive(Debug, Serialize)]
pub enum ProxyResponse {
    NotReady,
    Ok { response: String },
    Err { error: String },
}

enum ServerStatus {
    Starting(Instant),
    Idle(Instant),
    Busy,
}

pub struct ProxyService {
    online_poller: OnlinePoller,
    status: ServerStatus,
    idle_timeout: Duration,
    rx: Receiver<(ProxyMessage, oneshot::Sender<ProxyResponse>)>,
    current_online: u32,
    last_request_time: Instant,
}

impl ProxyService {
    pub fn new(
        online_poller: OnlinePoller,
        idle_timeout: Duration,
    ) -> (Self, Sender<(ProxyMessage, oneshot::Sender<ProxyResponse>)>) {
        let start_time = Instant::now();
        let status = ServerStatus::Starting(start_time);
        let (tx, rx) = channel(16);
        (
            Self {
                online_poller,
                status,
                idle_timeout,
                rx,
                current_online: 0,
                last_request_time: start_time,
            },
            tx,
        )
    }

    pub async fn run(mut self) {
        match self.do_run().await {
            Ok(_r) => {}
            Err(e) => error!("Proxy service encountered an error: {}", &e),
        };
        info!("Sending shut down command to MC server...");
        for _ in 0..3 {
            if (self.shutdown().await).is_ok() {
                break;
            }
            error!("Error while shutting down MC server, attempting again in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn do_run(&mut self) -> Result<(), ProxyResponseError> {
        info!("Start polling...");
        let frequency = Duration::from_secs(5);
        let mut deadline = Instant::now() + frequency;
        loop {
            debug!("Polling new message for 5 seconds...");
            match timeout_at(deadline, self.rx.recv()).await {
                Ok(val) => {
                    let (message, rx) = match val {
                        Some((message, rx)) => (message, rx),
                        None => return Err(ProxyResponseError::IncomingChannelClosed),
                    };
                    info!("Received new message: {:?}", &message);
                    if matches!(&self.status, ServerStatus::Starting(_)) {
                        warn!("Server is not ready yet...");
                        rx.send(ProxyResponse::NotReady)?;
                        continue;
                    }
                    let response = match message {
                        ProxyMessage::Ban { nickname, reason } => self.ban(nickname, reason),
                        ProxyMessage::Pardon { nickname } => self.pardon(nickname),
                        ProxyMessage::Kick { nickname, reason } => self.kick(nickname, reason),
                        ProxyMessage::WhitelistAdd { nickname } => self.whitelist_add(nickname),
                        ProxyMessage::WhitelistRemove { nickname } => {
                            self.whitelist_remove(nickname)
                        }
                        ProxyMessage::OpAdd { nickname } => self.op_add(nickname),
                        ProxyMessage::DeOp { nickname } => self.de_op(nickname),
                        ProxyMessage::Ping => Ok(self.current_online.to_string()),
                    };

                    match response {
                        Ok(response) => rx.send(ProxyResponse::Ok { response })?,
                        Err(err) => match err {
                            ProxyResponseError::Spam => rx.send(ProxyResponse::Err {
                                error: err.to_string(),
                            })?,
                            ProxyResponseError::McServerConnect(_) => {
                                rx.send(ProxyResponse::Err {
                                    error: err.to_string(),
                                })?
                            }
                            ProxyResponseError::McServerAuth(_) => rx.send(ProxyResponse::Err {
                                error: err.to_string(),
                            })?,
                            ProxyResponseError::McServerCommand(_) => {
                                rx.send(ProxyResponse::Err {
                                    error: err.to_string(),
                                })?
                            }
                            _ => return Err(err),
                        },
                    }
                }
                Err(_e) => {
                    deadline = Instant::now() + frequency;
                    debug!("Timed out, polling MC server...");
                    let polling_result = self.online_poller.current_online().await;
                    self.current_online = match polling_result {
                        Ok(number_of_players) => number_of_players,
                        Err(_) => {
                            match &self.status {
                                ServerStatus::Starting(time) => {
                                    if time.elapsed() > self.idle_timeout {
                                        warn!("The server took too long to start");
                                        return Ok(());
                                    }
                                }
                                ServerStatus::Idle(time) => {
                                    if time.elapsed() > self.idle_timeout {
                                        info!("The server has been idle for too long");
                                        return Ok(());
                                    }
                                }
                                ServerStatus::Busy => {
                                    self.status = ServerStatus::Idle(Instant::now());
                                }
                            }
                            continue;
                        }
                    };

                    debug!("Current online: {}", &self.current_online);
                    match (&self.status, self.current_online) {
                        (ServerStatus::Starting(_), number) => {
                            self.status = {
                                info!("MC server is ready...");
                                if number == 0 {
                                    ServerStatus::Idle(Instant::now())
                                } else {
                                    ServerStatus::Busy
                                }
                            }
                        }
                        (ServerStatus::Busy, 0) => self.status = ServerStatus::Idle(Instant::now()),
                        (ServerStatus::Idle(time), 0) => {
                            if time.elapsed() > self.idle_timeout {
                                info!("The server has been idle for too long");
                                return Ok(());
                            } else {
                                info!(
                                    "Server has been idle for {} seconds",
                                    time.elapsed().as_secs()
                                );
                            }
                        }
                        (ServerStatus::Idle(_), _) => self.status = ServerStatus::Busy,
                        _ => {}
                    }
                }
            }
        }
    }

    async fn shutdown(&mut self) -> Result<(), ProxyResponseError> {
        let command = "/stop".to_string();
        let _ = self.send_command(command, false)?;
        Ok(())
    }

    fn ban(
        &mut self,
        nickname: String,
        reason: Option<String>,
    ) -> Result<String, ProxyResponseError> {
        let command = match reason {
            Some(reason) => format!("/ban {} {}", nickname, reason),
            None => format!("/ban {}", nickname),
        };
        self.send_command(command, true)
    }

    fn pardon(&mut self, nickname: String) -> Result<String, ProxyResponseError> {
        let command = format!("/pardon {}", nickname);
        self.send_command(command, true)
    }

    fn kick(
        &mut self,
        nickname: String,
        reason: Option<String>,
    ) -> Result<String, ProxyResponseError> {
        let command = match reason {
            Some(reason) => format!("/kick {} {}", nickname, reason),
            None => format!("/kick {}", nickname),
        };
        self.send_command(command, true)
    }

    fn whitelist_add(&mut self, nickname: String) -> Result<String, ProxyResponseError> {
        let command = format!("/whitelist add {}", nickname);
        self.send_command(command, true)
    }

    fn whitelist_remove(&mut self, nickname: String) -> Result<String, ProxyResponseError> {
        let command = format!("/whitelist remove {}", nickname);
        self.send_command(command, true)
    }

    fn op_add(&mut self, nickname: String) -> Result<String, ProxyResponseError> {
        let command = format!("/op {}", nickname);
        self.send_command(command, true)
    }

    fn de_op(&mut self, nickname: String) -> Result<String, ProxyResponseError> {
        let command = format!("/deop {}", nickname);
        self.send_command(command, true)
    }

    fn send_command(
        &mut self,
        command: String,
        protect_from_spam: bool,
    ) -> Result<String, ProxyResponseError> {
        if protect_from_spam && self.last_request_time.elapsed() < Duration::from_secs(5) {
            return Err(ProxyResponseError::Spam);
        }
        let mut client = match Client::new("127.0.0.1:25567".to_string()) {
            Ok(r) => r,
            Err(e) => return Err(ProxyResponseError::McServerConnect(e)),
        };

        let message = match client.authenticate("M1n3cr@ft".to_string()) {
            Ok(r) => r,
            Err(e) => return Err(ProxyResponseError::McServerAuth(e)),
        };
        info!("Message: {:?}", message);

        let message = match client.send_command(command) {
            Ok(r) => r,
            Err(e) => return Err(ProxyResponseError::McServerCommand(e)),
        };

        self.last_request_time = Instant::now();
        Ok(message.body)
    }
}
