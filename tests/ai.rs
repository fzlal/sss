use slime_soccer::game::ai::AIController;
use slime_soccer::game::config::*;
use slime_soccer::game::sim::{Match, MatchConfig};

const STEP: f32 = 1.0 / 120.0;

fn idle_vs_ai(sport: Sport, difficulty: Difficulty, seed: u64, secs: f32) -> Match {
    let mut m = Match::new(MatchConfig {
        sport,
        arena: 0,
        p1: 3,
        p2: 0,
        play_to: 99,
        mana_mode: ManaMode::Bonus,
        seed,
        p1_control: 1.0,
        p2_control: 1.0,
    });
    let mut ai = AIController::new(difficulty, 1, seed ^ 0x9e37_79b9);
    for _ in 0..(120.0 * secs) as usize {
        let input = ai.update(&m, STEP);
        m.update(STEP, [InputFrame::NONE, input]);
        m.take_events();
    }
    m
}

fn ai_vs_ai(sport: Sport, left: Difficulty, right: Difficulty, seed: u64, secs: f32) -> Match {
    let mut m = Match::new(MatchConfig {
        sport,
        arena: 0,
        p1: 0,
        p2: 0,
        play_to: 99,
        mana_mode: ManaMode::Bonus,
        seed,
        p1_control: 1.0,
        p2_control: 1.0,
    });
    let mut a = AIController::new(left, 0, seed ^ 0x1234_5678);
    let mut b = AIController::new(right, 1, seed ^ 0x9e37_79b9);
    for _ in 0..(120.0 * secs) as usize {
        let i1 = a.update(&m, STEP);
        let i2 = b.update(&m, STEP);
        m.update(STEP, [i1, i2]);
        m.take_events();
    }
    m
}

#[test]
fn hard_ai_scores_against_a_still_opponent_in_every_sport() {
    for sport in Sport::ALL {
        let secs = if sport == Sport::Basketball {
            120.0
        } else {
            45.0
        };
        let m = idle_vs_ai(sport, Difficulty::Hard, 424_242, secs);
        assert!(
            m.score[1] > m.score[0],
            "{sport:?}: AI did not win against an idle opponent, score={:?}",
            m.score
        );
        assert!(m.stats.hits[1] > 0, "{sport:?}: AI never touched the ball");
    }
}

#[test]
fn basketball_ai_stays_engaged_over_a_long_match() {
    for seed in [424_242u64, 777, 55] {
        let m = idle_vs_ai(Sport::Basketball, Difficulty::Hard, seed, 120.0);
        assert!(
            m.stats.hits[1] >= 40,
            "seed {seed}: basketball AI only made {} contacts",
            m.stats.hits[1]
        );
        assert!(
            m.score[1] > m.score[0],
            "seed {seed}: basketball AI lost to an idle opponent, score={:?}",
            m.score
        );
    }
}

#[test]
fn volleyball_hard_ai_beats_easy_ai() {
    for seed in [777u64, 424_242, 55] {
        let m = ai_vs_ai(
            Sport::Volleyball,
            Difficulty::Hard,
            Difficulty::Easy,
            seed,
            60.0,
        );
        assert!(
            m.score[0] > m.score[1],
            "seed {seed}: Hard should beat Easy, score={:?}",
            m.score
        );
    }
}

#[test]
fn volleyball_rallies_do_not_stall() {
    let m = ai_vs_ai(
        Sport::Volleyball,
        Difficulty::Hard,
        Difficulty::Hard,
        777,
        60.0,
    );
    assert!(
        m.stats.hits[0] + m.stats.hits[1] >= 20,
        "hits={:?}",
        m.stats.hits
    );
    assert!(m.score[0] + m.score[1] >= 6, "score={:?}", m.score);
}
