use serde::Deserialize;

#[derive(Deserialize)]
pub struct Ban {
    pub nickname: String,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct Pardon {
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct Kick {
    pub nickname: String,
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct WhitelistAdd {
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct WhitelistRemove {
    pub nickname: String,
}

#[derive(Deserialize)]
pub struct GenerateWorld {
    pub radius: u16,
}
