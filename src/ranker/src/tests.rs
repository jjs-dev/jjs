use super::*;
use maplit::btreemap as map;

const SIMPLE_CONFIG: Config = Config {
    penalty_aggregation: PenaltyAggregation::Sum,
    score_problems: ProblemScoreAggregationTarget::All,
};

const EMPTY_CELL: Cell = Cell {
    empty: true,
    ok: false,
    score: 0,
    marked: false,
    attempts: 0,
};

fn simple_problem_config(name: &str) -> ProblemConfig {
    ProblemConfig {
        name: name.to_string(),
        accepted_score: 100,
        score_runs: RunScoreAggregationTarget::All,
        aggregation: RunScoreAggregation::Max,
    }
}

fn problem_id(n: u32) -> ProblemId {
    ProblemId(NonZeroU32::new(n).unwrap())
}

fn party_id(n: u32) -> PartyId {
    PartyId(NonZeroU32::new(n).unwrap())
}

fn subtask_id(n: u32) -> SubtaskId {
    SubtaskId(NonZeroU32::new(n).unwrap())
}

fn check_same(expected: Monitor, actual: Monitor) {
    pretty_assertions::assert_eq!(expected, actual);
}

#[test]
fn test_simple() {
    let prob_easy = problem_id(1);
    let prob_hard = problem_id(2);
    let problems = [
        (prob_easy, simple_problem_config("easy")),
        (prob_hard, simple_problem_config("hard")),
    ];
    let user1 = party_id(1);
    let user2 = party_id(2);
    // codeforces.com/profile/tourist
    let korotkevich = party_id(3);
    let runs = [
        // user1 solved easy
        Run {
            subtasks: map! {
                subtask_id(1) => 50,
                subtask_id(2) => 50
            },
            party: user1,
            problem: prob_easy,
        },
        // user2 also got good score on easy
        Run {
            subtasks: map! {
                subtask_id(1) => 50,
                subtask_id(2) => 35
            },
            party: user2,
            problem: prob_easy,
        },
        // user2 attempted hard, but unsuccessfully
        Run {
            subtasks: map! {
            subtask_id(1) => 0
            },
            party: user2,
            problem: prob_hard,
        },
        // well, easy is easy
        Run {
            subtasks: map! {
            subtask_id(1) => 50,
            subtask_id(2) => 50
            },
            party: korotkevich,
            problem: prob_easy,
        },
        // hard is hard even for G. Korotkevich
        Run {
            subtasks: map! {
                subtask_id(1) => 40,
                subtask_id(2) => 25
            },
            party: korotkevich,
            problem: prob_hard,
        },
        Run {
            subtasks: map! {
            subtask_id(1) => 40,
            subtask_id(2) => 40,
            subtask_id(3) => 11
            },
            party: korotkevich,
            problem: prob_hard,
        },
    ];
    let parties = [user1, user2, korotkevich];
    let monitor = build_monitor(&runs, &problems, &parties, &SIMPLE_CONFIG);
    let expected = Monitor {
        parties: map! {
            user1 => PartyRow {
                stats: PartyStats {
                   color: 1
                },
                problems: map! {
                    prob_easy => Cell {
                         empty: false,
                         ok: true,
                         score: 100,
                         marked: false,
                         attempts: 1
                    },
                    prob_hard => EMPTY_CELL,
                }
            },
            user2 => PartyRow {
                stats: PartyStats {
                    color: 1
                },
                problems: map! {
                    prob_easy => Cell {
                        empty: false,
                        ok: false,
                        score: 85,
                        marked: false,
                        attempts: 1
                    },
                    prob_hard => Cell {
                        empty: false,
                        ok: false,
                        score: 0,
                        marked: false,
                        attempts: 1
                    }
                }
            },
            korotkevich => PartyRow {
                stats: PartyStats {
                    color: 0
                },
                 problems: map! {
                     prob_easy => Cell {
                         empty: false,
                         ok: true,
                         score: 100,
                         marked: false,
                         attempts: 1
                     },
                     prob_hard => Cell {
                         empty: false,
                         ok: false,
                         score: 91,
                         marked: false,
                         attempts: 2,
                     }
                 }
            }
        },
        stats: StatsRow {
            problems: map! {
                prob_easy => ProblemStats {
                    total_runs: 3,
                    accepted_runs: 2,
                    max_score: 100
                },
                prob_hard => ProblemStats {
                    total_runs: 3,
                    accepted_runs: 0,
                    max_score: 91,
                }
            },
        },
    };
    check_same(expected, monitor);
}
