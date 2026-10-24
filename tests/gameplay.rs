use slime_soccer::game::abilities;
use slime_soccer::game::ai::AIController;
use slime_soccer::game::characters::{AbilityKind, ProjKind, CHARACTERS};
use slime_soccer::game::config::*;
use slime_soccer::game::sim::*;

const STEP: f32 = 1.0 / 120.0;

fn config(sport: Sport) -> MatchConfig {
    MatchConfig {
        sport,
        arena: 0,
        p1: 0,
        p2: 4,
        play_to: 9,
        mana_mode: ManaMode::Bonus,
        seed: 0x5eed_1234,
        p1_control: 1.0,
        p2_control: 1.0,
    }
}

fn playing(sport: Sport) -> Match {
    let mut m = Match::new(config(sport));
    m.phase = Phase::Playing;
    m.phase_timer = 0.0;
    m
}

fn run(m: &mut Match, ticks: usize) {
    for _ in 0..ticks {
        m.update(STEP, [InputFrame::NONE; 2]);
    }
}

fn finite_match(m: &Match) -> bool {
    m.ball.x.is_finite()
        && m.ball.y.is_finite()
        && m.ball.vx.is_finite()
        && m.ball.vy.is_finite()
        && m.players
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite() && p.vx.is_finite() && p.vy.is_finite())
}

#[test]
fn countdown_hands_over_to_play() {
    let mut m = Match::new(config(Sport::Soccer));
    assert_eq!(m.phase, Phase::Countdown);
    let mut saw_go = false;
    for _ in 0..((COUNTDOWN * 120.0) as usize + 10) {
        m.update(STEP, [InputFrame::NONE; 2]);
        if m.take_events()
            .iter()
            .any(|event| matches!(event, Event::Go))
        {
            saw_go = true;
        }
    }
    assert_eq!(m.phase, Phase::Playing);
    assert!(saw_go);
}

#[test]
fn soccer_goal_is_awarded_to_the_attacking_side() {
    let mut m = playing(Sport::Soccer);
    m.ball.x = W - GOAL_DEPTH + 22.0;
    m.ball.y = GROUND_Y - 40.0;
    m.ball.prev_y = m.ball.y;
    m.ball.vx = 140.0;
    m.ball.vy = 0.0;

    for _ in 0..30 {
        m.update(STEP, [InputFrame::NONE; 2]);
        if m.phase == Phase::Point {
            break;
        }
    }

    assert_eq!(m.score, [1, 0]);
    assert_eq!(m.stats.goals, [1, 0]);
    assert_eq!(m.phase, Phase::Point);
    assert!(m.last_score.is_some());
}

#[test]
fn own_goal_awards_the_other_side() {
    let mut m = playing(Sport::Soccer);
    m.ball.x = GOAL_DEPTH - 12.0;
    m.ball.y = GROUND_Y - 40.0;
    m.ball.prev_y = m.ball.y;
    m.ball.vx = -140.0;
    m.ball.last_toucher = Some(0);

    for _ in 0..30 {
        m.update(STEP, [InputFrame::NONE; 2]);
        if m.phase == Phase::Point {
            break;
        }
    }

    assert_eq!(m.score, [0, 1]);
    assert!(m.last_score.as_ref().is_some_and(|s| s.own_goal));
}

#[test]
fn crossbar_deflects_a_high_shot_and_emits_post_event() {
    let mut m = playing(Sport::Soccer);
    m.ball.x = LEFT_GOAL + 24.0;
    m.ball.y = GOAL_TOP - 2.0;
    m.ball.prev_y = m.ball.y;
    m.ball.vx = -360.0;
    m.ball.vy = 0.0;
    let mut saw_post = false;
    for _ in 0..24 {
        m.update(STEP, [InputFrame::NONE; 2]);
        saw_post |= m
            .take_events()
            .iter()
            .any(|event| matches!(event, Event::Bounce { kind: "post", .. }));
    }
    assert_eq!(m.score, [0, 0], "crossbar contact must not be a goal");
    assert!(saw_post, "crossbar contact should emit a post bounce event");
    assert!(
        m.ball.vx > 0.0,
        "ball should rebound out of the goal, vx={}",
        m.ball.vx
    );
}

