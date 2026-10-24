use crate::game::arenas::ARENAS;
use crate::game::characters::CHARACTERS;
use crate::game::config::{ManaMode, Sport};
use crate::game::sim::MatchConfig;
use crate::net::DEFAULT_PORT;

#[derive(Clone, Debug)]
pub struct Cli {
    pub server: bool,
    pub host: bool,
    pub connect: Option<String>,
    pub port: u16,
    pub sport: Sport,
    pub arena: usize,
    pub play_to: u32,
    pub mana_mode: ManaMode,
    pub p1: usize,
    pub p2: usize,
    pub name: String,
    pub help: bool,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            server: false,
            host: false,
            connect: None,
            port: DEFAULT_PORT,
            sport: Sport::Soccer,
            arena: 0,
            play_to: 6,
            mana_mode: ManaMode::Bonus,
            p1: 0,
            p2: 4,
            name: "SUPER SLIME".into(),
            help: false,
        }
    }
}

impl Cli {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut cli = Self::default();
        let mut index = 0;
        while index < args.len() {
            let arg = args[index].as_str();
            match arg {
                "--server" => cli.server = true,
                "--host" => cli.host = true,
                "--connect" | "-c" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--connect needs an address".to_string())?;
                    cli.connect = Some(value.clone());
                }
                "--port" | "-p" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--port needs a number".to_string())?;
                    cli.port = value
                        .parse::<u16>()
                        .map_err(|_| format!("invalid port: {value}"))?;
                    if cli.port == 0 {
                        return Err("port must be between 1 and 65535".into());
                    }
                }
                "--sport" => {
                    index += 1;
                    let value = args.get(index).ok_or_else(|| {
                        "--sport needs soccer, volleyball or basketball".to_string()
                    })?;
                    cli.sport = match value.to_ascii_lowercase().as_str() {
                        "soccer" | "football" => Sport::Soccer,
                        "volleyball" | "volley" => Sport::Volleyball,
                        "basketball" | "basket" => Sport::Basketball,
                        _ => return Err(format!("unknown sport: {value}")),
                    };
                }
                "--arena" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--arena needs a number".to_string())?;
                    cli.arena = value
                        .parse::<usize>()
                        .map_err(|_| format!("invalid arena: {value}"))?;
                    cli.arena = cli.arena.min(ARENAS.len() - 1);
                }
                "--play-to" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--play-to needs a number".to_string())?;
                    cli.play_to = value
                        .parse::<u32>()
                        .map_err(|_| format!("invalid target score: {value}"))?
                        .clamp(1, 15);
                }
                "--mana" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--mana needs bonus, offensive or round".to_string())?;
                    cli.mana_mode = match value.to_ascii_lowercase().as_str() {
                        "bonus" => ManaMode::Bonus,
                        "offensive" => ManaMode::Offensive,
                        "round" => ManaMode::Round,
                        _ => return Err(format!("unknown mana mode: {value}")),
                    };
                }
                "--p1" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--p1 needs a slime index".to_string())?;
                    cli.p1 = parse_char(value)?;
                }
                "--p2" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--p2 needs a slime index".to_string())?;
                    cli.p2 = parse_char(value)?;
                }
                "--name" | "-n" => {
                    index += 1;
                    let value = args
                        .get(index)
                        .ok_or_else(|| "--name needs text".to_string())?;
                    cli.name = value.clone();
                }
                "--help" | "-h" => cli.help = true,
                _ => return Err(format!("unknown argument: {arg}")),
            }
            index += 1;
        }
        if cli.server && cli.host {
            return Err("--server and --host cannot be used together".into());
        }
        if cli.server && cli.connect.is_some() {
            return Err("--server and --connect cannot be used together".into());
        }
        if cli.host && cli.connect.is_some() {
            return Err("--host and --connect cannot be used together".into());
        }
        Ok(cli)
    }

    pub fn match_config(&self, seed: u64, control: f32) -> MatchConfig {
        MatchConfig {
            sport: self.sport,
            arena: self.arena.min(ARENAS.len() - 1),
            p1: self.p1.min(CHARACTERS.len() - 1),
            p2: self.p2.min(CHARACTERS.len() - 1),
            play_to: self.play_to.clamp(1, 15),
            mana_mode: self.mana_mode,
            seed,
            p1_control: control,
            p2_control: control,
        }
    }
}

fn parse_char(value: &str) -> Result<usize, String> {
    let index: usize = value
        .parse()
        .map_err(|_| format!("invalid slime index: {value}"))?;
    if index >= CHARACTERS.len() {
        return Err(format!(
            "slime index must be between 0 and {}",
            CHARACTERS.len() - 1
        ));
    }
    Ok(index)
}

pub fn usage() -> &'static str {
    "Super Slime Soccer

USAGE:
    slime-soccer                     Open the game menu
    slime-soccer --host              Host an online match
    slime-soccer --connect ADDRESS   Join a match by IP or hostname
    slime-soccer --server            Run a headless two-player server

OPTIONS:
    --port PORT          UDP port (default 43210)
    --sport SPORT        soccer, volleyball or basketball
    --arena NUMBER       0..10
    --play-to NUMBER     1..15
    --mana MODE          bonus, offensive or round
    --p1 NUMBER          slime for the host or for --connect (0..34)
    --p2 NUMBER          initial rival slime, replaced when a client joins (0..34)
    --name TEXT          Player or server name
    -h, --help           Show this help

The host/server runs the authoritative simulation. Clients can use
LAN GAMES to discover servers, or join with a direct IP address."
}
