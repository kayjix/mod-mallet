//! The places of the world map, and whose they are.
//!
//! Each map marker is the game's own, added by a mod, changed by a mod or
//! removed by one (ADR 0031). This is decided by comparing the version of
//! the plugin that placed the marker with the version the game uses.

use mallet_core::facts::markers::{MapMarkerFact, MarkerVersion};
use mallet_core::{PlaceKind, PlaceOrigin};

/// How far a mod has to move a place before it counts as moved, in game
/// units: about 3.5 metres. Smaller shifts are corrections, not changes.
const MOVED: i64 = 256;

/// Whose a place is, and which plugin made it so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Origin<'a> {
    /// Whose it is.
    pub(crate) whose: PlaceOrigin,
    /// The plugin that added, changed or removed it. `None` for the game's.
    pub(crate) plugin: Option<&'a str>,
    /// Where the game puts it, if a mod moved it.
    pub(crate) moved_from: Option<[i32; 3]>,
    /// What the game calls it, if a mod renamed it.
    pub(crate) renamed_from: Option<&'a str>,
}

/// Whose `marker` is.
pub(crate) fn origin(marker: &MapMarkerFact) -> Origin<'_> {
    let (first, last) = (&marker.first, &marker.last);
    let game = Origin {
        whose: PlaceOrigin::Game,
        plugin: None,
        moved_from: None,
        renamed_from: None,
    };
    if !first.official {
        // A mod's own place, unless a later plugin took it away again.
        let removed = last.removed && last.plugin != first.plugin;
        return Origin {
            whose: if removed {
                PlaceOrigin::Removed
            } else {
                PlaceOrigin::Added
            },
            plugin: Some(if removed { &last.plugin } else { &first.plugin }),
            ..game
        };
    }
    if last.official {
        return game;
    }
    if last.removed && !first.removed {
        return Origin {
            whose: PlaceOrigin::Removed,
            plugin: Some(&last.plugin),
            ..game
        };
    }
    let moved_from = moved(first, last).then_some(first.position);
    let renamed_from = (first.name != last.name)
        .then_some(first.name.as_deref())
        .flatten();
    if moved_from.is_none() && renamed_from.is_none() && first.kind == last.kind {
        // A mod that repeats the game's version changes nothing.
        return game;
    }
    Origin {
        whose: PlaceOrigin::Changed,
        plugin: Some(&last.plugin),
        moved_from,
        renamed_from,
    }
}

/// Whether `last` stands somewhere else than `first`.
fn moved(first: &MarkerVersion, last: &MarkerVersion) -> bool {
    let distance_squared: i64 = first
        .position
        .iter()
        .zip(&last.position)
        .map(|(a, b)| {
            let difference = i64::from(*a) - i64::from(*b);
            difference.saturating_mul(difference)
        })
        .fold(0, i64::saturating_add);
    distance_squared > MOVED * MOVED
}

