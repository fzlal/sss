//! Online and LAN play.
//!
//! The host (or a dedicated `--server` process) owns the simulation. Clients
//! send their input and render snapshot streams. Transport is plain UDP with
//! sequence numbers, fragment reassembly and periodic snapshots, which keeps
//! latency low on LAN and weak internet links.

pub mod cli;
pub mod client;
pub mod lan;
pub mod server;

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::game::config::{InputFrame, Sport};
use crate::game::sim::{Event, Match};

pub use client::{Client, ClientStatus};
pub use lan::{LanBrowser, LanOffer};
pub use server::{Server, ServerMode, ServerStatus};

pub const PROTOCOL_VERSION: u16 = 2;
pub const NET_TICK_RATE: u32 = 120;
pub const SNAPSHOT_RATE: u32 = 30;
pub const INPUT_RATE: u32 = 60;
pub const INPUT_HISTORY: usize = 8;
pub const DEFAULT_PORT: u16 = 43210;
pub const SNAPSHOT_INTERVAL: f32 = 1.0 / SNAPSHOT_RATE as f32;
pub const NET_STEP: f32 = 1.0 / NET_TICK_RATE as f32;

const MAX_PACKET_BYTES: usize = 64 * 1024;
const SNAPSHOT_CHUNK_BYTES: usize = 1000;
const MAX_SNAPSHOT_CHUNKS: u8 = 48;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NetPacket {
    Hello {
        version: u16,
        name: String,
        char_id: u8,
    },
    Welcome {
        version: u16,
        side: u8,
        players: u8,
        needed: u8,
        name: String,
    },
    Reject {
        reason: String,
    },
    Input {
        side: u8,
        newest_seq: u32,
        frames: Vec<InputFrame>,
    },
    Snapshot {
        seq: u32,
        tick: u64,
        recipient: u8,
        chunk: u8,
        chunks: u8,
        data: Vec<u8>,
    },
    Ping {
        stamp: u64,
    },
    Pong {
        stamp: u64,
    },
    Bye {
        reason: String,
    },
    Discover {
        version: u16,
    },
    Offer {
        version: u16,
        name: String,
        port: u16,
        sport: Sport,
        play_to: u8,
        players: u8,
        max_players: u8,
        in_match: bool,
    },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SnapshotPayload {
    pub state: Match,
    pub events: Vec<u8>,
}

pub fn encode(packet: &NetPacket) -> Option<Vec<u8>> {
    postcard::to_stdvec(packet).ok()
}

pub fn decode(bytes: &[u8]) -> Option<NetPacket> {
    if bytes.is_empty() || bytes.len() > MAX_PACKET_BYTES {
        return None;
    }
    postcard::from_bytes(bytes).ok()
}

pub fn encode_snapshot(snapshot: &SnapshotPayload) -> Option<Vec<u8>> {
    postcard::to_stdvec(snapshot).ok()
}

pub fn decode_snapshot(bytes: &[u8]) -> Option<SnapshotPayload> {
    if bytes.len() > MAX_PACKET_BYTES {
        return None;
    }
    postcard::from_bytes(bytes).ok()
}

pub fn split_snapshot(data: &[u8]) -> Vec<Vec<u8>> {
    if data.is_empty() {
        return vec![Vec::new()];
    }
    let chunks: Vec<Vec<u8>> = data
        .chunks(SNAPSHOT_CHUNK_BYTES)
        .map(|c| c.to_vec())
        .collect();
    if chunks.len() > MAX_SNAPSHOT_CHUNKS as usize {
        return Vec::new();
    }
    chunks
}

pub fn normalize_address(input: &str) -> Result<SocketAddr, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("enter a host address".into());
    }
    let candidate = if trimmed.starts_with('[') || trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("{trimmed}:{DEFAULT_PORT}")
    };
    candidate
        .to_socket_addrs()
        .map_err(|e| format!("bad address: {e}"))?
        .next()
        .ok_or_else(|| "could not resolve address".to_string())
}

pub fn local_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect((Ipv4Addr::new(8, 8, 8, 8), 80)).ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_unspecified() => Some(ip),
        _ => None,
    }
}

pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '_' | '-'))
        .take(18)
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "PLAYER".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn side_label(side: usize) -> &'static str {
    if side == 0 {
        "P1"
    } else {
        "P2"
    }
}

/// Sound effects shared with clients. Kept as bytes so snapshots stay small.
pub mod event_code {
    pub const COUNTDOWN: u8 = 1;
    pub const GO: u8 = 2;
    pub const JUMP: u8 = 3;
    pub const KICK: u8 = 4;
    pub const HEAD: u8 = 5;
    pub const BOUNCE: u8 = 6;
    pub const NET: u8 = 7;
    pub const POST: u8 = 8;
    pub const ABILITY: u8 = 9;
    pub const DENY: u8 = 10;
    pub const WHISTLE: u8 = 11;
    pub const MATCH_END: u8 = 12;
    pub const EXPLOSION: u8 = 13;
    pub const SCORE_SOCCER: u8 = 14;
    pub const SCORE_VOLLEY: u8 = 15;
    pub const SCORE_BASKET: u8 = 16;
}

