//! The catalogue of everything Mod Mallet can report.
//!
//! Every kind of finding is declared here once. The declaration fixes its
//! identifier, how severe and how certain it is, and which arguments its
//! message takes. The texts live in the `mallet-messages` crate, which checks
//! that every kind listed in [`ALL`] has a complete entry in every language.
//!
//! To add a kind: declare a constant, add it to [`ALL`], write its messages,
//! then run `cargo xtask gen` to refresh the reference documentation.

use crate::{Confidence, Severity};

/// The fixed properties of one kind of finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindInfo {
    /// Stable identifier, `area.what-is-wrong`. Never reuse or rename one:
    /// people's "ignore" decisions refer to it.
    pub id: &'static str,
    /// How much findings of this kind matter.
    pub severity: Severity,
    /// How sure Mod Mallet is about findings of this kind.
    pub confidence: Confidence,
    /// The message arguments every finding of this kind carries.
    pub args: &'static [&'static str],
}

/// A script extender plugin was switched off because it was built for a
/// different version of the game.
pub const EXTENDER_PLUGIN_INCOMPATIBLE: KindInfo = KindInfo {
    id: "extender.plugin-incompatible",
    severity: Severity::Error,
    confidence: Confidence::Certain,
    args: &["file", "gameVersion"],
};

/// A script extender plugin was not loaded for a reason Mod Mallet has no
/// dedicated kind for.
pub const EXTENDER_PLUGIN_NOT_LOADED: KindInfo = KindInfo {
    id: "extender.plugin-not-loaded",
    severity: Severity::Error,
    confidence: Confidence::Certain,
    args: &["file", "reason"],
};

/// A DLL in the script extender's plugin directory is not a plugin.
pub const EXTENDER_PLUGIN_NO_VERSION_DATA: KindInfo = KindInfo {
    id: "extender.plugin-no-version-data",
    severity: Severity::Info,
    confidence: Confidence::Certain,
    args: &["file"],
};

/// Characters whose face files and winning record describe different
/// faces.
pub const NPCS_FACE_MISMATCH: KindInfo = KindInfo {
    id: "npcs.face-mismatch",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &[
        "count",
        "examples",
        "winnerPlugin",
        "facePlugin",
        "faceOrigin",
        "faceMod",
    ],
};

/// A mod's installer would now tick options that were not installed.
pub const INSTALLER_OPTIONS_NOW_RECOMMENDED: KindInfo = KindInfo {
    id: "installer.options-now-recommended",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["mod", "count", "examples"],
};

/// An active plugin builds on a plugin that is not installed.
pub const PLUGINS_MASTER_MISSING: KindInfo = KindInfo {
    id: "plugins.master-missing",
    severity: Severity::Error,
    confidence: Confidence::Certain,
    args: &["plugin", "count", "masters"],
};

/// A plugin that a tool generated from the load order was made before the
/// load order last changed.
pub const PLUGINS_GENERATED_OUTDATED: KindInfo = KindInfo {
    id: "plugins.generated-outdated",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["plugin", "tool", "count", "examples"],
};

/// The LOOT masterlist says an active plugin needs a file that is not in
/// the game.
pub const PLUGINS_REQUIREMENT_MISSING: KindInfo = KindInfo {
    id: "plugins.requirement-missing",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["plugin", "count", "files"],
};

/// The LOOT masterlist says an active plugin cannot be used together with
/// a file that is in the game.
pub const PLUGINS_INCOMPATIBLE: KindInfo = KindInfo {
    id: "plugins.incompatible",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["plugin", "other"],
};

/// The LOOT masterlist knows this exact version of a plugin to delete
/// objects or navigation meshes of the game that a cleaning tool restores.
pub const PLUGINS_NEEDS_CLEANING: KindInfo = KindInfo {
    id: "plugins.needs-cleaning",
    severity: Severity::Warning,
    confidence: Confidence::Certain,
    args: &["plugin", "identical", "deleted", "navmeshes", "utility"],
};

/// The LOOT masterlist knows this exact version of a plugin to contain
/// unchanged copies of records, and nothing worse.
pub const PLUGINS_IDENTICAL_RECORDS: KindInfo = KindInfo {
    id: "plugins.identical-records",
    severity: Severity::Info,
    confidence: Confidence::Certain,
    args: &["plugin", "count", "utility"],
};

/// LOOT warns about an active plugin in this setup, in its own words.
pub const PLUGINS_LOOT_WARNING: KindInfo = KindInfo {
    id: "plugins.loot-warning",
    severity: Severity::Warning,
    confidence: Confidence::Hint,
    args: &["plugin", "count", "messages"],
};

/// LOOT has a note about an active plugin in this setup, in its own words.
pub const PLUGINS_LOOT_NOTE: KindInfo = KindInfo {
    id: "plugins.loot-note",
    severity: Severity::Info,
    confidence: Confidence::Hint,
    args: &["plugin", "count", "messages"],
};

