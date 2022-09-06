use std::{io, net::Ipv4Addr, num::ParseIntError, time::Duration};

use log::debug;
use thiserror::Error;
use tokio::{net::UdpSocket, time::timeout};

use crate::error::OnlinePollerError;

#[derive(Error, Debug)]
pub enum InnerError {
    #[error("Generic IO error: {0}")]
    IO(#[from] io::Error),
    #[error("Packet is too short: {0}")]
    TooShort(usize),
    #[error("Incorrect type. Expected: <{0}>, received: <{1}>")]
    IncorrectType(u8, u8),
    #[error("Response doesn't have `challenge` field.")]
    NoChallenge,
    #[error("Couldn't parse challenge: {0}")]
    Parsing(#[from] ParseIntError),
    #[error("Packet doesn't have <{0}> field")]
    NoField(String),
    #[error("Read timeout")]
    ReadTimeout,
}

pub struct OnlinePoller {
    socket: UdpSocket,
    read_timeout: Duration,
}

struct Challenge {
    challenge: u32,
    session: u32,
}

impl OnlinePoller {
    pub async fn new() -> Result<Self, OnlinePollerError> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
        socket.connect((Ipv4Addr::LOCALHOST, 25566)).await?;
        Ok(Self {
            socket,
            read_timeout: Duration::from_secs(1),
        })
    }

    pub async fn current_online(&self) -> Result<u32, InnerError> {
        let challenge = self.get_challenge().await?;
        self.get_online(challenge).await
    }

    async fn get_challenge(&self) -> Result<Challenge, InnerError> {
        debug!("Asking for the challenge");
        let mut packet = Vec::with_capacity(64);
        packet.push(0xFE);
        packet.push(0xFD);
        packet.push(0x09);

        let session = 0x01_u32.to_be_bytes();
        for byte in session {
            packet.push(byte);
        }

        debug!("Sending packet to the socket to get the challenge...");
        self.socket.send(&packet).await?;

        let mut buffer = [0u8; 512];
        debug!("Reading challenge from the socket...");
        let n = match timeout(self.read_timeout, self.socket.recv(&mut buffer)).await {
            Ok(r) => r?,
            Err(_e) => return Err(InnerError::ReadTimeout),
        };
        debug!("Received packet: {:?}", &buffer[..n]);
        if n < 5 {
            return Err(InnerError::TooShort(n));
        }

        let packet_type = buffer[0];
        if packet_type != 9 {
            return Err(InnerError::IncorrectType(9, packet_type));
        }

        let mut session = [0u8; 4];
        session.clone_from_slice(&buffer[1..=4]);
        let session = u32::from_be_bytes(session);

        let challenge = match (buffer[5..]).split(|byte| byte == &0).next() {
            Some(slice) => slice,
            None => return Err(InnerError::NoChallenge),
        };

        let challenge = String::from_utf8_lossy(challenge).parse::<u32>()?;
        Ok(Challenge { challenge, session })
    }

    async fn get_online(&self, challenge: Challenge) -> Result<u32, InnerError> {
        debug!("Asking for current online");
        let mut packet = Vec::with_capacity(64);
        packet.push(0xFE);
        packet.push(0xFD);
        packet.push(0x00);

        let session = challenge.session.to_be_bytes();
        for byte in session {
            packet.push(byte);
        }

        let challenge = challenge.challenge.to_be_bytes();
        for byte in challenge {
            packet.push(byte);
        }

        debug!("Sending packet to the socket to get the number of players online...");
        self.socket.send(&packet).await?;

        let mut buffer = [0u8; 512];
        debug!("Reading the number of players online from the socket...");
        let n = match timeout(self.read_timeout, self.socket.recv(&mut buffer)).await {
            Ok(r) => r?,
            Err(_e) => return Err(InnerError::ReadTimeout),
        };
        debug!("Received packet: {:?}", &buffer[..n]);
        if n < 5 {
            return Err(InnerError::TooShort(n));
        }

        let packet_type = buffer[0];
        if packet_type != 0 {
            return Err(InnerError::IncorrectType(0, packet_type));
        }

        let mut strings = buffer[5..].split(|byte| byte == &0);

        match strings.next() {
            Some(_) => {}
            None => return Err(InnerError::NoField("MOTD".to_string())),
        };

        match strings.next() {
            Some(_) => {}
            None => return Err(InnerError::NoField("Game type".to_string())),
        };

        match strings.next() {
            Some(_) => {}
            None => return Err(InnerError::NoField("Map".to_string())),
        };

        let current_online = match strings.next() {
            Some(bytes) => String::from_utf8_lossy(bytes).parse::<u32>()?,
            None => return Err(InnerError::NoField("Current online".to_string())),
        };

        Ok(current_online)
    }
}
