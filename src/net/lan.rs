use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use crate::game::config::Sport;

use super::{decode, send_packet, NetPacket, DEFAULT_PORT, PROTOCOL_VERSION};

#[derive(Clone, Debug)]
pub struct LanOffer {
    pub name: String,
    pub addr: SocketAddr,
    pub sport: Sport,
    pub play_to: u8,
    pub players: u8,
    pub max_players: u8,
    pub in_match: bool,
    pub seen: Instant,
}

impl LanOffer {
    pub fn open_slots(&self) -> u8 {
        self.max_players.saturating_sub(self.players)
    }
}

pub struct LanBrowser {
    socket: UdpSocket,
    last_discover: Option<Instant>,
    offers: Vec<LanOffer>,
}

impl LanBrowser {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.set_nonblocking(true)?;
        socket.set_broadcast(true)?;
        Ok(Self {
            socket,
            last_discover: None,
            offers: Vec::new(),
        })
    }

    pub fn offers(&self) -> &[LanOffer] {
        &self.offers
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        if self
            .last_discover
            .map(|sent| now.duration_since(sent) > Duration::from_millis(750))
            .unwrap_or(true)
        {
            let discover = NetPacket::Discover {
                version: PROTOCOL_VERSION,
            };
            send_packet(
                &self.socket,
                SocketAddr::from((Ipv4Addr::BROADCAST, DEFAULT_PORT)),
                &discover,
            );
            send_packet(
                &self.socket,
                SocketAddr::from((Ipv4Addr::LOCALHOST, DEFAULT_PORT)),
                &discover,
            );
            self.last_discover = Some(now);
        }

        let mut buffer = [0u8; 2048];
        loop {
            match self.socket.recv_from(&mut buffer) {
                Ok((len, addr)) => {
                    let Some(NetPacket::Offer {
                        version,
                        name,
                        port: _,
                        sport,
                        play_to,
                        players,
                        max_players,
                        in_match,
                    }) = decode(&buffer[..len])
                    else {
                        continue;
                    };
                    if version != PROTOCOL_VERSION {
                        continue;
                    }
                    if let Some(offer) = self.offers.iter_mut().find(|offer| offer.addr == addr) {
                        offer.name = name;
                        offer.sport = sport;
                        offer.play_to = play_to;
                        offer.players = players;
                        offer.max_players = max_players;
                        offer.in_match = in_match;
                        offer.seen = now;
                    } else {
                        self.offers.push(LanOffer {
                            name,
                            addr,
                            sport,
                            play_to,
                            players,
                            max_players,
                            in_match,
                            seen: now,
                        });
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        self.offers
            .retain(|offer| now.duration_since(offer.seen) < Duration::from_secs(4));
        self.offers.sort_by(|a, b| a.name.cmp(&b.name));
    }
}
