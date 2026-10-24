use slime_soccer::game::config::{Difficulty, ManaMode, Sport};
use slime_soccer::game::sim::{MatchConfig, Stats};
use slime_soccer::save::SaveData;
use slime_soccer::ui::{ChampionshipRun, FrontEnd, LaunchMode, LaunchRequest, MatchReport, Screen};

fn report(mode: LaunchMode, winner: usize, score: [u32; 2]) -> MatchReport {
    MatchReport {
        request: LaunchRequest {
            config: MatchConfig {
                sport: Sport::Soccer,
                arena: 2,
                p1: 3,
                p2: 8,
                play_to: 3,
                mana_mode: ManaMode::Bonus,
                seed: 42,
                p1_control: 1.0,
                p2_control: 1.0,
            },
            ai: true,
            difficulty: Difficulty::Normal,
            mode,
            local_side: 0,
        },
        score,
        winner,
        stats: Stats::default(),
        elapsed: 33.0,
    }
}

#[test]
fn championship_advances_and_awards_a_cup_after_three_wins() {
    let mut save = SaveData::default();
    let mut front = FrontEnd::new(&save);
    front.championship = Some(ChampionshipRun {
        round: 0,
        sport: Sport::Soccer,
        arena: 2,
        player: 3,
        opponents: [8, 9, 10],
        difficulty: Difficulty::Normal,
        scores: Vec::new(),
        done: false,
        won: false,
    });

    front.on_match_finished(report(LaunchMode::Championship, 0, [3, 1]), &mut save);
    assert_eq!(front.screen, Screen::Results);
    assert_eq!(save.progress.matches, 1);
    assert_eq!(save.progress.wins, 1);
    assert_eq!(save.progress.championships, 0);
    {
        let run = front.championship.as_ref().unwrap();
        assert_eq!(run.round, 1);
        assert_eq!(run.scores, vec![[3, 1]]);
        assert!(!run.done);
    }

    front.on_match_finished(report(LaunchMode::Championship, 0, [3, 2]), &mut save);
    {
        let run = front.championship.as_ref().unwrap();
        assert_eq!(run.round, 2);
        assert!(!run.done);
        assert_eq!(save.progress.matches, 2);
        assert_eq!(save.progress.wins, 2);
    }

    front.on_match_finished(report(LaunchMode::Championship, 0, [3, 0]), &mut save);
    {
        let run = front.championship.as_ref().unwrap();
        assert!(run.done);
        assert!(run.won);
        assert_eq!(run.scores.len(), 3);
    }
    assert_eq!(save.progress.championships, 1);
    assert_eq!(save.progress.matches, 3);
    assert_eq!(save.progress.wins, 3);
    assert!(save
        .progress
        .trophies
        .iter()
        .any(|trophy| trophy == "SOCCER CUP"));
    assert!(front.dirty);
}

#[test]
fn championship_loss_ends_the_run_without_a_cup() {
    let mut save = SaveData::default();
    let mut front = FrontEnd::new(&save);
    front.championship = Some(ChampionshipRun {
        round: 1,
        sport: Sport::Soccer,
        arena: 2,
        player: 3,
        opponents: [8, 9, 10],
        difficulty: Difficulty::Normal,
        scores: vec![[2, 3]],
        done: false,
        won: false,
    });

    front.on_match_finished(report(LaunchMode::Championship, 1, [1, 3]), &mut save);
    let run = front.championship.as_ref().unwrap();
    assert!(run.done);
    assert!(!run.won);
    assert_eq!(run.scores, vec![[2, 3], [1, 3]]);
    assert_eq!(save.progress.championships, 0);
    assert_eq!(save.progress.wins, 0);
    assert_eq!(save.progress.matches, 1);
}

#[test]
fn online_results_credit_the_local_side() {
    let mut save = SaveData::default();
    let mut front = FrontEnd::new(&save);
    let mut win = report(LaunchMode::Online, 1, [2, 5]);
    win.request.local_side = 1;
    win.request.ai = false;

    front.on_match_finished(win, &mut save);
    assert_eq!(front.screen, Screen::Results);
    assert_eq!(save.progress.matches, 1);
    assert_eq!(save.progress.wins, 1);
    assert_eq!(save.progress.goals, 5);
    assert!(front.championship.is_none());

    let mut loss = report(LaunchMode::Online, 0, [6, 1]);
    loss.request.local_side = 1;
    loss.request.ai = false;
    front.on_match_finished(loss, &mut save);
    assert_eq!(save.progress.matches, 2);
    assert_eq!(save.progress.wins, 1);
    assert_eq!(save.progress.goals, 6);
}

#[test]
fn casual_results_are_remembered_for_the_next_setup() {
    let mut save = SaveData::default();
    let mut front = FrontEnd::new(&save);
    let mut result = report(LaunchMode::Casual, 1, [2, 4]);
    result.request.config.sport = Sport::Volleyball;
    result.request.config.arena = 6;
    result.request.config.p1 = 15;
    result.request.config.p2 = 21;
    result.request.config.play_to = 7;
    result.request.config.mana_mode = ManaMode::Round;
    result.request.difficulty = Difficulty::Hard;

    front.on_match_finished(result, &mut save);

    assert_eq!(front.screen, Screen::Results);
    assert_eq!(save.progress.matches, 1);
    assert_eq!(save.progress.wins, 0);
    assert_eq!(save.progress.goals, 2);
    assert_eq!(save.progress.last_sport, 1);
    assert_eq!(save.progress.last_arena, 6);
    assert_eq!(save.progress.last_p1, 15);
    assert_eq!(save.progress.last_p2, 21);
    assert_eq!(save.progress.last_play_to, 7);
    assert_eq!(save.progress.last_mana_mode, 2);
    assert_eq!(save.progress.last_difficulty, 2);
    assert!(front.championship.is_none());
}
