use std::fmt::Display;
use std::io::Write;
use std::num::{NonZeroU16, NonZeroU8};
use std::{env::VarError, net::SocketAddr, path::Path, str::FromStr, sync::Arc, time::Duration};

use axum::routing::delete;
use axum::{
    routing::{get, post},
    Extension, Router,
};
use controllers::{auth, protected};
use error::DifficultyParserError;
use log::{error, info, warn};
use models::auth::Keys;
use rand::RngCore;
use server::online_poller::OnlinePoller;
use server::proxy_service::{ProxyMessage, ProxyResponse, ProxyService};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tower_http::cors::{Any, CorsLayer};

mod controllers;
mod error;
mod logger;
mod models;
mod server;

static SERVER_PROPERTIES: &str = include_str!("../static/server.properties");

pub struct Context {
    pub username: String,
    pub password: String,
    pub keys: Keys,
    pub tx: Sender<(ProxyMessage, oneshot::Sender<ProxyResponse>)>,
}

enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl FromStr for Difficulty {
    type Err = DifficultyParserError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let difficulty = match s.to_ascii_lowercase().as_str() {
            "peaceful" => Difficulty::Peaceful,
            "easy" => Difficulty::Easy,
            "normal" => Difficulty::Normal,
            "hard" => Difficulty::Hard,
            _ => return Err(DifficultyParserError::Parse(s.to_string())),
        };

        Ok(difficulty)
    }
}

impl Display for Difficulty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Difficulty::Peaceful => write!(f, "peaceful"),
            Difficulty::Easy => write!(f, "easy"),
            Difficulty::Normal => write!(f, "normal"),
            Difficulty::Hard => write!(f, "hard"),
        }
    }
}

pub struct Environment {
    eula: bool,
    difficulty: Difficulty,
    hardcore: bool,
    max_players: NonZeroU8,
    max_world_radius: NonZeroU16,
    motd: String,
    player_idle_timeout: NonZeroU8,
    server_idle_timeout: NonZeroU8,
    view_distance: NonZeroU8,
    pvp: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    logger::init_logger().expect("Couldn't create logger, shutting down...");

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

    let env = load_env();
    if !check_eula(&env) {
        return;
    }

    match check_server_parameters(&env) {
        Ok(_r) => {}
        Err(e) => {
            error!("Couldn't check server parameters: {}", &e);
            return;
        }
    }

    let server = match start_server() {
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

    let (proxy_service, tx) = ProxyService::new(server, online_poller, idle_timeout);
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
        .route("/generate", post(protected::generate_world))
        .route("/generate", delete(protected::cancel_generation))
        .route("/ping", get(protected::server_status))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(Extension(context));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let result = axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(async {
            match proxy_task.await {
                Ok(_r) => {}
                Err(e) => {
                    error!("Error while waiting for proxy layer to shutdown: {}", &e);
                }
            };

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

fn load_env() -> Environment {
    info!("Loading environment variables...");
    let eula = get_env("EULA", false);
    let difficulty = get_env("DIFFICULTY", Difficulty::Normal);
    let hardcore = get_env("HARDCORE", false);
    let max_players = get_env("MAX_PLAYERS", NonZeroU8::new(10).unwrap());
    let max_world_radius = get_env("MAX_WORLD_RADIUS", NonZeroU16::new(1000).unwrap());
    let motd = get_env("MOTD", "Minecraft on demand".to_owned());
    let player_idle_timeout = get_env("PLAYER_IDLE_TIMEOUT", NonZeroU8::new(10).unwrap());
    let server_idle_timeout = get_env("SERVER_IDLE_TIMEOUT", NonZeroU8::new(10).unwrap());
    let view_distance = get_env("VIEW_DISTANCE", NonZeroU8::new(10).unwrap());
    let pvp = get_env("PVP", false);
    Environment {
        eula,
        difficulty,
        hardcore,
        max_players,
        max_world_radius,
        motd,
        player_idle_timeout,
        server_idle_timeout,
        view_distance,
        pvp,
    }
}

fn get_env<T: FromStr>(key: &str, default: T) -> T {
    match std::env::var(key) {
        Ok(v) => match v.parse() {
            Ok(parsed) => parsed,
            Err(_) => {
                warn!(
                    "<{}>=<{}>: couldn't parse value, using default value",
                    key, &v
                );
                default
            }
        },
        Err(e) => {
            warn!("<{}>: couldn't get value: {}", key, &e);
            default
        }
    }
}

fn check_eula(environment: &Environment) -> bool {
    info!("Checking EULA...");
    let eula_path = Path::new("./eula.txt");
    if eula_path.exists() {
        return true;
    }

    if environment.eula {
        info!("eula.txt doesn't exist, creating...");
        match std::fs::write(&eula_path, "eula=true") {
            Ok(_r) => true,
            Err(e) => {
                error!("Couldn't create eula.txt: {}", &e);
                false
            }
        }
    } else {
        warn!("You need to accept Minecraft End User License Agreement");
        warn!("at https://account.mojang.com/documents/minecraft_eula");
        warn!("by setting <EULA> environment variably to <true>");
        false
    }
}

fn check_server_parameters(environment: &Environment) -> Result<(), std::io::Error> {
    info!("Checking server.properties...");
    let properties_path = Path::new("./server.properties");
    if properties_path.exists() {
        return Ok(());
    }

    info!("server.properties doesn't exist, creating...");
    let mut file = std::fs::File::create(&properties_path)?;
    writeln!(file, "{}", SERVER_PROPERTIES)?;
    writeln!(file, "difficulty={}", &environment.difficulty)?;
    writeln!(file, "hardcore={}", &environment.hardcore)?;
    writeln!(file, "max-players={}", &environment.max_players)?;
    writeln!(file, "max-world-size={}", &environment.max_world_radius)?;
    writeln!(file, "motd={}", &environment.motd)?;
    writeln!(
        file,
        "player-idle-timeout={}",
        &environment.player_idle_timeout
    )?;
    writeln!(file, "view-distance={}", &environment.view_distance)?;
    writeln!(file, "pvp={}", &environment.pvp)?;
    Ok(())
}

fn start_server() -> Result<Child, std::io::Error> {
    info!("Starting Minecraft server...");
    Command::new("java")
        .arg("-Dlog4j2.formatMsgNoLookups=true")
        .arg("-XX:+UseG1GC")
        .arg("-XX:+ParallelRefProcEnabled")
        .arg("-XX:MaxGCPauseMillis=200")
        .arg("-XX:+UnlockExperimentalVMOptions")
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
        .arg("fabric-server-launch.jar")
        .arg("nogui")
        .spawn()
}
