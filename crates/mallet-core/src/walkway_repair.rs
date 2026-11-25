//! What repairing the paths over hidden objects does, for the window to
//! show before the user chooses it (ADR 0058).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The prefix of the patch choices that repair a navigation mesh: the
/// choice `repair:<navmesh>`, with the mesh as in [`RepairedPaths::navmesh`],
/// asks for the repair of that mesh. A writer that does not know a prefix
/// leaves the choice alone.
pub const REPAIR_CHOICE_PREFIX: &str = "repair:";

/// The name of the plugin, and of the mod, that carries the repaired paths
/// (ADR 0058); the plugin is `Mod Mallet - Paths.esp`.
pub const PATHS_NAME: &str = "Mod Mallet - Paths";

/// The repairs of paths to write into the plugin of paths: where, and the
/// objects the patch hides, whose paths are repaired.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathRepairs {
    /// Each place with meshes to repair.
    pub places: Vec<RepairPlace>,
    /// Every object the patch hides. The repair of a place takes those
    /// that stand there, so that it is the same whichever choice asked
    /// for it.
    pub hidden: Vec<crate::ObjectRef>,
}

/// The meshes to repair in one place.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RepairPlace {
    /// The worldspace, or the interior cell by its editor ID.
    pub place: String,
    /// Outside, a point near the objects, in whole game units; inside,
    /// `None`.
    pub centre: Option<[i32; 2]>,
    /// The meshes chosen for repair there, as in [`RepairedPaths::navmesh`].
    pub navmeshes: Vec<String>,
    /// Whether every mesh there whose path the game uses runs over a hidden
    /// object is repaired, not only those chosen: a patch repairs the paths
    /// over what it hides by itself (ADR 0070).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub every: bool,
    /// Door triangles there that are to name another door: those that
    /// named a door the patch hides because stacked doors were merged into
    /// the kept one (ADR 0067, ADR 0088). They are written into the plugin
    /// of paths with the repairs, in the same copies of the meshes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doors: Vec<DoorRelink>,
}

impl RepairPlace {
    /// Whether anything is to be repaired or re-attached there.
    pub fn wanted(&self) -> bool {
        self.every || !self.navmeshes.is_empty() || !self.doors.is_empty()
    }
}

/// A door triangle of a navigation mesh that is to name another door: a
/// path of the characters re-attached from a hidden door to the door kept
/// in its place (ADR 0088).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorRelink {
    /// The navigation mesh, by the plugin that first defined it.
    pub navmesh: crate::ObjectRef,
    /// The triangle, by its number in the mesh.
    pub triangle: u32,
    /// The door it names now: the one the patch hides.
    pub from: crate::ObjectRef,
    /// The door it is to name: the one that stays.
    pub to: crate::ObjectRef,
}

/// The plugin of paths as written into Mod Mallet's own folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedPaths {
    /// The archive that holds it, ready for the mod manager; empty if
    /// nothing was written.
    pub path: String,
    /// The meshes it repairs.
    pub navmeshes: Vec<String>,
    /// The plugins it names as masters. It loads after them.
    pub masters: Vec<String>,
    /// Whether it is flagged as light.
    pub light: bool,
    /// The meshes chosen that it does not repair, and why.
    pub refused: Vec<RefusedPath>,
    /// The door triangles it re-attaches from hidden doors to the doors
    /// kept in their place (ADR 0088).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relinked: Vec<DoorRelink>,
    /// The re-attachments asked for that it does not make: a repair took
    /// the triangle out, or the mesh, the triangle or the door was not
    /// found.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_relinked: Vec<DoorRelink>,
}

/// A mesh that was chosen for repair but is not repaired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefusedPath {
    /// The mesh, as in [`RepairedPaths::navmesh`].
    pub navmesh: String,
    /// Why.
    pub reason: PathRefusal,
}

/// Why a mesh is not repaired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum PathRefusal {
    /// Nothing over hidden objects runs on it any more, or it was not
    /// found where the repair was asked for.
    NothingToRepair,
    /// Its cell or worldspace has a field Mod Mallet does not know how to
    /// copy.
    UnknownField {
        /// The record type and the field, such as `CELL XCWT`.
        field: String,
    },
    /// Its cell or worldspace was not found, or names a plugin the patch
    /// cannot name.
    NotFound,
    /// The game is not one whose plugins Mod Mallet can write.
    UnsupportedGame,
}

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
