//! Changes to where characters can walk that a later plugin undoes.
//!
//! A plugin that changes a navigation mesh replaces it as a whole, so of
//! several plugins that change the same one, only the last one's version
//! is used. That is fine when the last plugin was made with the earlier
//! ones as masters: its author saw their changes and could keep them. When
//! it was not, characters walk where the earlier mod's buildings now stand,
//! or no longer find the way into what the earlier mod added.
//!
//! Plugins are compared through all their masters: a plugin made on top of
//! a patch counts as made with everything the patch was made with.

use std::collections::{HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::NavmeshChange;
use mallet_core::{Finding, GamePath, Party, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many places a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Reports pairs of plugins where the later one undoes the earlier one's
/// changes to navigation meshes.
pub struct NavmeshConflicts;

/// The navigation meshes one plugin undoes of another.
#[derive(Default)]
struct Undone<'a> {
    count: usize,
    places: Vec<&'a str>,
}

impl Analysis for NavmeshConflicts {
    fn id(&self) -> &'static str {
        "navmesh-conflicts"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let facts = context.facts;
        let mut changed_by: HashMap<&RecordId, Vec<(&str, &NavmeshChange)>> = HashMap::new();
        for changes in &facts.navmeshes.changes {
            for navmesh in &changes.navmeshes {
                changed_by
                    .entry(&navmesh.id)
                    .or_default()
                    .push((changes.plugin.as_str(), navmesh));
            }
        }

        let masters: HashMap<String, Vec<String>> = facts
            .plugins
            .iter()
            .map(|plugin| {
                (
                    plugin.name.to_lowercase(),
                    plugin
                        .masters
                        .iter()
                        .map(|master| master.to_lowercase())
                        .collect(),
                )
            })
            .collect();
        let mut ancestors: HashMap<String, HashSet<String>> = HashMap::new();

        let mut undone: HashMap<(&str, &str), Undone<'_>> = HashMap::new();
        for plugins in changed_by.values() {
            let Some(((winner, _), earlier)) = plugins.split_last() else {
                continue;
            };
            let built_on = ancestors
                .entry(winner.to_lowercase())
                .or_insert_with_key(|winner| all_masters(winner, &masters));
            for (loser, change) in earlier {
                if built_on.contains(&loser.to_lowercase()) {
                    continue;
                }
                let entry = undone.entry((*winner, *loser)).or_default();
                entry.count += 1;
                entry.places.extend(change.place.as_deref());
            }
        }

        let path_of = |name: &str| {
            facts
                .plugins
                .iter()
                .find(|plugin| plugin.name.eq_ignore_ascii_case(name))
                .map_or_else(
                    || GamePath::new("data").join(name),
                    |plugin| plugin.path.clone(),
                )
        };
        undone
            .into_iter()
            .map(|((winner, loser), undone)| {
                let (winner_path, loser_path) = (path_of(winner), path_of(loser));
                // The pair in a fixed order, so that the identity does not
                // change when the load order does.
                let (first, second) = if winner_path <= loser_path {
                    (winner_path, loser_path.clone())
                } else {
                    (loser_path.clone(), winner_path)
                };
                Finding::new(
                    &kinds::NAVMESHES_CHANGES_UNDONE,
                    Subject::Pair {
                        first: Party::File { path: first },
                        second: Party::File { path: second },
                    },
                )
                .with_arg("winner", winner)
                .with_arg("loser", loser)
                .with_arg("count", i64::try_from(undone.count).unwrap_or(i64::MAX))
                .with_arg("places", examples(undone.places, EXAMPLE_LIMIT))
                .with_provider(context.mods.provider_of(&loser_path))
            })
            .collect()
    }
}

