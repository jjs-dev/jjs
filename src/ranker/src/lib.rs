//! Ranker is library, responsible for generating monitor
//! Is is used in both frontend and invoker

use serde::Serialize;
use std::{cmp::Ordering, collections::HashMap, num::NonZeroU32};

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Serialize, Copy, Clone)]
pub struct SubtaskId(pub NonZeroU32);

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Serialize, Copy, Clone)]
pub struct RunId(pub NonZeroU32);

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Serialize, Copy, Clone)]
pub struct ProblemId(pub NonZeroU32);

#[derive(Hash, Ord, PartialOrd, Eq, PartialEq, Debug, Serialize, Copy, Clone)]
pub struct PartyId(pub NonZeroU32);

pub type Score = i32;

#[derive(Debug)]
pub struct Run {
    pub subtasks: HashMap<SubtaskId, Score>,
    pub party: PartyId,
    pub problem: ProblemId,
}

/// Represents one cell in monitor
#[derive(Debug, Serialize)]
pub struct Cell {
    /// True if party haven't attempted to solve problem
    pub empty: bool,
    /// True if problem is solved
    pub ok: bool,
    /// Score gained
    pub score: Score,
    /// True if cell should be highlighted
    /// For example, in ICPC contest run will be `marked` probably if it is first full solution for problem
    pub marked: bool,
    /// Count of non-ignored attempts which should be displayed
    ///
    /// E.g. not accounts for runs after full solution
    pub attempts: u32,
}

/// Represents some properties of row, describing party
#[derive(Debug, Serialize)]
pub struct PartyStats {
    /// This is used to distinguish groups of parties
    ///
    /// E.g.: participants who solved 8 problems, get `color == 0`; participants who solved 7
    /// problems get `color == 1`, and so on.
    ///
    /// Probably, you will want to use not color, but `color % 2`.
    pub color: u32,
}

#[derive(Debug, Serialize)]
pub struct PartyRow {
    stats: PartyStats,
    problems: HashMap<ProblemId, Cell>,
}

/// Represents some statistics of problem
#[derive(Debug, Serialize)]
pub struct ProblemStats {
    pub total_runs: u32,
    pub accepted_runs: u32,
    pub max_score: Score,
}

#[derive(Debug, Serialize)]
pub struct StatsRow {
    pub problems: HashMap<ProblemId, ProblemStats>,
}

/// Determines which runs will be used to calculate total score for problem
///
/// Note that exact way of calculating score depends on [`RunScoreAggregation`](RunScoreAggregation)
#[derive(Debug)]
pub enum RunScoreAggregationTarget {
    /// `k` latest (later = id is greater) will be used
    Latest(u32),
    /// Run with max score will be used
    /// If there are several runs with max score, run with greatest id will be used
    Best,
    /// All runs will be used
    All,
}

/// Determines how total score for problem is calculated
#[derive(Debug)]
pub enum RunScoreAggregation {
    /// Score for problem maximum run score on this problem
    Max,
    /// Score for problem is sum of subtask score for all subtasks
    /// Subtask score is maximum run score on this subtask
    MergeSubtasks,
}

#[derive(Debug)]
pub struct ProblemConfig {
    pub name: String,
    pub accepted_score: Score,
    pub score_runs: RunScoreAggregationTarget,
    pub aggregation: RunScoreAggregation,
}

#[derive(Debug)]
pub enum PenaltyAggregation {
    Sum,
    Max,
}

#[derive(Debug)]
pub enum ProblemScoreAggregationTarget {
    All,
    Best(u32),
}

#[derive(Debug)]
pub struct Config {
    pub penalty_aggregation: PenaltyAggregation,
    pub score_problems: ProblemScoreAggregationTarget,
}

#[derive(Debug, Serialize)]
pub struct Monitor {
    pub parties: HashMap<PartyId, PartyRow>,
    pub stats: StatsRow,
}

/// Builds a `Monitor`, given list of all runs
// Probably, later some means to build this incrementally will be implemented
pub fn build_monitor(
    runs: &[Run],
    problems: &[(ProblemId, ProblemConfig)],
    parties: &[PartyId],
    _config: &Config,
) -> Monitor {
    let mut party_info = HashMap::new();
    let mut runs_by_party_and_problem = HashMap::new();
    for (i, run) in runs.iter().enumerate() {
        let k = (run.party, run.problem);
        runs_by_party_and_problem
            .entry(k)
            .or_insert_with(Vec::new)
            .push(i);
    }
    let mut cell_by_party_and_problem = HashMap::new();

    let mut stats = StatsRow {
        problems: HashMap::new(),
    };
    for problem in problems {
        stats.problems.insert(
            problem.0,
            ProblemStats {
                total_runs: 0,
                accepted_runs: 0,
                max_score: 0,
            },
        );
    }

    for &party in parties {
        for problem in problems {
            let mut cell = Cell {
                empty: true,
                ok: false,
                score: 0,
                // TODO
                marked: false,
                attempts: 0,
            };
            let empty_run_ids = Vec::new();
            let run_ids = match runs_by_party_and_problem.get(&(party, problem.0)) {
                Some(ids) => ids,
                None => &empty_run_ids,
            };
            let problem_stats = stats.problems.get_mut(&problem.0).unwrap();
            for &run_serial_id in run_ids {
                let run = &runs[run_serial_id];
                cell.empty = false;
                let mut run_score = 0;
                for &sc in run.subtasks.values() {
                    run_score += sc;
                }
                cell.score = std::cmp::max(cell.score, run_score);
                problem_stats.total_runs += 1;
                match run_score.cmp(&problem.1.accepted_score) {
                    Ordering::Less => {
                        cell.attempts += 1;
                    }
                    Ordering::Equal => {
                        cell.ok = true;
                        problem_stats.accepted_runs += 1;
                    }
                    Ordering::Greater => {
                        // TODO handle error gracefully
                        panic!("run's score is more then total possible")
                    }
                }
                problem_stats.max_score = std::cmp::max(problem_stats.max_score, run_score);
            }
            cell_by_party_and_problem.insert((party, problem.0), cell);
        }
    }
    for &party in parties {
        let stats = PartyStats { color: 0 };
        let mut row = PartyRow {
            stats,
            problems: HashMap::new(),
        };
        for problem in problems {
            row.problems.insert(
                problem.0,
                cell_by_party_and_problem
                    .remove(&(party, problem.0))
                    .unwrap(),
            );
        }
        party_info.insert(party, row);
    }
    Monitor {
        parties: party_info,
        stats,
    }
}