/// The group of the game's kind of map marker `kind`.
pub(crate) fn kind(kind: u8) -> PlaceKind {
    use PlaceKind::{
        Camp, Castle, Cave, City, Clearing, DaedricShrine, Docks, Doomstone, DragonLair,
        DwemerRuin, Farm, Fort, GiantCamp, Grove, ImperialCamp, ImperialTower, Landmark,
        Lighthouse, Mine, NordicDwelling, NordicRuin, NordicTower, OrcStronghold, Other, Pass,
        Passage, Settlement, Shack, Shipwreck, Shrine, Smelter, Stable, StormcloakCamp, Town,
        WheatMill, WoodMill,
    };
    match kind {
        1 => City,
        2 | 54 => Town,
        3 | 56 => Settlement,
        4 => Cave,
        5 => Camp,
        6 => Fort,
        7 => NordicRuin,
        8 => DwemerRuin,
        9 => Shipwreck,
        10 => Grove,
        11 | 26 | 53 => Landmark,
        12 => DragonLair,
        13 => Farm,
        14 => WoodMill,
        15 => Mine,
        16 => ImperialCamp,
        17 => StormcloakCamp,
        18 | 55 => Doomstone,
        19 => WheatMill,
        20 => Smelter,
        21 => Stable,
        22 => ImperialTower,
        23 => Clearing,
        24 => Pass,
        25 => Shrine,
        27 => Lighthouse,
        28 => OrcStronghold,
        29 => GiantCamp,
        30 => Shack,
        31 => NordicTower,
        32 => NordicDwelling,
        33 => Docks,
        34 => DaedricShrine,
        // The holds' castles (odd) and capitals (even).
        35..=52 if kind.is_multiple_of(2) => City,
        35..=52 | 59 => Castle,
        57 | 58 => Passage,
        _ => Other,
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::RecordId;

    use super::*;

    fn version(plugin: &str, position: [i32; 3], name: &str) -> MarkerVersion {
        MarkerVersion {
            plugin: plugin.to_owned(),
            official: plugin == "Skyrim.esm" || plugin == "Update.esm",
            kind: 2,
            name: Some(name.to_owned()),
            position,
            removed: false,
        }
    }

    fn marker(first: MarkerVersion, last: MarkerVersion) -> MapMarkerFact {
        MapMarkerFact {
            id: RecordId {
                plugin: first.plugin.to_lowercase(),
                local_id: 1,
            },
            worldspace: RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id: 0x3C,
            },
            first,
            last,
        }
    }

    #[test]
    fn the_games_own_places_stay_the_games_even_if_a_mod_repeats_them() {
        let game = version("Skyrim.esm", [0, 0, 0], "Millbrook");
        assert_eq!(
            origin(&marker(game.clone(), game.clone())).whose,
            PlaceOrigin::Game
        );
        let repeated = version("Town.esp", [100, 0, 0], "Millbrook");
        assert_eq!(
            origin(&marker(game.clone(), repeated)).whose,
            PlaceOrigin::Game
        );
        // The game's own update changes nothing that a mod did.
        let updated = version("Update.esm", [9000, 0, 0], "Millbrook");
        assert_eq!(origin(&marker(game, updated)).whose, PlaceOrigin::Game);
    }

    #[test]
    fn a_mod_that_moves_or_renames_a_place_changes_it() {
        let game = version("Skyrim.esm", [0, 0, 0], "Millbrook");
        let moved_marker = marker(game.clone(), version("Town.esp", [5000, 0, 0], "Millbrook"));
        let moved = origin(&moved_marker);
        assert_eq!(moved.whose, PlaceOrigin::Changed);
        assert_eq!(moved.plugin, Some("Town.esp"));
        assert_eq!(moved.moved_from, Some([0, 0, 0]));
        assert_eq!(moved.renamed_from, None);

        let renamed_marker = marker(game, version("Town.esp", [0, 0, 0], "Millbrook City"));
        let renamed = origin(&renamed_marker);
        assert_eq!(renamed.whose, PlaceOrigin::Changed);
        assert_eq!(renamed.renamed_from, Some("Millbrook"));
    }

    #[test]
    fn places_of_mods_are_added_and_can_be_removed() {
        let added = version("Village.esp", [0, 0, 0], "New Village");
        let added_marker = marker(added.clone(), added.clone());
        let found = origin(&added_marker);
        assert_eq!(found.whose, PlaceOrigin::Added);
        assert_eq!(found.plugin, Some("Village.esp"));

        let removed = MarkerVersion {
            plugin: "Patch.esp".to_owned(),
            removed: true,
            ..added.clone()
        };
        let removed_marker = marker(added, removed);
        let found = origin(&removed_marker);
        assert_eq!(found.whose, PlaceOrigin::Removed);
        assert_eq!(found.plugin, Some("Patch.esp"));

        let game = version("Skyrim.esm", [0, 0, 0], "Millbrook");
        let gone = MarkerVersion {
            plugin: "Patch.esp".to_owned(),
            official: false,
            removed: true,
            ..game.clone()
        };
        assert_eq!(origin(&marker(game, gone)).whose, PlaceOrigin::Removed);
    }

    #[test]
    fn kinds_are_grouped_as_the_map_shows_them() {
        assert_eq!(kind(1), PlaceKind::City);
        assert_eq!(kind(4), PlaceKind::Cave);
        assert_eq!(kind(35), PlaceKind::Castle);
        assert_eq!(kind(36), PlaceKind::City);
        assert_eq!(kind(58), PlaceKind::Passage);
        assert_eq!(kind(0), PlaceKind::Other);
        assert_eq!(kind(200), PlaceKind::Other);
    }
}