#[test]
fn volleyball_floor_scores_the_opposite_side() {
    let mut left = playing(Sport::Volleyball);
    left.ball.x = 120.0;
    left.ball.y = GROUND_Y - BALL_RADIUS - 1.0;
    left.ball.vx = 0.0;
    left.ball.vy = 90.0;
    run(&mut left, 12);
    assert_eq!(left.score, [0, 1], "ball on P1 floor should score for P2");

    let mut right = playing(Sport::Volleyball);
    right.ball.x = W - 120.0;
    right.ball.y = GROUND_Y - BALL_RADIUS - 1.0;
    right.ball.vx = 0.0;
    right.ball.vy = 90.0;
    run(&mut right, 12);
    assert_eq!(right.score, [1, 0], "ball on P2 floor should score for P1");
}

#[test]
fn volleyball_net_stops_the_ball() {
    let mut m = playing(Sport::Volleyball);
    m.ball.x = NET_X - 22.0;
    m.ball.y = NET_TOP + 34.0;
    m.ball.vx = 420.0;
    m.ball.vy = 0.0;
    let mut saw_net = false;
    for _ in 0..30 {
        m.update(STEP, [InputFrame::NONE; 2]);
        saw_net |= m
            .take_events()
            .iter()
            .any(|event| matches!(event, Event::Bounce { kind: "net", .. }));
    }
    assert!(m.ball.x < NET_X, "ball crossed the net: {}", m.ball.x);
    assert!(m.ball.vx < 0.0, "ball should bounce back, vx={}", m.ball.vx);
    assert!(saw_net, "net contact should emit a bounce event");
}

#[test]
fn basketball_hoops_score_for_the_attacking_side() {
    let mut right = playing(Sport::Basketball);
    right.ball.x = HOOP_RIGHT_X;
    right.ball.y = HOOP_Y - 12.0;
    right.ball.vx = 0.0;
    right.ball.vy = 260.0;
    run(&mut right, 120);
    assert_eq!(right.score, [1, 0], "right hoop should score for P1");

    let mut left = playing(Sport::Basketball);
    left.ball.x = HOOP_LEFT_X;
    left.ball.y = HOOP_Y - 12.0;
    left.ball.vx = 0.0;
    left.ball.vy = 260.0;
    run(&mut left, 120);
    assert_eq!(left.score, [0, 1], "left hoop should score for P2");
}

#[test]
fn jump_reaches_apex_and_lands_without_auto_repeat() {
    let mut m = playing(Sport::Soccer);
    let ground_y = m.players[0].y;
    m.update(
        STEP,
        [
            InputFrame {
                jump: true,
                ..InputFrame::NONE
            },
            InputFrame::NONE,
        ],
    );
    assert!(!m.players[0].on_ground);
    let mut min_y = m.players[0].y;
    for _ in 0..180 {
        m.update(
            STEP,
            [
                InputFrame {
                    jump: true,
                    ..InputFrame::NONE
                },
                InputFrame::NONE,
            ],
        );
        min_y = min_y.min(m.players[0].y);
    }
    assert!(min_y < ground_y - 35.0, "jump too small: {min_y}");
    assert!(m.players[0].on_ground);
    assert!((m.players[0].y - ground_y).abs() < 0.6);
}

#[test]
fn running_into_the_ball_kicks_it_forward() {
    let mut m = playing(Sport::Soccer);
    {
        let p = &mut m.players[0];
        p.x = 190.0;
        p.y = GROUND_Y - p.h / 2.0;
        p.vx = 0.0;
        p.vy = 0.0;
    }
    m.ball.x = 190.0 + PLAYER_W / 2.0 + BALL_RADIUS + 3.0;
    m.ball.y = GROUND_Y - 22.0;
    m.ball.prev_y = m.ball.y;
    m.ball.vx = 0.0;
    m.ball.vy = 0.0;
    let mut saw_kick = false;
    for _ in 0..45 {
        m.update(
            STEP,
            [
                InputFrame {
                    right: true,
                    ..InputFrame::NONE
                },
                InputFrame::NONE,
            ],
        );
        saw_kick |= m
            .take_events()
            .iter()
            .any(|event| matches!(event, Event::Kick { side: 0, .. }));
    }
    assert!(saw_kick, "no kick event");
    assert!(m.stats.hits[0] >= 1);
    assert!(m.ball.vx > 20.0, "ball vx={}", m.ball.vx);
}

