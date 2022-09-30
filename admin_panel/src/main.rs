use std::fs::read_dir;
use std::path::PathBuf;
use std::{env::VarError, net::SocketAddr, sync::Arc, time::Duration};

use axum::routing::delete;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use controllers::{auth, protected};
use fs_extra::dir::CopyOptions;
use log::{error, info, warn};
use models::auth::Keys;
use rand::RngCore;
use server::online_poller::OnlinePoller;
use server::proxy_service::{ProxyMessage, ProxyResponse, ProxyService};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::{self, channel};
use tower_http::cors::{Any, CorsLayer};

use crate::env::Environment;

mod controllers;
mod env;
mod error;
mod logger;
mod models;
mod server;

pub struct Context {
    pub username: String,
    pub password: String,
    pub keys: Keys,
    pub tx: Sender<(ProxyMessage, oneshot::Sender<ProxyResponse>)>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    logger::init_logger().expect("Couldn't create logger, shutting down...");

    match recreate_symlinks() {
        Ok(_r) => {}
        Err(e) => {
            error!("Couldn't recreate symlinks: {}", &e);
            return;
        }
    }

    let (username, password) = match load_credentials() {
        Ok((u, p)) => (u, p),
        Err(e) => {
            error!("Couldn't load credentials: {}", &e);
            return;
        }
    };

    if username.is_empty() || password.is_empty() {
        error!("Username and password must not be empty!");
        return;
    }

    let env = Environment::load();
    if !env.eula_accepted() {
        return;
    }

    match env.save_server_parameters() {
        Ok(_r) => {}
        Err(e) => {
            error!("Couldn't check server parameters: {}", &e);
            return;
        }
    }

    let mut server = match start_server() {
        Ok(server_process) => server_process,
        Err(e) => {
            error!("Couldn't start Minecraft server: {}", &e);
            return;
        }
    };

    info!("Starting proxy layer...");
    let idle_timeout = Duration::from_secs(env.server_idle_timeout.get() as u64 * 60);
    let online_poller = match OnlinePoller::new().await {
        Ok(online_poller) => online_poller,
        Err(e) => {
            error!("Couldn't create Online watcher: {}", &e);
            return;
        }
    };

    let (proxy_service, tx) = ProxyService::new(online_poller, idle_timeout);
    let proxy_task = tokio::spawn(proxy_service.run());

    info!("Starting web server...");
    let mut secret = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut secret);
    let keys = Keys::new(&secret);

    let context = Arc::new(Context {
        username,
        password,
        keys,
        tx,
    });

    let router = Router::new()
        .route("/", get(auth::login))
        .route("/login", post(auth::login_post))
        .route("/home", get(protected::home))
        .route("/ban", post(protected::ban_user))
        .route("/ban", delete(protected::pardon))
        .route("/kick", post(protected::kick_user))
        .route("/whitelist", post(protected::whitelist_add))
        .route("/whitelist", delete(protected::whitelist_remove))
        .route("/ping", get(protected::server_status))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(Extension(context.clone()));

    let addr = SocketAddr::from(([0, 0, 0, 0], 80));
    let result = axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(async {
            // If MC server has crashed, proxy service will not be able to detect it.
            // Instead, we can await on MC server's process directly, and manually stop
            // proxy service, in case of a crash.
            match server.wait().await {
                Ok(r) => info!("MC server exit code: {}", &r),
                Err(e) => error!("Error while shutting down MC server: {}", &e),
            }
            // During the normal operation, proxy service will close before MC server,
            // so attempting to send a message to it will return an error
            let (rx, _) = channel();
            match context.tx.send((ProxyMessage::Quit, rx)).await {
                Ok(_r) => {}
                Err(_e) => info!("Proxy is closed"),
            }
            // If the server closed normally, proxy_task will return immediately.
            // In case of MC server crash, await for proxy service to process `Quit` message
            match proxy_task.await {
                Ok(_r) => {}
                Err(e) => error!("Error while waiting for proxy layer to shutdown: {}", &e),
            };
            // Move all files from `/server` to `/data` directory. Upon the next launch, the server will
            // create symlinks to these dirs and files, so from that point on Fabric and Minecraft server
            // will write to `/data` directory directly.
            match backup_files() {
                Ok(_r) => {}
                Err(e) => error!("Error while backuping server files: {}", &e),
            }
            info!("Closing web server...");
        })
        .await;

    match result {
        Ok(_r) => {}
        Err(e) => error!("Error while waiting for the web server to shutdown: {}", &e),
    };
}