pub fn event_code(event: &Event) -> Option<u8> {
    use event_code as c;
    Some(match event {
        Event::Countdown(_) => c::COUNTDOWN,
        Event::Go => c::GO,
        Event::Jump { .. } => c::JUMP,
        Event::Kick { head, .. } => {
            if *head {
                c::HEAD
            } else {
                c::KICK
            }
        }
        Event::Bounce { kind, .. } => match *kind {
            "net" => c::NET,
            "post" | "rim" => c::POST,
            _ => c::BOUNCE,
        },
        Event::Ability { .. } => c::ABILITY,
        Event::Deny { .. } => c::DENY,
        Event::Score { sport, .. } => match sport {
            Sport::Soccer => c::SCORE_SOCCER,
            Sport::Volleyball => c::SCORE_VOLLEY,
            Sport::Basketball => c::SCORE_BASKET,
        },
        Event::Whistle { .. } => c::WHISTLE,
        Event::MatchEnd { .. } => c::MATCH_END,
        Event::Explosion => c::EXPLOSION,
        Event::Geyser { .. } => c::BOUNCE,
        Event::DiscoHit { .. } | Event::ProjectileHit { .. } | Event::WaveHit { .. } => c::KICK,
    })
}

pub fn send_packet(socket: &UdpSocket, addr: SocketAddr, packet: &NetPacket) {
    if let Some(bytes) = encode(packet) {
        let _ = socket.send_to(&bytes, addr);
    }
}

pub fn send_connected(socket: &UdpSocket, packet: &NetPacket) {
    if let Some(bytes) = encode(packet) {
        let _ = socket.send(&bytes);
    }
}

pub fn input_timeout() -> Duration {
    Duration::from_millis(450)
}

pub fn client_timeout() -> Duration {
    Duration::from_secs(3)
}

pub fn server_timeout() -> Duration {
    Duration::from_secs(6)
}

pub fn now_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5eed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::config::{ManaMode, Sport};
    use crate::game::sim::{Match, MatchConfig};

    #[test]
    fn packets_round_trip() {
        let packet = NetPacket::Hello {
            version: PROTOCOL_VERSION,
            name: "slime".into(),
            char_id: 7,
        };
        assert_eq!(
            decode(&encode(&packet).unwrap()).unwrap(),
            NetPacket::Hello {
                version: PROTOCOL_VERSION,
                name: "slime".into(),
                char_id: 7
            }
        );
    }

    #[test]
    fn snapshots_round_trip_game_state() {
        let mut m = Match::new(MatchConfig {
            sport: Sport::Basketball,
            arena: 3,
            p1: 2,
            p2: 9,
            play_to: 5,
            mana_mode: ManaMode::Round,
            seed: 77,
            p1_control: 1.0,
            p2_control: 1.0,
        });
        m.update(NET_STEP, [InputFrame::NONE; 2]);
        let payload = SnapshotPayload {
            state: m.clone(),
            events: vec![event_code::GO],
        };
        let bytes = encode_snapshot(&payload).unwrap();
        let decoded = decode_snapshot(&bytes).unwrap();
        assert_eq!(decoded.state.tick, m.tick);
        assert_eq!(decoded.state.config, m.config);
        assert_eq!(decoded.state.players[0].char_id, 2);
        assert_eq!(decoded.state.players[1].char_id, 9);
        assert_eq!(decoded.state.players[0].x, m.players[0].x);
        assert_eq!(decoded.events, vec![event_code::GO]);
    }

    #[test]
    fn snapshot_chunks_can_be_split_and_joined() {
        let data: Vec<u8> = (0..2500).map(|i| (i % 251) as u8).collect();
        let chunks = split_snapshot(&data);
        assert_eq!(chunks.len(), 3);
        let joined: Vec<u8> = chunks.into_iter().flatten().collect();
        assert_eq!(joined, data);
    }

    #[test]
    fn address_normalization_appends_the_default_port() {
        let addr = normalize_address("127.0.0.1").unwrap();
        assert_eq!(addr.port(), DEFAULT_PORT);
        let addr = normalize_address("127.0.0.1:5000").unwrap();
        assert_eq!(addr.port(), 5000);
    }

    #[test]
    fn event_codes_are_stable() {
        assert_eq!(event_code(&Event::Go), Some(event_code::GO));
        assert_eq!(
            event_code(&Event::Score {
                side: 0,
                own_goal: false,
                score: [1, 0],
                pending_end: false,
                sport: Sport::Soccer,
            }),
            Some(event_code::SCORE_SOCCER)
        );
    }
}