#[test]
fn players_stay_inside_the_pitch() {
    for sport in Sport::ALL {
        let mut m = playing(sport);
        m.players[0].x = 30.0;
        for _ in 0..600 {
            m.update(
                STEP,
                [
                    InputFrame {
                        right: true,
                        ..InputFrame::NONE
                    },
                    InputFrame {
                        left: true,
                        ..InputFrame::NONE
                    },
                ],
            );
        }
        let half = m.players[0].w / 2.0;
        let (min_x, max_x) = match sport {
            Sport::Volleyball => (half + 6.0, NET_X - half - 7.0),
            Sport::Soccer => (GOAL_DEPTH + half, W - GOAL_DEPTH - half),
            Sport::Basketball => (half + 10.0, W - half - 10.0),
        };
        assert!(m.players[0].x >= min_x - 0.01, "{sport:?} left bound");
        assert!(m.players[0].x <= max_x + 0.01, "{sport:?} right bound");
        assert!(m.players[1].x >= m.players[1].w / 2.0);
        assert!(m.players[1].x <= W - m.players[1].w / 2.0);
        assert!(finite_match(&m));
    }
}

#[test]
fn mana_fills_faster_in_the_opponent_half() {
    fn mana_after(mode: ManaMode, x: f32) -> f32 {
        let mut m = Match::new(MatchConfig {
            mana_mode: mode,
            ..config(Sport::Soccer)
        });
        m.phase = Phase::Playing;
        m.players[0].x = x;
        m.players[0].mana = 0.0;
        m.ball.x = W / 2.0;
        m.ball.y = 100.0;
        run(&mut m, 120);
        m.players[0].mana
    }

    let bonus_own = mana_after(ManaMode::Bonus, W * 0.25);
    let bonus_away = mana_after(ManaMode::Bonus, W * 0.75);
    let offensive_own = mana_after(ManaMode::Offensive, W * 0.25);
    let offensive_away = mana_after(ManaMode::Offensive, W * 0.75);
    let round_own = mana_after(ManaMode::Round, W * 0.25);
    let round_away = mana_after(ManaMode::Round, W * 0.75);

    assert!((bonus_own - 6.0).abs() < 0.3, "bonus own {bonus_own}");
    assert!((bonus_away - 18.0).abs() < 0.3, "bonus away {bonus_away}");
    assert!(offensive_own.abs() < 0.01, "offensive own {offensive_own}");
    assert!(
        (offensive_away - 11.0).abs() < 0.3,
        "offensive away {offensive_away}"
    );
    assert!((round_own - 3.0).abs() < 0.3, "round own {round_own}");
    assert!((round_away - 9.0).abs() < 0.3, "round away {round_away}");
}

#[test]
fn round_mana_refills_after_a_point_and_bonus_does_not() {
    for (mode, expect_refill) in [(ManaMode::Round, true), (ManaMode::Bonus, false)] {
        let mut m = Match::new(MatchConfig {
            mana_mode: mode,
            ..config(Sport::Soccer)
        });
        m.phase = Phase::Playing;
        m.players[0].mana = 17.0;
        m.award_point(0, false);
        assert_eq!(m.phase, Phase::Point);
        for _ in 0..300 {
            m.update(STEP, [InputFrame::NONE; 2]);
            if m.phase == Phase::Countdown {
                break;
            }
        }
        assert_eq!(m.phase, Phase::Countdown);
        if expect_refill {
            assert!((m.players[0].mana - MANA_MAX).abs() < 0.01);
        } else {
            assert!((m.players[0].mana - 17.0).abs() < 0.01);
        }
    }
}

