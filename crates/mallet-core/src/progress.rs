//! How far a scan has come.
//!
//! A scan takes a while on a large setup. The engine tells whoever asked
//! for it which step it is in and how many of the step's items are done,
//! so that a shell can show it. Like a finding, a step carries no prose:
//! it names the step, and the words for it belong to the shell's texts.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The steps of a scan, in the order they are taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ScanStep {
    /// Finding the game and listing its files.
    Library,
    /// Looking into the game's archives.
    Archives,
    /// Reading the loose meshes.
    Meshes,
    /// Reading the plugins.
    Plugins,
    /// Reading the mods from the mod manager.
    Mods,
    /// Reading the models of objects that may collide.
    Models,
    /// Looking for problems in everything read.
    Checking,
    /// Remembering the result.
    Saving,
}

impl ScanStep {
    /// Every step, in the order they are taken.
    pub const ALL: [Self; 8] = [
        Self::Library,
        Self::Archives,
        Self::Meshes,
        Self::Plugins,
        Self::Mods,
        Self::Models,
        Self::Checking,
        Self::Saving,
    ];

    /// The position of the step, counted from 1.
    #[must_use]
    pub const fn number(self) -> u32 {
        match self {
            Self::Library => 1,
            Self::Archives => 2,
            Self::Meshes => 3,
            Self::Plugins => 4,
            Self::Mods => 5,
            Self::Models => 6,
            Self::Checking => 7,
            Self::Saving => 8,
        }
    }
}

/// Where a scan stands: the step, and how many of its items are done.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    /// The step the scan is in.
    pub step: ScanStep,
    /// The step's position, counted from 1.
    pub step_number: u32,
    /// How many steps there are.
    pub steps: u32,
    /// How many items of the step are done.
    pub done: u32,
    /// How many items the step has; 0 if that is not known.
    pub of: u32,
}

/// The least time between two reports of counts, so that reporting never
/// slows a scan down.
const MIN_INTERVAL: Duration = Duration::from_millis(200);

/// What a scan reports its progress to. Reports are rationed: a step's
/// beginning and the first and last count always get through, the counts in
/// between at most a few times a second.
pub struct Progress<'a> {
    sink: Option<&'a (dyn Fn(ScanProgress) + Sync)>,
    last_count: Mutex<Option<Instant>>,
}

impl<'a> Progress<'a> {
    /// A scan whose progress nobody follows.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            sink: None,
            last_count: Mutex::new(None),
        }
    }

    /// A scan whose progress goes to `sink`.
    #[must_use]
    pub const fn to(sink: &'a (dyn Fn(ScanProgress) + Sync)) -> Self {
        Self {
            sink: Some(sink),
            last_count: Mutex::new(None),
        }
    }

    /// The scan enters `step`.
    pub fn step(&self, step: ScanStep) {
        self.send(step, 0, 0);
    }

    /// `done` of `of` items of `step` are done.
    pub fn count(&self, step: ScanStep, done: usize, of: usize) {
        if self.sink.is_none() {
            return;
        }
        if done < of && done > 1 {
            let Ok(mut last) = self.last_count.lock() else {
                return;
            };
            let now = Instant::now();
            if last.is_some_and(|at| now.duration_since(at) < MIN_INTERVAL) {
                return;
            }
            *last = Some(now);
        }
        self.send(
            step,
            u32::try_from(done).unwrap_or(u32::MAX),
            u32::try_from(of).unwrap_or(u32::MAX),
        );
    }

    fn send(&self, step: ScanStep, done: u32, of: u32) {
        if let Some(sink) = self.sink {
            sink(ScanProgress {
                step,
                step_number: step.number(),
                steps: u32::try_from(ScanStep::ALL.len()).unwrap_or(u32::MAX),
                done,
                of,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_numbers_follow_the_order() {
        for (index, step) in ScanStep::ALL.iter().enumerate() {
            assert_eq!(usize::try_from(step.number()).unwrap(), index + 1);
        }
    }

    #[test]
    fn a_quick_run_of_counts_is_rationed_but_ends_complete() {
        let seen = Mutex::new(Vec::new());
        let sink = |progress: ScanProgress| seen.lock().unwrap().push(progress);
        let progress = Progress::to(&sink);
        progress.step(ScanStep::Plugins);
        for done in 1..=1000 {
            progress.count(ScanStep::Plugins, done, 1000);
        }
        let seen = seen.lock().unwrap();
        assert!(seen.len() < 20, "{} reports", seen.len());
        let last = seen.last().unwrap();
        assert_eq!((last.done, last.of), (1000, 1000));
        assert_eq!(seen.first().unwrap().of, 0);
    }

    #[test]
    fn nobody_listening_costs_nothing() {
        let progress = Progress::none();
        progress.step(ScanStep::Library);
        progress.count(ScanStep::Archives, 1, 2);
    }
}