/// Two mods place the same object at almost the same spot.
pub const OBJECTS_DUPLICATED: KindInfo = KindInfo {
    id: "objects.duplicated",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["first", "second", "count", "examples"],
};

/// Most of one plugin's objects stand in another plugin as well: the one is
/// part of the other.
pub const PLUGINS_CONTAINED: KindInfo = KindInfo {
    id: "plugins.contained",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &[
        "plugin",
        "pluginFile",
        "container",
        "count",
        "total",
        "dependentCount",
        "dependents",
    ],
};

/// Objects of two mods stand largely inside each other.
pub const OBJECTS_OVERLAPPING: KindInfo = KindInfo {
    id: "objects.overlapping",
    severity: Severity::Warning,
    confidence: Confidence::Hint,
    args: &["first", "second", "count", "examples"],
};

/// Objects of two mods whose models touch or cut a little into each other.
pub const OBJECTS_TOUCHING: KindInfo = KindInfo {
    id: "objects.touching",
    severity: Severity::Info,
    confidence: Confidence::Hint,
    args: &["first", "second", "count", "examples"],
};

/// Meshes of a mod are in the format of another game or of another edition
/// of this one. A note and a hint: many such meshes work, and well-kept
/// mods ship hundreds of them; they are a lead when something crashes or
/// stays invisible.
pub const MESHES_MADE_FOR_OTHER_GAME: KindInfo = KindInfo {
    id: "meshes.made-for-other-game",
    severity: Severity::Info,
    confidence: Confidence::Hint,
    args: &["count", "skinned", "madeFor", "examples"],
};

/// Meshes of a mod name textures that nothing in the game provides. A
/// hint: a mesh that is shown looks wrong, but a mod may bring meshes the
/// game never shows.
pub const MESHES_TEXTURE_MISSING: KindInfo = KindInfo {
    id: "meshes.texture-missing",
    severity: Severity::Warning,
    confidence: Confidence::Hint,
    args: &["meshes", "count", "examples"],
};

/// Replacement meshes no longer use textures that a texture mod replaces.
pub const MESHES_RETEXTURE_BYPASSED: KindInfo = KindInfo {
    id: "meshes.retexture-bypassed",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["count", "examples", "meshOrigin", "meshMod", "textureMod"],
};

/// Loose files replace files inside another mod's archive.
pub const ASSETS_LOOSE_OVER_ARCHIVE: KindInfo = KindInfo {
    id: "assets.loose-over-archive",
    severity: Severity::Info,
    confidence: Confidence::Certain,
    args: &[
        "count",
        "examples",
        "winnerOrigin",
        "winnerMod",
        "archive",
        "archiveMod",
    ],
};

/// An archive replaces files inside another mod's archive.
pub const ASSETS_ARCHIVE_OVER_ARCHIVE: KindInfo = KindInfo {
    id: "assets.archive-over-archive",
    severity: Severity::Info,
    confidence: Confidence::Certain,
    args: &[
        "count",
        "examples",
        "winnerArchive",
        "winnerOrigin",
        "winnerMod",
        "archive",
        "archiveMod",
    ],
};

/// A file of the setup could not be read.
pub const SETUP_FILE_UNREADABLE: KindInfo = KindInfo {
    id: "setup.file-unreadable",
    severity: Severity::Warning,
    confidence: Confidence::Certain,
    args: &["file", "reason"],
};

/// The load order could not be determined, so most checks did not run.
pub const SETUP_LOAD_ORDER_UNKNOWN: KindInfo = KindInfo {
    id: "setup.load-order-unknown",
    severity: Severity::Warning,
    confidence: Confidence::Certain,
    args: &["reason"],
};

/// A later plugin replaces navigation meshes that an earlier plugin
/// changed, without having been made with it. A note, not a warning: large
/// setups have dozens of these, and most only show as a character taking a
/// detour.
pub const NAVMESHES_CHANGES_UNDONE: KindInfo = KindInfo {
    id: "navmeshes.changes-undone",
    severity: Severity::Info,
    confidence: Confidence::Likely,
    args: &["winner", "loser", "count", "places"],
};

/// A later plugin replaces navigation meshes in which an earlier plugin
/// connects doors that mods add, and its version does not connect them.
pub const NAVMESHES_DOOR_CUT_OFF: KindInfo = KindInfo {
    id: "navmeshes.door-cut-off",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["winner", "loser", "count", "places"],
};

/// The recent crash reports, summed up.
pub const CRASHES_RECENT: KindInfo = KindInfo {
    id: "crashes.recent",
    severity: Severity::Info,
    confidence: Confidence::Certain,
    args: &["count", "days", "last", "examples"],
};

/// Recent crashes happened inside a program file that a mod brings.
pub const CRASHES_IN_DLL: KindInfo = KindInfo {
    id: "crashes.in-dll",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &["file", "count", "last"],
};