/// Every plugin that `plugin` builds on, directly or through other masters,
/// in lower case.
fn all_masters(plugin: &str, masters: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut open: Vec<&String> = masters.get(plugin).into_iter().flatten().collect();
    while let Some(master) = open.pop() {
        if seen.insert(master.clone()) {
            open.extend(masters.get(master).into_iter().flatten());
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::navmeshes::{NavmeshChanges, NavmeshFacts};
    use mallet_core::{ArgValue, Confidence, Facts, NoModSource, Severity};

    use super::*;
    use crate::run_all;

    fn plugin(name: &str, masters: &[&str]) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: masters.iter().map(|master| (*master).to_owned()).collect(),
            npcs: Vec::new(),
        }
    }

    fn changes(plugin: &str, navmeshes: &[(u32, &str)]) -> NavmeshChanges {
        NavmeshChanges {
            plugin: plugin.to_owned(),
            navmeshes: navmeshes
                .iter()
                .map(|(local_id, place)| NavmeshChange {
                    id: RecordId {
                        plugin: "skyrim.esm".to_owned(),
                        local_id: *local_id,
                    },
                    place: Some((*place).to_owned()),
                })
                .collect(),
        }
    }

    fn navmesh_findings(facts: &Facts) -> Vec<Finding> {
        run_all(&Context::new(facts, &NoModSource))
            .into_iter()
            .filter(|finding| finding.kind == "navmeshes.changes-undone")
            .collect()
    }

    fn facts(plugins: Vec<PluginFacts>, changes: Vec<NavmeshChanges>) -> Facts {
        Facts {
            plugins,
            navmeshes: NavmeshFacts { changes },
            ..Facts::default()
        }
    }

    #[test]
    fn a_later_plugin_that_undoes_an_earlier_ones_navmesh_is_reported() {
        let facts = facts(
            vec![
                plugin("Skyrim.esm", &[]),
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Farm.esp", &["Skyrim.esm"]),
            ],
            vec![
                changes("Town.esp", &[(0x10, "Riverwood"), (0x11, "Tamriel 5,-3")]),
                changes("Farm.esp", &[(0x10, "Riverwood"), (0x11, "Tamriel 5,-3")]),
            ],
        );
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "navmeshes.changes-undone");
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "navmeshes.changes-undone:pair:file:data/farm.esp+file:data/town.esp"
        );
        assert_eq!(finding.args["winner"], ArgValue::from("Farm.esp"));
        assert_eq!(finding.args["loser"], ArgValue::from("Town.esp"));
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(
            finding.args["places"],
            ArgValue::from("Riverwood, Tamriel 5,-3")
        );
    }

    #[test]
    fn a_patch_made_with_both_plugins_undoes_nothing() {
        let facts = facts(
            vec![
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Farm.esp", &["Skyrim.esm"]),
                plugin("Town - Farm Patch.esp", &["Town.esp", "Farm.esp"]),
            ],
            vec![
                changes("Town.esp", &[(0x10, "Riverwood")]),
                changes("Farm.esp", &[(0x10, "Riverwood")]),
                changes("Town - Farm Patch.esp", &[(0x10, "Riverwood")]),
            ],
        );
        assert_eq!(navmesh_findings(&facts), Vec::new());
    }

    #[test]
    fn only_the_plugins_the_winner_was_not_made_with_lose() {
        let facts = facts(
            vec![
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Farm.esp", &["Skyrim.esm"]),
                plugin("Farm Extended.esp", &["Farm.esp"]),
            ],
            vec![
                changes("Town.esp", &[(0x10, "Riverwood")]),
                changes("Farm.esp", &[(0x10, "Riverwood")]),
                changes("Farm Extended.esp", &[(0x10, "Riverwood")]),
            ],
        );
        let findings = navmesh_findings(&facts);
        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.args["winner"], ArgValue::from("Farm Extended.esp"));
        assert_eq!(finding.args["loser"], ArgValue::from("Town.esp"));
    }

    #[test]
    fn masters_count_through_other_masters() {
        let facts = facts(
            vec![
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Town Extended.esp", &["Town.esp"]),
                plugin("Town Extended - Fixes.esp", &["Town Extended.esp"]),
            ],
            vec![
                changes("Town.esp", &[(0x10, "Riverwood")]),
                changes("Town Extended - Fixes.esp", &[(0x10, "Riverwood")]),
            ],
        );
        assert_eq!(navmesh_findings(&facts), Vec::new());
    }

    #[test]
    fn different_navmeshes_do_not_clash() {
        let facts = facts(
            vec![
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Farm.esp", &["Skyrim.esm"]),
            ],
            vec![
                changes("Town.esp", &[(0x10, "Riverwood")]),
                changes("Farm.esp", &[(0x20, "Whiterun")]),
            ],
        );
        assert_eq!(navmesh_findings(&facts), Vec::new());
    }

    #[test]
    fn masters_that_build_on_each_other_in_a_circle_end() {
        let masters = HashMap::from([
            ("a.esp".to_owned(), vec!["b.esp".to_owned()]),
            ("b.esp".to_owned(), vec!["a.esp".to_owned()]),
        ]);
        assert_eq!(
            all_masters("a.esp", &masters),
            HashSet::from(["a.esp".to_owned(), "b.esp".to_owned()])
        );
    }
}
