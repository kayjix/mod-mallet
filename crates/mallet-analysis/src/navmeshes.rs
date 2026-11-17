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
//!
//! Where the earlier version connects a door that a mod added and the
//! winning version does not, nobody but the player can use that door any
//! more. That is reported as a warning; the rest as notes.

use std::collections::{HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::NavmeshChange;
use mallet_core::{Facts, Finding, GamePath, Party, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many places a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Reports pairs of plugins where the later one undoes the earlier one's
/// changes to navigation meshes.
pub struct NavmeshConflicts;

/// What one plugin undoes of another.
#[derive(Default)]
struct Undone<'a> {
    /// How many navigation meshes.
    count: usize,
    /// Where they lie.
    places: Vec<&'a str>,
    /// How many doors that mods add they no longer connect.
    doors: usize,
    /// Where those doors are.
    door_places: Vec<&'a str>,
}

impl Analysis for NavmeshConflicts {
    fn id(&self) -> &'static str {
        "navmesh-conflicts"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        undone(context.facts)
            .into_iter()
            .map(|((winner, loser), undone)| finding(context, winner, loser, undone))
            .collect()
    }
}

/// What each later plugin undoes of each earlier one, by (winner, loser).
fn undone(facts: &Facts) -> HashMap<(&str, &str), Undone<'_>> {
    let mut changed_by: HashMap<&RecordId, Vec<(&str, &NavmeshChange)>> = HashMap::new();
    for changes in &facts.navmeshes.changes {
        for navmesh in &changes.navmeshes {
            changed_by
                .entry(&navmesh.id)
                .or_default()
                .push((changes.plugin.as_str(), navmesh));
        }
    }
    // Doors that mods add. A door of the game that a mod moves keeps its
    // identity, and versions made from the game's mesh connect it too.
    let added: HashSet<&RecordId> = facts
        .objects
        .placed
        .iter()
        .filter(|object| object.moved.is_none())
        .map(|object| &object.id)
        .collect();
    let masters: HashMap<String, Vec<String>> = facts
        .plugins
        .iter()
        .map(|plugin| {
            let masters = plugin.masters.iter().map(|master| master.to_lowercase());
            (plugin.name.to_lowercase(), masters.collect())
        })
        .collect();
    let mut ancestors: HashMap<String, HashSet<String>> = HashMap::new();

    let mut undone: HashMap<(&str, &str), Undone<'_>> = HashMap::new();
    for plugins in changed_by.values() {
        let Some(((winner, used), earlier)) = plugins.split_last() else {
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
            let cut_off = doors_cut_off(change, used, &added);
            if cut_off > 0 {
                entry.doors += cut_off;
                entry.door_places.extend(change.place.as_deref());
            }
        }
    }
    undone
}

/// How many doors that mods add the `lost` version of a navigation mesh
/// connects and the `used` one does not. Zero where either version's
/// doors are unknown.
fn doors_cut_off(lost: &NavmeshChange, used: &NavmeshChange, added: &HashSet<&RecordId>) -> usize {
    let (Some(lost), Some(used)) = (&lost.doors, &used.doors) else {
        return 0;
    };
    lost.iter()
        .filter(|door| added.contains(door) && !used.contains(door))
        .count()
}

