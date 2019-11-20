//! Ranker is library, responsible for generating monitor
//! Is is used in both frontend and invoker

#[cfg(test)]
mod tests;

use serde::Serialize;
use std::{cmp, cmp::Ordering, collections::BTreeMap, num::NonZeroU32};

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
    pub subtasks: BTreeMap<SubtaskId, Score>,
    pub party: PartyId,
    pub problem: ProblemId,
}

/// Represents one cell in monitor
#[derive(Debug, Serialize, Eq, PartialEq)]
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
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PartyStats {
    /// This is used to distinguish groups of parties
    ///
    /// E.g.: participants who solved 8 problems, get `color == 0`; participants who solved 7
    /// problems get `color == 1`, and so on.
    ///
    /// Probably, you will want to use not color, but `color % 2`.
    pub color: u32,
    /// Total score gained by party in contest
    pub score: Score,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct PartyRow {
    stats: PartyStats,
    problems: BTreeMap<ProblemId, Cell>,
}

/// Represents some statistics of problem
#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct ProblemStats {
    pub total_runs: u32,
    /// How many runs are accepted. If one party made two accepted runs, both are counted.
    pub accepted_runs: u32,
    /// Max party score gained on this problem.
    ///
    /// Note: if `merge_subtasks` mode is enabled, it is possible that e.g. max_score is 100
    /// but every particular run got <=90 points.
    pub max_score: Score,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct StatsRow {
    pub problems: BTreeMap<ProblemId, ProblemStats>,
}

/// Determines which runs will be used to calculate total score for problem.
///
/// Note that exact way of calculating score depends on [`RunScoreAggregation`](RunScoreAggregation)
#[derive(Debug)]
pub enum RunScoreAggregationTarget {
    /// All runs will be used
    All,
    /// `k` latest (later = id is greater) will be used
    Latest(u32),
    /// Run with max score will be used
    /// If there are several runs with max score, run with greatest id will be used
    Best,
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

#[derive(Debug, Serialize, Eq, PartialEq)]
pub struct Monitor {
    pub parties: BTreeMap<PartyId, PartyRow>,
    pub stats: StatsRow,
}

/// Builds a `Monitor`, given list of all runs
/// # Panics
/// Panics if provided arguments are invalid
// Probably, later some means to build this incrementally will be implemented
pub fn build_monitor(
    runs: &[Run],
    problems: &[(ProblemId, ProblemConfig)],
    parties: &[PartyId],
    _config: &Config,
) -> Monitor {
    let mut party_info = BTreeMap::new();
    let mut runs_by_party_and_problem = BTreeMap::new();
    for (i, run) in runs.iter().enumerate() {
        let k = (run.party, run.problem);
        runs_by_party_and_problem
            .entry(k)
            .or_insert_with(Vec::new)
            .push(i);
    }
    let mut cell_by_party_and_problem = BTreeMap::new();

    let mut stats = StatsRow {
        problems: BTreeMap::new(),
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
            let empty_run_ids = Vec::new();
            let run_ids = match runs_by_party_and_problem.get(&(party, problem.0)) {
                Some(ids) => ids,
                None => &empty_run_ids,
            };
            let runs = run_ids.iter().map(|&run_id| &runs[run_id]);
            let problem_stats = stats.problems.get_mut(&problem.0).unwrap();

            let cell = build_cell(runs, &problem.1, problem_stats);

            cell_by_party_and_problem.insert((party, problem.0), cell);
        }
    }
    for &party in parties {
        let stats = PartyStats { color: 0, score: 0 };
        let mut row = PartyRow {
            stats,
            problems: BTreeMap::new(),
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
    let mut mon = Monitor {
        parties: party_info,
        stats,
    };
    build_party_stats(&mut mon, parties);
    mon
}

fn build_cell<'a>(
    runs: impl Iterator<Item = &'a Run>,
    problem: &ProblemConfig,
    problem_stats: &mut ProblemStats,
) -> Cell {
    let mut cell = Cell {
        empty: true,
        ok: false,
        score: 0,
        // TODO
        marked: false,
        attempts: 0,
    };
    let mut max_based_score = 0;
    let mut merge_subtask_based_score = BTreeMap::new();

    for run in runs {
        // cell is not empty, because there are attempts for this problem
        cell.empty = false;
        problem_stats.total_runs += 1;
        let mut run_score = 0;
        for (&st_id, &st_score) in run.subtasks.iter() {
            run_score += st_score;
            let subtask_opt = merge_subtask_based_score.entry(st_id).or_insert(0);
            *subtask_opt = cmp::max(*subtask_opt, st_score);
        }
        max_based_score = cmp::max(max_based_score, run_score);

        match run_score.cmp(&problem.accepted_score) {
            Ordering::Less => {
                cell.attempts += 1;
            }
            Ordering::Equal => {
                cell.ok = true;
                problem_stats.accepted_runs += 1;
            }
            Ordering::Greater => panic!("run's score is more than total possible"),
        }
    }
    cell.score = match problem.aggregation {
        RunScoreAggregation::Max => max_based_score,
        RunScoreAggregation::MergeSubtasks => {
            merge_subtask_based_score.into_iter().map(|(_k, v)| v).sum()
        }
    };
    problem_stats.max_score = std::cmp::max(problem_stats.max_score, cell.score);
    cell
}

fn build_party_stats(mon: &mut Monitor, parties: &[PartyId]) {
    // step 1: calculate PartyStats.score
    for party in parties {
        let mut score = 0;
        for cell in mon.parties[party].problems.values() {
            score += cell.score;
        }
        mon.parties.get_mut(party).unwrap().stats.score = score;
    }
    // step 2: calculate PartyStats.color
    // at first, we want to calculate coloring key
    // TODO: it is hardcoded as score / 100
    let mut coloring_key = BTreeMap::new();
    for &party in parties {
        coloring_key.insert(party, mon.parties[&party].stats.score / 100);
    }
    let mut distinct_color_keys: Vec<_> = coloring_key
        .values()
        .copied()
        .map(std::cmp::Reverse)
        .collect();
    distinct_color_keys.sort_unstable();
    distinct_color_keys.dedup_by_key(|x| *x);
    // and now, color of party is position of it's coloring key in `distinct_color_keys`
    for &party in parties {
        let color = distinct_color_keys
            .binary_search(&std::cmp::Reverse(coloring_key[&party]))
            .expect("distinct_color_keys is incorrect");
        mon.parties.get_mut(&party).unwrap().stats.color = color as u32;
    }
}