#[test]
fn every_character_can_activate_its_ability() {
    for (index, character) in CHARACTERS.iter().enumerate() {
        let mut m = Match::new(MatchConfig {
            p1: index,
            p2: (index + 1) % CHARACTERS.len(),
            ..config(Sport::Soccer)
        });
        m.phase = Phase::Playing;
        {
            let p = &mut m.players[0];
            p.x = W / 2.0 - 60.0;
            p.y = GROUND_Y - p.h / 2.0;
            p.vx = 0.0;
            p.vy = 0.0;
            p.facing = 1.0;
            p.on_ground = true;
            p.mana = MANA_MAX;
            p.cooldown = 0.0;
        }
        {
            let p = &mut m.players[1];
            p.x = W / 2.0 + 45.0;
            p.y = GROUND_Y - p.h / 2.0;
            p.facing = -1.0;
        }
        m.ball.x = W / 2.0;
        m.ball.y = GROUND_Y - 70.0;
        m.ball.prev_y = m.ball.y;
        m.ball.vx = 0.0;
        m.ball.vy = 0.0;

        let activated = abilities::activate(&mut m, 0);
        if matches!(character.ability.kind, AbilityKind::None) {
            assert!(!activated, "{} should have no ability", character.name);
        } else {
            assert!(activated, "{} failed to activate", character.name);
            assert_eq!(m.stats.abilities[0], 1, "{} stat", character.name);
            assert!(
                m.players[0].mana < MANA_MAX,
                "{} should spend mana",
                character.name
            );
            assert!(
                m.take_events()
                    .iter()
                    .any(|event| matches!(event, Event::Ability { side: 0, .. })),
                "{} should emit an ability event",
                character.name
            );

            for _ in 0..240 {
                m.update(STEP, [InputFrame::NONE; 2]);
            }
            assert!(
                finite_match(&m),
                "{} produced invalid physics",
                character.name
            );
        }
    }
}

#[test]
fn abilities_deny_without_mana_or_during_cooldown() {
    let index = CHARACTERS
        .iter()
        .position(|c| {
            matches!(
                c.ability.kind,
                AbilityKind::Projectile {
                    kind: ProjKind::Fireball,
                    ..
                }
            )
        })
        .expect("fireball character");

    let mut m = Match::new(MatchConfig {
        p1: index,
        ..config(Sport::Soccer)
    });
    m.phase = Phase::Playing;
    m.players[0].mana = 0.0;
    m.players[0].cooldown = 0.0;
    assert!(!abilities::activate(&mut m, 0));
    assert!(m.take_events().iter().any(|event| {
        matches!(
            event,
            Event::Deny {
                side: 0,
                reason: "mana"
            }
        )
    }));
    assert_eq!(m.stats.abilities[0], 0);

    m.players[0].mana = MANA_MAX;
    m.players[0].cooldown = 1.0;
    assert!(!abilities::activate(&mut m, 0));
    assert_eq!(m.stats.abilities[0], 0);

    m.players[0].cooldown = 0.0;
    assert!(abilities::activate(&mut m, 0));
    assert_eq!(m.stats.abilities[0], 1);
}

#[test]
fn final_point_ends_the_match_with_a_winner() {
    let mut m = Match::new(MatchConfig {
        play_to: 1,
        ..config(Sport::Soccer)
    });
    m.phase = Phase::Playing;
    m.ball.x = W - GOAL_DEPTH + 22.0;
    m.ball.y = GROUND_Y - 40.0;
    m.ball.prev_y = m.ball.y;
    m.ball.vx = 140.0;
    let mut final_whistle = false;
    for _ in 0..600 {
        m.update(STEP, [InputFrame::NONE; 2]);
        final_whistle |= m.take_events().iter().any(|event| {
            matches!(
                event,
                Event::Whistle {
                    final_whistle: true
                }
            )
        });
        if m.phase == Phase::Ended {
            break;
        }
    }
    assert_eq!(m.phase, Phase::Ended);
    assert_eq!(m.winner, Some(0));
    assert_eq!(m.score, [1, 0]);
    assert!(final_whistle);
}

