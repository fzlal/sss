use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::game::characters::CHARACTERS;
use crate::game::config::InputFrame;
use crate::game::sim::{Event, Match, MatchConfig, Phase};

use super::{
    decode, encode_snapshot, event_code, input_timeout, local_ipv4, sanitize_name, send_packet,
    split_snapshot, NetPacket, SnapshotPayload, INPUT_HISTORY, NET_STEP, PROTOCOL_VERSION,
    SNAPSHOT_INTERVAL,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerMode {
    Hosted,
    Dedicated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerStatus {
    Lobby,
    Playing,
    Ended,
    Aborted(String),
}

#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub name: String,
    pub side: usize,
    pub char_id: usize,
}

struct ClientSlot {
    addr: SocketAddr,
    side: usize,
    name: String,
    char_id: usize,
    last_frame: InputFrame,
    last_seq: u32,
    has_input: bool,
    pending_jump: bool,
    pending_ability: bool,
    last_heard: Instant,
    last_input: Instant,
}

impl ClientSlot {
    fn new(addr: SocketAddr, side: usize, name: String, char_id: usize) -> Self {
        let now = Instant::now();
        Self {
            addr,
            side,
            name,
            char_id,
            last_frame: InputFrame::NONE,
            last_seq: 0,
            has_input: false,
            pending_jump: false,
            pending_ability: false,
            last_heard: now,
            last_input: now,
        }
    }
}

pub struct Server {
    socket: UdpSocket,
    config: MatchConfig,
    mode: ServerMode,
    port: u16,
    name: String,
    slots: [Option<ClientSlot>; 2],
    match_state: Option<Match>,
    status: ServerStatus,
    accumulator: f32,
    snapshot_accum: f32,
    snapshot_seq: u32,
    events: Vec<Event>,
    net_events: Vec<u8>,
    notices: Vec<String>,
}

impl Server {
    pub fn new(
        config: MatchConfig,
        mode: ServerMode,
        port: u16,
        name: String,
    ) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_nonblocking(true)?;
        let port = socket.local_addr().map(|addr| addr.port()).unwrap_or(port);
        let mut config = config;
        config.p1 = config.p1.min(CHARACTERS.len() - 1);
        config.p2 = config.p2.min(CHARACTERS.len() - 1);
        Ok(Self {
            socket,
            config,
            mode,
            port,
            name: sanitize_name(&name),
            slots: [None, None],
            match_state: None,
            status: ServerStatus::Lobby,
            accumulator: 0.0,
            snapshot_accum: SNAPSHOT_INTERVAL,
            snapshot_seq: 0,
            events: Vec::new(),
            net_events: Vec::new(),
            notices: Vec::new(),
        })
    }

    pub fn config(&self) -> MatchConfig {
        self.match_state
            .as_ref()
            .map(|m| m.config)
            .unwrap_or(self.config)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn status(&self) -> &ServerStatus {
        &self.status
    }

    pub fn local_side(&self) -> Option<usize> {
        match self.mode {
            ServerMode::Hosted => Some(0),
            ServerMode::Dedicated => None,
        }
    }

    pub fn match_state(&self) -> Option<&Match> {
        self.match_state.as_ref()
    }

    pub fn peers(&self) -> Vec<PeerInfo> {
        self.slots
            .iter()
            .flatten()
            .map(|slot| PeerInfo {
                name: slot.name.clone(),
                side: slot.side,
                char_id: slot.char_id,
            })
            .collect()
    }

    pub fn connected_players(&self) -> u8 {
        let remote = self.slots.iter().filter(|slot| slot.is_some()).count() as u8;
        remote + if self.local_side().is_some() { 1 } else { 0 }
    }

    pub fn needed_players(&self) -> u8 {
        2
    }

    pub fn local_address(&self) -> String {
        match local_ipv4() {
            Some(ip) => format!("{ip}:{}", self.port),
            None => format!("YOUR IP:{}", self.port),
        }
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn take_notices(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notices)
    }

    pub fn shutdown(&mut self) {
        for slot in self.slots.iter().flatten() {
            send_packet(
                &self.socket,
                slot.addr,
                &NetPacket::Bye {
                    reason: "HOST CLOSED".into(),
                },
            );
        }
    }

    pub fn reset_to_lobby(&mut self, seed: u64) {
        for slot in self.slots.iter().flatten() {
            send_packet(
                &self.socket,
                slot.addr,
                &NetPacket::Bye {
                    reason: "MATCH COMPLETE".into(),
                },
            );
        }
        self.slots = [None, None];
        self.match_state = None;
        self.status = ServerStatus::Lobby;
        self.accumulator = 0.0;
        self.snapshot_accum = SNAPSHOT_INTERVAL;
        self.snapshot_seq = 0;
        self.events.clear();
        self.net_events.clear();
        self.config.seed = seed;
        self.notices.push("READY FOR A NEW MATCH".into());
    }

    pub fn update(&mut self, dt: f32, local_input: Option<InputFrame>) {
        self.poll();
        let dt = dt.clamp(0.0, 0.25);
        match self.status {
            ServerStatus::Playing => {
                self.accumulator += dt;
                let mut steps = 0;
                while self.accumulator >= NET_STEP
                    && steps < 32
                    && matches!(self.status, ServerStatus::Playing)
                {
                    self.step(local_input);
                    self.accumulator -= NET_STEP;
                    steps += 1;
                }
                if matches!(self.status, ServerStatus::Playing) {
                    self.snapshot_accum += dt;
                    if self.snapshot_accum >= SNAPSHOT_INTERVAL {
                        self.snapshot_accum = 0.0;
                        self.send_snapshot();
                    }
                } else {
                    self.send_snapshot();
                }
            }
            ServerStatus::Ended => {
                self.snapshot_accum += dt;
                if self.snapshot_accum >= 0.1 {
                    self.snapshot_accum = 0.0;
                    self.send_snapshot();
                }
            }
            ServerStatus::Lobby | ServerStatus::Aborted(_) => {}
        }
    }

    fn poll(&mut self) {
        let mut buffer = [0u8; 2048];
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    if let Some(packet) = decode(&buffer[..len]) {
                        self.handle_packet(addr, packet);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        self.expire_slots();
    }

    fn expire_slots(&mut self) {
        let now = Instant::now();
        let playing = matches!(self.status, ServerStatus::Playing | ServerStatus::Ended);
        let abort_match = matches!(self.status, ServerStatus::Playing);
        let mut lost = None;
        for side in 0..2 {
            let Some(slot) = &self.slots[side] else {
                continue;
            };
            let limit = if playing {
                Duration::from_secs(4)
            } else {
                Duration::from_secs(8)
            };
            if now.duration_since(slot.last_heard) > limit {
                lost = Some((side, slot.name.clone()));
                break;
            }
        }
        if let Some((side, name)) = lost {
            self.slots[side] = None;
            self.notices.push(format!("{name} DISCONNECTED"));
            if abort_match {
                self.abort(format!("{name} DISCONNECTED"));
            }
        }
    }

    fn abort(&mut self, reason: String) {
        self.status = ServerStatus::Aborted(reason.clone());
        for slot in self.slots.iter().flatten() {
            send_packet(
                &self.socket,
                slot.addr,
                &NetPacket::Bye {
                    reason: reason.clone(),
                },
            );
        }
        self.slots = [None, None];
        self.notices.push(reason);
    }

    fn handle_packet(&mut self, addr: SocketAddr, packet: NetPacket) {
        match packet {
            NetPacket::Discover { version } => {
                if version == PROTOCOL_VERSION {
                    self.send_offer(addr);
                }
            }
            NetPacket::Hello {
                version,
                name,
                char_id,
            } => self.handle_hello(addr, version, name, char_id),
            NetPacket::Input {
                side,
                newest_seq,
                frames,
            } => self.handle_input(addr, side as usize, newest_seq, &frames),
            NetPacket::Ping { stamp } => {
                if let Some(slot) = self.find_slot_mut(addr) {
                    slot.last_heard = Instant::now();
                }
                send_packet(&self.socket, addr, &NetPacket::Pong { stamp });
            }
            NetPacket::Bye { .. } => {
                if let Some((side, name)) = self.remove_slot(addr) {
                    self.notices.push(format!("{name} LEFT"));
                    if matches!(self.status, ServerStatus::Playing) {
                        self.abort(format!("{name} LEFT"));
                    }
                    let _ = side;
                }
            }
            _ => {}
        }
    }

    fn send_offer(&self, addr: SocketAddr) {
        let offer = NetPacket::Offer {
            version: PROTOCOL_VERSION,
            name: self.name.clone(),
            port: self.port,
            sport: self.config.sport,
            play_to: self.config.play_to as u8,
            players: self.connected_players(),
            max_players: 2,
            in_match: matches!(self.status, ServerStatus::Playing | ServerStatus::Ended),
        };
        send_packet(&self.socket, addr, &offer);
    }

    fn handle_hello(&mut self, addr: SocketAddr, version: u16, name: String, char_id: u8) {
        if version != PROTOCOL_VERSION {
            send_packet(
                &self.socket,
                addr,
                &NetPacket::Reject {
                    reason: "PROTOCOL MISMATCH".into(),
                },
            );
            return;
        }
        if let Some(side) = self
            .slots
            .iter()
            .flatten()
            .find(|slot| slot.addr == addr)
            .map(|slot| slot.side)
        {
            self.send_welcome(addr, side);
            return;
        }
        if !matches!(self.status, ServerStatus::Lobby) {
            send_packet(
                &self.socket,
                addr,
                &NetPacket::Reject {
                    reason: "MATCH IN PROGRESS".into(),
                },
            );
            return;
        }
        let side = match self.mode {
            ServerMode::Hosted => {
                if self.slots[1].is_some() {
                    None
                } else {
                    Some(1)
                }
            }
            ServerMode::Dedicated => {
                if self.slots[0].is_none() {
                    Some(0)
                } else if self.slots[1].is_none() {
                    Some(1)
                } else {
                    None
                }
            }
        };
        let Some(side) = side else {
            send_packet(
                &self.socket,
                addr,
                &NetPacket::Reject {
                    reason: "SERVER IS FULL".into(),
                },
            );
            return;
        };
        let name = sanitize_name(&name);
        let char_id = (char_id as usize).min(CHARACTERS.len() - 1);
        self.slots[side] = Some(ClientSlot::new(addr, side, name.clone(), char_id));
        match side {
            0 => self.config.p1 = char_id,
            _ => self.config.p2 = char_id,
        }
        self.notices
            .push(format!("{name} JOINED AS {}", super::side_label(side)));
        self.send_welcome(addr, side);
        self.broadcast_welcome();
        if self.ready() {
            self.start_match();
        }
    }

    fn send_welcome(&self, addr: SocketAddr, side: usize) {
        let welcome = NetPacket::Welcome {
            version: PROTOCOL_VERSION,
            side: side as u8,
            players: self.connected_players(),
            needed: self.needed_players(),
            name: self.name.clone(),
        };
        send_packet(&self.socket, addr, &welcome);
    }

    fn broadcast_welcome(&self) {
        for slot in self.slots.iter().flatten() {
            self.send_welcome(slot.addr, slot.side);
        }
    }

    fn ready(&self) -> bool {
        match self.mode {
            ServerMode::Hosted => self.slots[1].is_some(),
            ServerMode::Dedicated => self.slots[0].is_some() && self.slots[1].is_some(),
        }
    }

    fn start_match(&mut self) {
        if !matches!(self.status, ServerStatus::Lobby) {
            return;
        }
        let mut config = self.config;
        for slot in self.slots.iter().flatten() {
            match slot.side {
                0 => config.p1 = slot.char_id,
                _ => config.p2 = slot.char_id,
            }
        }
        self.config = config;
        self.match_state = Some(Match::new(config));
        self.status = ServerStatus::Playing;
        self.accumulator = 0.0;
        self.snapshot_accum = SNAPSHOT_INTERVAL;
        self.snapshot_seq = 0;
        self.events.clear();
        self.net_events.clear();
        self.notices.push("MATCH STARTED".into());
    }

    fn handle_input(
        &mut self,
        addr: SocketAddr,
        side: usize,
        newest_seq: u32,
        frames: &[InputFrame],
    ) {
        if side > 1 || frames.is_empty() {
            return;
        }
        let Some(slot) = self.slots[side].as_mut() else {
            return;
        };
        if slot.addr != addr {
            return;
        }
        slot.last_heard = Instant::now();
        slot.last_input = Instant::now();
        let frames = &frames[frames.len().saturating_sub(INPUT_HISTORY)..];
        if !slot.has_input {
            slot.last_frame = *frames.last().unwrap();
            slot.last_seq = newest_seq;
            slot.has_input = true;
            return;
        }
        let delta = newest_seq.wrapping_sub(slot.last_seq);
        if delta == 0 || delta > 120 {
            return;
        }
        let unseen = (delta as usize).min(frames.len());
        let start = frames.len() - unseen;
        let mut prev = slot.last_frame;
        for frame in &frames[start..] {
            if frame.jump && !prev.jump {
                slot.pending_jump = true;
            }
            if frame.ability && !prev.ability {
                slot.pending_ability = true;
            }
            prev = *frame;
        }
        slot.last_frame = *frames.last().unwrap();
        slot.last_seq = newest_seq;
    }

    fn find_slot_mut(&mut self, addr: SocketAddr) -> Option<&mut ClientSlot> {
        self.slots
            .iter_mut()
            .flatten()
            .find(|slot| slot.addr == addr)
    }

    fn remove_slot(&mut self, addr: SocketAddr) -> Option<(usize, String)> {
        for side in 0..2 {
            let matches = self.slots[side]
                .as_ref()
                .map(|slot| slot.addr == addr)
                .unwrap_or(false);
            if matches {
                let slot = self.slots[side].take()?;
                return Some((slot.side, slot.name));
            }
        }
        None
    }

    fn remote_input(&mut self, side: usize) -> InputFrame {
        let Some(slot) = self.slots[side].as_mut() else {
            return InputFrame::NONE;
        };
        if slot.last_input.elapsed() > input_timeout() {
            return InputFrame::NONE;
        }
        let mut frame = slot.last_frame;
        let jump_pulse = std::mem::take(&mut slot.pending_jump);
        let ability_pulse = std::mem::take(&mut slot.pending_ability);
        if let Some(m) = self.match_state.as_mut() {
            let player = &mut m.players[side];
            if jump_pulse {
                player.prev_input.jump = false;
                frame.jump = true;
            }
            if ability_pulse {
                player.prev_input.ability = false;
                frame.ability = true;
            }
        }
        frame
    }

    fn step(&mut self, local_input: Option<InputFrame>) {
        if !matches!(self.status, ServerStatus::Playing) {
            return;
        }
        let local_side = self.local_side();
        let mut inputs = [InputFrame::NONE; 2];
        for (side, input) in inputs.iter_mut().enumerate() {
            *input = if local_side == Some(side) {
                local_input.unwrap_or(InputFrame::NONE)
            } else {
                self.remote_input(side)
            };
        }
        let Some(m) = self.match_state.as_mut() else {
            return;
        };
        m.update(NET_STEP, inputs);
        let events = m.take_events();
        for event in &events {
            if let Some(code) = event_code(event) {
                self.net_events.push(code);
            }
        }
        self.events.extend(events);
        if m.phase == Phase::Ended {
            self.status = ServerStatus::Ended;
            let score = m.score;
            self.notices
                .push(format!("MATCH OVER  P1 {} - {} P2", score[0], score[1]));
        }
    }

    fn send_snapshot(&mut self) {
        let Some(m) = &self.match_state else {
            self.net_events.clear();
            return;
        };
        if self.slots.iter().all(|slot| slot.is_none()) {
            self.net_events.clear();
            return;
        }
        let payload = SnapshotPayload {
            state: m.clone(),
            events: std::mem::take(&mut self.net_events),
        };
        let Some(data) = encode_snapshot(&payload) else {
            return;
        };
        let chunks = split_snapshot(&data);
        if chunks.is_empty() {
            return;
        }
        self.snapshot_seq = self.snapshot_seq.wrapping_add(1);
        let seq = self.snapshot_seq;
        let total = chunks.len() as u8;
        let tick = m.tick;
        for side in 0..2 {
            let Some(slot) = &self.slots[side] else {
                continue;
            };
            for (index, chunk) in chunks.iter().enumerate() {
                let packet = NetPacket::Snapshot {
                    seq,
                    tick,
                    recipient: side as u8,
                    chunk: index as u8,
                    chunks: total,
                    data: chunk.clone(),
                };
                send_packet(&self.socket, slot.addr, &packet);
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn run_headless(cli: super::cli::Cli) -> ! {
    let config = cli.match_config(super::now_seed(), 1.0);
    let mut server = match Server::new(config, ServerMode::Dedicated, cli.port, cli.name.clone()) {
        Ok(server) => server,
        Err(error) => {
            eprintln!("cannot listen on udp port {}: {error}", cli.port);
            std::process::exit(1);
        }
    };
    println!(
        "Super Slime Soccer server listening on 0.0.0.0:{} ({})",
        server.port(),
        server.config().sport.label()
    );
    println!("LAN address: {}", server.local_address());
    println!("Waiting for 2 players. Press Ctrl+C to stop.");

    let mut last = Instant::now();
    let mut last_status = server.status().clone();
    let mut ended_at: Option<Instant> = None;
    let mut aborted_at: Option<Instant> = None;
    loop {
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32().min(0.25);
        last = now;
        server.update(dt, None);
        for notice in server.take_notices() {
            println!("[net] {notice}");
        }
        let status = server.status().clone();
        if status != last_status {
            println!("[net] state: {status:?}");
            last_status = status.clone();
        }
        if matches!(status, ServerStatus::Ended) {
            let started = ended_at.get_or_insert(now);
            if now.duration_since(*started) > Duration::from_secs(6) {
                server.reset_to_lobby(super::now_seed());
                ended_at = None;
                println!("[net] ready for the next match");
            }
        } else {
            ended_at = None;
        }
        if matches!(status, ServerStatus::Aborted(_)) {
            let started = aborted_at.get_or_insert(now);
            if now.duration_since(*started) > Duration::from_secs(3) {
                server.reset_to_lobby(super::now_seed());
                aborted_at = None;
                println!("[net] ready for the next match");
            }
        } else {
            aborted_at = None;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
