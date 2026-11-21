//! What repairing the paths over hidden objects does, for the window to
//! show before the user chooses it (ADR 0058).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The prefix of the patch choices that repair a navigation mesh: the
/// choice `repair:<navmesh>`, with the mesh as in [`RepairedPaths::navmesh`],
/// asks for the repair of that mesh. A writer that does not know a prefix
/// leaves the choice alone.
pub const REPAIR_CHOICE_PREFIX: &str = "repair:";

/// The navigation meshes the user chose to repair, from the window's patch
/// choices: the keys with [`REPAIR_CHOICE_PREFIX`] that have a value.
pub fn repair_choices<'a>(
    choices: impl IntoIterator<Item = (&'a String, &'a Option<String>)>,
) -> std::collections::BTreeSet<String> {
    choices
        .into_iter()
        .filter(|(_, value)| value.is_some())
        .filter_map(|(key, _)| key.strip_prefix(REPAIR_CHOICE_PREFIX))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn only_repair_choices_with_a_value_count() {
        let choices = BTreeMap::from([
            (
                "repair:skyrim.esm:102F4F".to_owned(),
                Some("repair".to_owned()),
            ),
            ("repair:skyrim.esm:1046FA".to_owned(), None),
            ("spot:abc".to_owned(), Some("A.esp".to_owned())),
        ]);
        assert_eq!(
            repair_choices(&choices).into_iter().collect::<Vec<_>>(),
            vec!["skyrim.esm:102F4F".to_owned()]
        );
    }
}

/// What repairing the paths over objects about to be hidden does: the
/// triangles of the paths taken out, and those laid on the ground below
/// instead.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WalkwayRepair {
    /// The worldspace or interior cell asked about.
    pub place: String,
    /// Each navigation mesh the repair changes.
    pub navmeshes: Vec<RepairedPaths>,
    /// How many areas of new paths are laid where the objects stood, each
    /// joined to the paths around.
    pub areas_laid: u32,
    /// How many of the objects that carried paths get none below them:
    /// characters go around them.
    pub bare_objects: u32,
    /// The doors characters could no longer step through, by plugin and
    /// number.
    pub lost_doors: Vec<String>,
}

/// One navigation mesh as the repair changes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RepairedPaths {
    /// The mesh, by the plugin that first defined it and its number there,
    /// such as `skyrim.esm:102F4F`.
    pub navmesh: String,
    /// The plugin of the version the game uses, which the repair copies.
    pub plugin: String,
    /// Where it lies, for people.
    pub cell: Option<String>,
    /// The triangles taken out, three corners each, in whole game units,
    /// in the coordinates of the worldspace or the cell.
    pub removed: Vec<[[i32; 3]; 3]>,
    /// The new triangles, likewise.
    pub added: Vec<[[i32; 3]; 3]>,
    /// How many of the new triangles lie under water.
    pub water: u32,
    /// Whether the repaired record could be built from the plugin's own
    /// version; if not, the mesh cannot be repaired.
    pub writable: bool,
}