/// Crash reports name records, meshes or program files of a mod near the
/// crash.
pub const CRASHES_MOD_INVOLVED: KindInfo = KindInfo {
    id: "crashes.mod-involved",
    severity: Severity::Warning,
    confidence: Confidence::Hint,
    args: &["count", "last", "examples"],
};

/// Every kind, in the order the reference documentation lists them.
pub const ALL: &[KindInfo] = &[
    EXTENDER_PLUGIN_INCOMPATIBLE,
    EXTENDER_PLUGIN_NOT_LOADED,
    EXTENDER_PLUGIN_NO_VERSION_DATA,
    PLUGINS_MASTER_MISSING,
    PLUGINS_GENERATED_OUTDATED,
    PLUGINS_REQUIREMENT_MISSING,
    PLUGINS_INCOMPATIBLE,
    PLUGINS_NEEDS_CLEANING,
    PLUGINS_IDENTICAL_RECORDS,
    PLUGINS_LOOT_WARNING,
    PLUGINS_LOOT_NOTE,
    NPCS_FACE_MISMATCH,
    INSTALLER_OPTIONS_NOW_RECOMMENDED,
    PLUGINS_CONTAINED,
    OBJECTS_DUPLICATED,
    OBJECTS_OVERLAPPING,
    OBJECTS_TOUCHING,
    NAVMESHES_DOOR_CUT_OFF,
    NAVMESHES_CHANGES_UNDONE,
    MESHES_MADE_FOR_OTHER_GAME,
    MESHES_TEXTURE_MISSING,
    MESHES_RETEXTURE_BYPASSED,
    ASSETS_LOOSE_OVER_ARCHIVE,
    ASSETS_ARCHIVE_OVER_ARCHIVE,
    SETUP_FILE_UNREADABLE,
    CRASHES_IN_DLL,
    CRASHES_MOD_INVOLVED,
    CRASHES_RECENT,
    SETUP_LOAD_ORDER_UNKNOWN,
    MODS_COLLIDES_WIDELY,
    MODS_MOSTLY_OVERRIDDEN,
    MODS_DUPLICATES_ANOTHER,
    PATCH_HIDES_WALKWAY,
];

/// A mod whose placed objects collide with many spots of other mods: a
/// suggestion to remove it, or to let it lose against them everywhere
/// (ADR 0043).
pub const MODS_COLLIDES_WIDELY: KindInfo = KindInfo {
    id: "mods.collides-widely",
    severity: Severity::Info,
    confidence: Confidence::Hint,
    args: &[
        "mod",
        "count",
        "modCount",
        "others",
        "objects",
        "total",
        "interiors",
        "interiorObjects",
        "lootCount",
        "loot",
    ],
};

/// A mod most of whose content does not reach the game, because mods
/// loaded later replace it: a suggestion to remove it (ADR 0043).
pub const MODS_MOSTLY_OVERRIDDEN: KindInfo = KindInfo {
    id: "mods.mostly-overridden",
    severity: Severity::Info,
    confidence: Confidence::Hint,
    args: &["mod", "total", "surviving", "percent", "overriders"],
};

/// A mod whose placed objects another mod places as well, largely at the
/// same places: a suggestion to keep only one of the two (ADR 0043).
pub const MODS_DUPLICATES_ANOTHER: KindInfo = KindInfo {
    id: "mods.duplicates-another",
    severity: Severity::Info,
    confidence: Confidence::Hint,
    args: &[
        "mod",
        "other",
        "count",
        "total",
        "patchCount",
        "patches",
        "otherPatchCount",
        "otherPatches",
        "lootCount",
        "loot",
    ],
};

/// Mod Mallet's patch hides objects that the paths characters walk on run
/// over (ADR 0051).
pub const PATCH_HIDES_WALKWAY: KindInfo = KindInfo {
    id: "patch.hides-walkway",
    severity: Severity::Warning,
    confidence: Confidence::Likely,
    args: &[
        "plugin",
        "count",
        "objects",
        "places",
        "placeCount",
        "triangles",
        "navmeshes",
    ],
};

/// Looks a kind up by its identifier.
pub fn find(id: &str) -> Option<&'static KindInfo> {
    ALL.iter().find(|kind| kind.id == id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn identifiers_are_unique() {
        let unique: HashSet<_> = ALL.iter().map(|kind| kind.id).collect();
        assert_eq!(unique.len(), ALL.len());
    }

    #[test]
    fn identifiers_follow_the_naming_scheme() {
        for kind in ALL {
            let (area, what) = kind
                .id
                .split_once('.')
                .unwrap_or_else(|| panic!("{} has no area", kind.id));
            for part in [area, what] {
                assert!(!part.is_empty(), "{} has an empty part", kind.id);
                assert!(
                    part.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{} must be lower-case kebab-case",
                    kind.id
                );
            }
        }
    }

    #[test]
    fn find_returns_the_declared_kind() {
        assert_eq!(
            find("extender.plugin-incompatible"),
            Some(&EXTENDER_PLUGIN_INCOMPATIBLE)
        );
        assert_eq!(find("does.not-exist"), None);
    }
}
