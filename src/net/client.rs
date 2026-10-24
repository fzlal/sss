use std::collections::VecDeque;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::game::config::InputFrame;
use crate::game::sim::{Match, Phase};

use super::{
    decode, decode_snapshot, normalize_address, sanitize_name, send_connected, NetPacket,
    SnapshotPayload, INPUT_HISTORY, INPUT_RATE, PROTOCOL_VERSION, SNAPSHOT_INTERVAL,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientStatus {
    Connecting,
    Lobby,
    Playing,
    Ended,
    Disconnected(String),
}

struct Reassembly {
    seq: u32,
    chunks: u8,
    parts: Vec<Option<Vec<u8>>>,
    received: usize,
}

pub struct Client {
    socket: UdpSocket,
    server: SocketAddr,
    name: String,
    char_id: usize,
    side: Option<u8>,
    status: ClientStatus,
    server_name: String,
    lobby_players: u8,
    lobby_needed: u8,
    prev: Option<Match>,
    last: Option<Match>,
    last_rx: Instant,
    last_hello: Instant,
    last_ping: Instant,
    ping_stamp: u64,
    ping_pending: Option<(u64, Instant)>,
    ping_ms: f32,
    assembly: Option<Reassembly>,
    last_snapshot_seq: u32,
    received_snapshots: u32,
    lost_snapshots: u32,
    events: Vec<u8>,
    samples: VecDeque<InputFrame>,
    input_seq: u32,
    sample_accum: f32,
}

impl Client {
    pub fn connect(addr: &str, name: &str, char_id: usize) -> Result<Self, String> {
        let server = normalize_address(addr)?;
        let socket = UdpSocket::bind(("0.0.0.0", 0)).map_err(|e| e.to_string())?;
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        socket.connect(server).map_err(|e| e.to_string())?;
        let now = Instant::now();
        let client = Self {
            socket,
            server,
            name: sanitize_name(name),
            char_id,
            side: None,
            status: ClientStatus::Connecting,
            server_name: String::new(),
            lobby_players: 1,
            lobby_needed: 2,
            prev: None,
            last: None,
            last_rx: now,
            last_hello: now,
            last_ping: now,
            ping_stamp: 0,
            ping_pending: None,
            ping_ms: 0.0,
            assembly: None,
            last_snapshot_seq: 0,
            received_snapshots: 0,
            lost_snapshots: 0,
            events: Vec::new(),
            samples: VecDeque::new(),
            input_seq: 0,
            sample_accum: 0.0,
        };
        client.send_hello();
        Ok(client)
    }

    pub fn server_addr(&self) -> SocketAddr {
        self.server
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    pub fn side(&self) -> Option<usize> {
        self.side.map(|side| side as usize)
    }

    pub fn status(&self) -> &ClientStatus {
        &self.status
    }

    pub fn ping_ms(&self) -> f32 {
        self.ping_ms
    }

    pub fn loss_percent(&self) -> f32 {
        let total = self.received_snapshots + self.lost_snapshots;
        if total == 0 {
            0.0
        } else {
            self.lost_snapshots as f32 / total as f32 * 100.0
        }
    }

    pub fn lobby_players(&self) -> u8 {
        self.lobby_players
    }

    pub fn lobby_needed(&self) -> u8 {
        self.lobby_needed
    }

    pub fn latest(&self) -> Option<&Match> {
        self.last.as_ref()
    }

    pub fn take_events(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.events)
    }

    pub fn quit(&mut self) {
        if !matches!(self.status, ClientStatus::Disconnected(_)) {
            send_connected(
                &self.socket,
                &NetPacket::Bye {
                    reason: "LEFT MATCH".into(),
                },
            );
            self.status = ClientStatus::Disconnected("LEFT MATCH".into());
        }
    }

    pub fn update(&mut self, dt: f32, input: InputFrame) {
        if matches!(self.status, ClientStatus::Disconnected(_)) {
            return;
        }
        self.poll();
        let now = Instant::now();

        if matches!(self.status, ClientStatus::Connecting)
            && now.duration_since(self.last_hello) > Duration::from_millis(400)
        {
            self.send_hello();
            self.last_hello = now;
        }

        if self.side.is_some() && matches!(self.status, ClientStatus::Playing) {
            let mut fresh = false;
            self.sample_accum += dt.clamp(0.0, 0.25);
            let step = 1.0 / INPUT_RATE as f32;
            let mut count = 0;
            while self.sample_accum >= step && count < 8 {
                self.sample_accum -= step;
                if self.samples.len() >= INPUT_HISTORY {
                    self.samples.pop_front();
                }
                self.samples.push_back(input);
                self.input_seq = self.input_seq.wrapping_add(1);
                fresh = true;
                count += 1;
            }
            if fresh {
                self.send_input();
            }
        }

        if now.duration_since(self.last_ping) > Duration::from_secs(1)
            && self
                .ping_pending
                .map(|(_, sent)| sent.elapsed() > Duration::from_secs(2))
                .unwrap_or(true)
        {
            self.ping_stamp = self.ping_stamp.wrapping_add(1);
            self.ping_pending = Some((self.ping_stamp, now));
            send_connected(
                &self.socket,
                &NetPacket::Ping {
                    stamp: self.ping_stamp,
                },
            );
            self.last_ping = now;
        }

        if matches!(self.status, ClientStatus::Connecting)
            && now.duration_since(self.last_rx) > Duration::from_secs(6)
        {
            self.status = ClientStatus::Disconnected("NO RESPONSE".into());
        } else if !matches!(
            self.status,
            ClientStatus::Connecting | ClientStatus::Disconnected(_)
        ) && now.duration_since(self.last_rx) > Duration::from_secs(3)
        {
            self.status = ClientStatus::Disconnected("CONNECTION LOST".into());
        }
    }

    fn send_hello(&self) {
        send_connected(
            &self.socket,
            &NetPacket::Hello {
                version: PROTOCOL_VERSION,
                name: self.name.clone(),
                char_id: self.char_id.min(u8::MAX as usize) as u8,
            },
        );
    }

    fn send_input(&self) {
        let side = self.side.unwrap_or(0);
        let frames: Vec<InputFrame> = self.samples.iter().copied().collect();
        send_connected(
            &self.socket,
            &NetPacket::Input {
                side,
                newest_seq: self.input_seq,
                frames,
            },
        );
    }

    fn poll(&mut self) {
        let mut buffer = [0u8; 2048];
        loop {
            match self.socket.recv(&mut buffer) {
                Ok(len) => {
                    if let Some(packet) = decode(&buffer[..len]) {
                        self.handle_packet(packet);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    fn handle_packet(&mut self, packet: NetPacket) {
        match packet {
            NetPacket::Welcome {
                version,
                side,
                players,
                needed,
                name,
            } => {
                if version != PROTOCOL_VERSION {
                    self.status = ClientStatus::Disconnected("PROTOCOL MISMATCH".into());
                    return;
                }
                self.side = Some(side);
                self.server_name = name;
                self.lobby_players = players;
                self.lobby_needed = needed;
                if !matches!(self.status, ClientStatus::Disconnected(_)) {
                    if self.last.is_some() {
                        self.status = ClientStatus::Playing;
                    } else {
                        self.status = ClientStatus::Lobby;
                    }
                }
                self.last_rx = Instant::now();
            }
            NetPacket::Reject { reason } => {
                self.status = ClientStatus::Disconnected(reason);
            }
            NetPacket::Snapshot {
                seq,
                tick,
                recipient,
                chunk,
                chunks,
                data,
            } => {
                if let Some(side) = self.side {
                    if recipient != side {
                        return;
                    }
                } else {
                    self.side = Some(recipient);
                }
                if chunks == 0 || chunk >= chunks || data.len() > 1200 {
                    return;
                }
                if self
                    .assembly
                    .as_ref()
                    .map(|assembly| assembly.seq != seq || assembly.chunks != chunks)
                    .unwrap_or(true)
                {
                    self.assembly = Some(Reassembly {
                        seq,
                        chunks,
                        parts: vec![None; chunks as usize],
                        received: 0,
                    });
                }
                let Some(assembly) = self.assembly.as_mut() else {
                    return;
                };
                let slot = &mut assembly.parts[chunk as usize];
                if slot.is_none() {
                    *slot = Some(data);
                    assembly.received += 1;
                }
                if assembly.received != chunks as usize {
                    return;
                }
                let Some(assembly) = self.assembly.take() else {
                    return;
                };
                let data: Vec<u8> = assembly.parts.into_iter().flatten().flatten().collect();
                if let Some(payload) = decode_snapshot(&data) {
                    self.apply_snapshot(seq, tick, payload);
                }
            }
            NetPacket::Pong { stamp } => {
                if let Some((pending, sent)) = self.ping_pending {
                    if pending == stamp {
                        self.ping_ms = sent.elapsed().as_secs_f32() * 1000.0;
                        self.ping_pending = None;
                    }
                }
            }
            NetPacket::Bye { reason } => {
                self.status = ClientStatus::Disconnected(reason);
            }
            _ => {}
        }
    }

    fn apply_snapshot(&mut self, seq: u32, tick: u64, payload: SnapshotPayload) {
        let state = payload.state;
        if state.tick != tick {
            return;
        }
        if self
            .last
            .as_ref()
            .map(|current| state.tick < current.tick)
            .unwrap_or(false)
        {
            return;
        }
        if self.received_snapshots > 0 {
            let gap = seq.wrapping_sub(self.last_snapshot_seq);
            if gap > 1 && gap < 1000 {
                self.lost_snapshots = self.lost_snapshots.saturating_add(gap - 1);
            }
        }
        self.received_snapshots = self.received_snapshots.saturating_add(1);
        self.last_snapshot_seq = seq;
        self.last_rx = Instant::now();
        self.events.extend(payload.events);
        self.status = if state.phase == Phase::Ended {
            ClientStatus::Ended
        } else {
            ClientStatus::Playing
        };
        self.prev = self.last.take();
        self.last = Some(state);
    }

    /// Interpolated snapshot for rendering. Falls back to the newest state
    /// when there is no previous snapshot yet or the stream just stalled.
    pub fn view(&self) -> Option<Match> {
        let last = self.last.as_ref()?;
        let mut view = last.clone();
        let elapsed = if matches!(self.status, ClientStatus::Disconnected(_)) {
            SNAPSHOT_INTERVAL
        } else {
            self.last_rx.elapsed().as_secs_f32()
        };
        view.time = last.time + elapsed;
        if let Some(prev) = &self.prev {
            let t = (elapsed / SNAPSHOT_INTERVAL).clamp(0.0, 1.0);
            if t < 1.0 {
                interpolate(&mut view, prev, last, t);
            }
        }
        Some(view)
    }
}

fn interpolate(view: &mut Match, prev: &Match, next: &Match, t: f32) {
    for side in 0..2 {
        view.players[side].x =
            prev.players[side].x + (next.players[side].x - prev.players[side].x) * t;
        view.players[side].y =
            prev.players[side].y + (next.players[side].y - prev.players[side].y) * t;
    }
    view.ball.x = prev.ball.x + (next.ball.x - prev.ball.x) * t;
    view.ball.y = prev.ball.y + (next.ball.y - prev.ball.y) * t;
    view.ball.spin = prev.ball.spin + (next.ball.spin - prev.ball.spin) * t;
    for entity in &mut view.entities {
        if let Some(old) = prev.entities.iter().find(|old| old.id == entity.id) {
            entity.x = old.x + (entity.x - old.x) * t;
            entity.y = old.y + (entity.y - old.y) * t;
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.quit();
    }
}