fn load_credentials() -> Result<(String, String), VarError> {
    info!("Loading credentials...");
    let username = std::env::var("ADMIN_USERNAME")?;
    let password = std::env::var("ADMIN_PASSWORD")?;
    Ok((username, password))
}

fn recreate_symlinks() -> Result<(), std::io::Error> {
    info!("Recreating symlinks...");
    for path in read_dir("/data")? {
        let entry = path?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "world" {
            continue;
        }
        let file_type = entry.file_type()?;
        let counter_part = format!("/server/{}", &file_name);
        if file_type.is_dir() {
            symlink_dir(&entry.path(), &counter_part)?;
        } else if file_type.is_file() {
            symlink_file(&entry.path(), &counter_part);
        } else {
            warn!("{} is a symlink", &entry.path().to_string_lossy());
        }
    }
    Ok(())
}

#[cfg(target_family = "unix")]
fn symlink_file(original: &PathBuf, link: &str) {
    info!(
        "Linking files, from <{}> to <{}>",
        original.to_string_lossy(),
        link
    );
    match std::os::unix::fs::symlink(original, link) {
        Ok(_r) => {}
        Err(e) => warn!(
            "Couldn't link file <{}>: {}",
            original.to_string_lossy(),
            &e
        ),
    }
}

#[cfg(target_family = "windows")]
fn symlink_file(original: &PathBuf, link: &str) {
    info!(
        "Linking files, from <{}> to <{}>",
        original.to_string_lossy(),
        link
    );
    match std::os::windows::fs::symlink_file(original, link) {
        Ok(_r) => {}
        Err(e) => warn!(
            "Skipping file <{}> because couldn't create link: {}",
            original.to_string_lossy(),
            &e
        ),
    }
}

#[cfg(target_family = "unix")]
fn symlink_dir(original: &PathBuf, link: &str) -> Result<(), std::io::Error> {
    info!(
        "Linking dirs, from <{}> to <{}>",
        original.to_string_lossy(),
        link
    );
    std::os::unix::fs::symlink(original, link)
}

#[cfg(target_family = "windows")]
fn symlink_dir(original: &PathBuf, link: &str) -> Result<(), std::io::Error> {
    info!(
        "Linking dirs, from <{}> to <{}>",
        original.to_string_lossy(),
        link
    );
    std::os::windows::fs::symlink_dir(original, link)
}

fn backup_files() -> Result<(), std::io::Error> {
    info!("Backing up server files...");
    let mut paths = Vec::new();
    for path in read_dir("/server")? {
        let entry = path?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == "config"
            || file_name == "mods"
            || file_name == "fabric-server-launcher.jar"
            || file_name == "admin_panel"
            || file_name == "logs"
            || file_name == "server-icon.png"
        {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        paths.push(entry.path());
    }
    let copy_options = CopyOptions::new();
    fs_extra::move_items(&paths, "/data", &copy_options)
        .map(|_r| ())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

fn start_server() -> Result<Child, std::io::Error> {
    info!("Starting Minecraft server...");
    Command::new("java")
        .arg("-Dlog4j2.formatMsgNoLookups=true")
        .arg("-XX:MinRAMPercentage=50.0")
        .arg("-XX:MaxRAMPercentage=90.0")
        .arg("-XX:+UseG1GC")
        .arg("-XX:+ParallelRefProcEnabled")
        .arg("-XX:MaxGCPauseMillis=200")
        .arg("-XX:+UnlockExperimentalVMOptions")
        .arg("-XX:+PrintFlagsFinal")
        .arg("-XX:+DisableExplicitGC")
        .arg("-XX:+AlwaysPreTouch")
        .arg("-XX:G1NewSizePercent=30")
        .arg("-XX:G1MaxNewSizePercent=40")
        .arg("-XX:G1HeapRegionSize=8M")
        .arg("-XX:G1ReservePercent=20")
        .arg("-XX:G1HeapWastePercent=5")
        .arg("-XX:G1MixedGCCountTarget=4")
        .arg("-XX:InitiatingHeapOccupancyPercent=15")
        .arg("-XX:G1MixedGCLiveThresholdPercent=90")
        .arg("-XX:G1RSetUpdatingPauseTimePercent=5")
        .arg("-XX:SurvivorRatio=32")
        .arg("-XX:+PerfDisableSharedMem")
        .arg("-XX:MaxTenuringThreshold=1")
        .arg("-jar")
        .arg("fabric-server-launcher.jar")
        .arg("nogui")
        .spawn()
}