/// The finding about what `winner` undoes of `loser`.
fn finding(context: &Context<'_>, winner: &str, loser: &str, undone: Undone<'_>) -> Finding {
    let path_of = |name: &str| {
        context
            .facts
            .plugins
            .iter()
            .find(|plugin| plugin.name.eq_ignore_ascii_case(name))
            .map_or_else(
                || GamePath::new("data").join(name),
                |plugin| plugin.path.clone(),
            )
    };
    let (winner_path, loser_path) = (path_of(winner), path_of(loser));
    // The pair in a fixed order, so that the identity does not change when
    // the load order does.
    let (first, second) = if winner_path <= loser_path {
        (winner_path, loser_path.clone())
    } else {
        (loser_path.clone(), winner_path)
    };
    let (kind, count, places) = if undone.doors > 0 {
        (
            &kinds::NAVMESHES_DOOR_CUT_OFF,
            undone.doors,
            undone.door_places,
        )
    } else {
        (
            &kinds::NAVMESHES_CHANGES_UNDONE,
            undone.count,
            undone.places,
        )
    };
    Finding::new(
        kind,
        Subject::Pair {
            first: Party::File { path: first },
            second: Party::File { path: second },
        },
    )
    .with_arg("winner", winner)
    .with_arg("loser", loser)
    .with_arg("count", i64::try_from(count).unwrap_or(i64::MAX))
    .with_arg("places", examples(places, EXAMPLE_LIMIT))
    .with_provider(context.mods.provider_of(&loser_path))
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
    use mallet_core::facts::objects::{ObjectFacts, Place, PlacedObject};
    use mallet_core::{ArgValue, Confidence, NoModSource, Severity};

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
                    doors: None,
                })
                .collect(),
        }
    }

    fn navmesh_findings(facts: &Facts) -> Vec<Finding> {
        navmesh_findings_of(facts, "navmeshes.changes-undone")
    }

    fn navmesh_findings_of(facts: &Facts, kind: &str) -> Vec<Finding> {
        run_all(&Context::new(facts, &NoModSource))
            .into_iter()
            .filter(|finding| finding.kind == kind)
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

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    /// The navigation mesh 0x10 in `plugin`'s version, connecting `doors`.
    fn with_doors(plugin: &str, doors: Vec<RecordId>) -> NavmeshChanges {
        NavmeshChanges {
            plugin: plugin.to_owned(),
            navmeshes: vec![NavmeshChange {
                id: id("skyrim.esm", 0x10),
                place: Some("Riverwood".to_owned()),
                doors: Some(doors),
            }],
        }
    }

    fn door(id: RecordId) -> PlacedObject {
        PlacedObject {
            plugin: "House.esp".to_owned(),
            id,
            base: RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id: 0x20,
            },
            place: Place::Interior {
                cell: RecordId {
                    plugin: "skyrim.esm".to_owned(),
                    local_id: 0x30,
                },
            },
            position: [0; 3],
            rotation: [0; 3],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        }
    }

    #[test]
    fn a_door_the_winning_version_does_not_connect_is_a_warning() {
        let house_door = id("house.esp", 0x0800);
        let game_door = id("skyrim.esm", 0x40);
        let mut facts = facts(
            vec![
                plugin("House.esp", &["Skyrim.esm"]),
                plugin("Paths.esp", &["Skyrim.esm"]),
            ],
            vec![
                with_doors("House.esp", vec![game_door.clone(), house_door.clone()]),
                with_doors("Paths.esp", vec![game_door]),
            ],
        );
        facts.objects = ObjectFacts {
            placed: vec![door(house_door)],
            ..ObjectFacts::default()
        };
        let findings = navmesh_findings_of(&facts, "navmeshes.door-cut-off");

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.args["winner"], ArgValue::from("Paths.esp"));
        assert_eq!(finding.args["loser"], ArgValue::from("House.esp"));
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["places"], ArgValue::from("Riverwood"));
        // The same pair is not reported as a note as well.
        assert_eq!(navmesh_findings(&facts), Vec::new());
    }

    #[test]
    fn doors_of_the_game_and_doors_still_connected_are_no_warning() {
        let house_door = id("house.esp", 0x0800);
        let mut facts = facts(
            vec![
                plugin("House.esp", &["Skyrim.esm"]),
                plugin("Paths.esp", &["Skyrim.esm"]),
            ],
            vec![
                with_doors(
                    "House.esp",
                    vec![id("skyrim.esm", 0x40), house_door.clone()],
                ),
                with_doors("Paths.esp", vec![house_door.clone()]),
            ],
        );
        facts.objects = ObjectFacts {
            placed: vec![door(house_door)],
            ..ObjectFacts::default()
        };
        assert_eq!(
            navmesh_findings_of(&facts, "navmeshes.door-cut-off"),
            Vec::new()
        );
        assert_eq!(navmesh_findings(&facts).len(), 1);
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