#[test]
fn same_inputs_produce_identical_matches() {
    fn scripted(tick: u64, side: usize) -> InputFrame {
        let phase = tick.wrapping_add(side as u64 * 37);
        InputFrame {
            left: phase % 311 < 40,
            right: phase % 263 < 90,
            jump: phase.is_multiple_of(97),
            ability: phase.is_multiple_of(211),
        }
    }

    for sport in Sport::ALL {
        let config = MatchConfig {
            seed: 0xdead_beef,
            ..config(sport)
        };
        let mut a = Match::new(config);
        let mut b = Match::new(config);
        for tick in 0..3600u64 {
            let inputs = [scripted(tick, 0), scripted(tick, 1)];
            a.update(STEP, inputs);
            b.update(STEP, inputs);
        }
        assert_eq!(a.score, b.score, "{sport:?}");
        assert_eq!(a.phase, b.phase, "{sport:?}");
        assert_eq!(a.stats.hits, b.stats.hits, "{sport:?}");
        assert_eq!(a.tick, b.tick, "{sport:?}");
        assert_eq!(a.ball.x.to_bits(), b.ball.x.to_bits(), "{sport:?}");
        assert_eq!(a.ball.y.to_bits(), b.ball.y.to_bits(), "{sport:?}");
        for side in 0..2 {
            assert_eq!(
                a.players[side].x.to_bits(),
                b.players[side].x.to_bits(),
                "{sport:?} side {side}"
            );
            assert_eq!(
                a.players[side].y.to_bits(),
                b.players[side].y.to_bits(),
                "{sport:?} side {side}"
            );
        }
    }
}

#[test]
fn ai_matches_stay_finite_and_interact() {
    let characters = [1usize, 3, 6, 12, 17, 24, 33];
    for sport in Sport::ALL {
        for difficulty in Difficulty::ALL {
            for pair in characters.windows(2) {
                let mut m = Match::new(MatchConfig {
                    p1: pair[0],
                    p2: pair[1],
                    play_to: 3,
                    seed: 0x1234_5678,
                    ..config(sport)
                });
                let mut ai1 = AIController::new(difficulty, 0, 11);
                let mut ai2 = AIController::new(difficulty, 1, 29);
                for _ in 0..(120 * 25) {
                    let i0 = ai1.update(&m, STEP);
                    let i1 = ai2.update(&m, STEP);
                    m.update(STEP, [i0, i1]);
                }
                assert!(finite_match(&m), "{sport:?} {difficulty:?}");
                assert!(
                    m.stats.hits[0] + m.stats.hits[1] > 0,
                    "{sport:?} {difficulty:?}: AI never touched the ball"
                );
            }
        }
    }
}

#[test]
fn repeated_ai_matches_are_deterministic() {
    for sport in Sport::ALL {
        let config = MatchConfig {
            p1: 12,
            p2: 24,
            play_to: 2,
            seed: 777,
            ..config(sport)
        };
        let mut a = Match::new(config);
        let mut b = Match::new(config);
        let mut a1 = AIController::new(Difficulty::Hard, 0, 99);
        let mut a2 = AIController::new(Difficulty::Hard, 1, 123);
        let mut b1 = AIController::new(Difficulty::Hard, 0, 99);
        let mut b2 = AIController::new(Difficulty::Hard, 1, 123);
        for _ in 0..(120 * 30) {
            let ia = [a1.update(&a, STEP), a2.update(&a, STEP)];
            a.update(STEP, ia);
            let ib = [b1.update(&b, STEP), b2.update(&b, STEP)];
            b.update(STEP, ib);
        }
        assert_eq!(a.score, b.score, "{sport:?}");
        assert_eq!(a.stats.hits, b.stats.hits, "{sport:?}");
        assert_eq!(a.ball.x.to_bits(), b.ball.x.to_bits(), "{sport:?}");
        assert_eq!(a.players[0].mana.to_bits(), b.players[0].mana.to_bits());
        assert_eq!(a.players[1].mana.to_bits(), b.players[1].mana.to_bits());
    }
}

#[test]
fn all_arenas_are_safe_for_every_sport() {
    for sport in Sport::ALL {
        for arena in 0..11 {
            let mut m = Match::new(MatchConfig {
                arena,
                p1: arena % CHARACTERS.len(),
                p2: (arena + 7) % CHARACTERS.len(),
                seed: 9000 + arena as u64,
                ..config(sport)
            });
            let mut ai1 = AIController::new(Difficulty::Normal, 0, 41);
            let mut ai2 = AIController::new(Difficulty::Normal, 1, 43);
            for _ in 0..(120 * 8) {
                let i0 = ai1.update(&m, STEP);
                let i1 = ai2.update(&m, STEP);
                m.update(STEP, [i0, i1]);
            }
            assert!(finite_match(&m), "{sport:?} arena {arena}");
            assert!(m.ball.x >= -0.01 && m.ball.x <= W + 0.01);
            assert!(m.ball.y >= CEILING_Y - 0.01 && m.ball.y <= GROUND_Y + 0.01);
        }
    }
}
