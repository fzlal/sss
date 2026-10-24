use std::time::Duration;

use slime_soccer::game::config::{InputFrame, ManaMode, Sport};
use slime_soccer::game::sim::MatchConfig;
use slime_soccer::net::{Client, ClientStatus, Server, ServerMode, ServerStatus};

fn test_config() -> MatchConfig {
    MatchConfig {
        sport: Sport::Soccer,
        arena: 0,
        p1: 0,
        p2: 7,
        play_to: 3,
        mana_mode: ManaMode::Bonus,
        seed: 12345,
        p1_control: 1.0,
        p2_control: 1.0,
    }
}

#[test]
fn host_and_client_exchange_snapshots_and_input() {
    let mut server = Server::new(test_config(), ServerMode::Hosted, 0, "TEST HOST".into()).unwrap();
    let addr = format!("127.0.0.1:{}", server.port());
    let mut client = Client::connect(&addr, "GUEST", 7).unwrap();

    let right = InputFrame {
        right: true,
        ..InputFrame::NONE
    };
    let mut playing = false;
    let mut start_x = 0.0;
    let mut last_tick = 0;
    for _ in 0..1000 {
        server.update(0.01, Some(InputFrame::NONE));
        client.update(0.01, right);
        if !playing {
            if let (ClientStatus::Playing, Some(state)) = (client.status(), client.latest()) {
                playing = true;
                start_x = state.players[1].x;
            }
        } else if let Some(state) = client.latest() {
            last_tick = state.tick;
            if last_tick > 40 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    assert!(playing, "client never entered play");
    assert_eq!(client.side(), Some(1));
    assert_eq!(server.connected_players(), 2);
    assert!(matches!(server.status(), ServerStatus::Playing));
    let view = client.view().expect("client has no snapshot view");
    assert!(
        view.players[1].x > start_x + 4.0,
        "remote input did not move P2: {start_x} -> {}",
        view.players[1].x
    );
    assert!(last_tick > 40, "simulation did not advance");
    assert!(client.latest().unwrap().tick > 0);

    client.quit();
    for _ in 0..80 {
        server.update(0.01, Some(InputFrame::NONE));
        if matches!(server.status(), ServerStatus::Aborted(_)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        matches!(server.status(), ServerStatus::Aborted(_)),
        "host did not notice the client leaving: {:?}",
        server.status()
    );
}
