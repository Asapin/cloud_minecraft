use log::{error, info, warn};
use std::io::Write;
use std::{
    fmt::Display,
    num::{NonZeroU16, NonZeroU8},
    path::Path,
    str::FromStr,
};

use crate::error::DifficultyParserError;

static SERVER_PROPERTIES: &str = include_str!("../static/server.properties");

pub enum Difficulty {
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
    pub eula: bool,
    pub difficulty: Difficulty,
    pub hardcore: bool,
    pub max_players: NonZeroU8,
    pub max_world_radius: NonZeroU16,
    pub motd: String,
    pub player_idle_timeout: NonZeroU8,
    pub server_idle_timeout: NonZeroU8,
    pub view_distance: NonZeroU8,
    pub pvp: bool,
}

impl Environment {
    pub fn load() -> Self {
        info!("Loading environment variables...");
        let eula = Self::get_env("EULA", false);
        let difficulty = Self::get_env("DIFFICULTY", Difficulty::Normal);
        let hardcore = Self::get_env("HARDCORE", false);
        let max_players = Self::get_env("MAX_PLAYERS", NonZeroU8::new(10).unwrap());
        let max_world_radius = Self::get_env("MAX_WORLD_RADIUS", NonZeroU16::new(1000).unwrap());
        let motd = Self::get_env("MOTD", "Minecraft on demand".to_owned());
        let player_idle_timeout = Self::get_env("PLAYER_IDLE_TIMEOUT", NonZeroU8::new(10).unwrap());
        let server_idle_timeout = Self::get_env("SERVER_IDLE_TIMEOUT", NonZeroU8::new(10).unwrap());
        let view_distance = Self::get_env("VIEW_DISTANCE", NonZeroU8::new(10).unwrap());
        let pvp = Self::get_env("PVP", false);
        Self {
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
            Ok(v) => match v.to_ascii_lowercase().parse() {
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

    pub fn eula_accepted(&self) -> bool {
        info!("Checking EULA...");
        let eula_path = Path::new("/server/eula.txt");
        if eula_path.exists() {
            return true;
        }

        if self.eula {
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

    pub fn save_server_parameters(&self) -> Result<(), std::io::Error> {
        info!("Checking server.properties...");
        let properties_path = Path::new("./server.properties");
        if properties_path.exists() {
            return Ok(());
        }

        info!("server.properties doesn't exist, creating...");
        let mut file = std::fs::File::create(&properties_path)?;
        writeln!(file, "{}", SERVER_PROPERTIES)?;
        writeln!(file, "difficulty={}", &self.difficulty)?;
        writeln!(file, "hardcore={}", &self.hardcore)?;
        writeln!(file, "max-players={}", &self.max_players)?;
        writeln!(file, "max-world-size={}", &self.max_world_radius)?;
        writeln!(file, "motd={}", &self.motd)?;
        writeln!(file, "player-idle-timeout={}", &self.player_idle_timeout)?;
        writeln!(file, "view-distance={}", &self.view_distance)?;
        writeln!(file, "pvp={}", &self.pvp)?;
        Ok(())
    }
}
