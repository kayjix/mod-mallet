//! Where characters walk in an area, and who made each part of it, for the
//! paths mode of the 3D views (ADR 0071): the game's own paths, those a mod
//! added or changed, those Mod Mallet's plugin of paths repaired, and those
//! the current choices would leave over nothing. Where characters sit,
//! lean, sleep or work comes with the objects of the area (ADR 0069).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectRef;

/// The paths of the cells around a point of a worldspace, or of an
/// interior cell.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AreaPaths {
    /// The worldspace or interior cell asked about.
    pub place: String,
    /// Outside, the cells read, as column and row; inside, empty.
    pub cells: Vec<[i32; 2]>,
    /// The plugins that made a version of a navigation mesh here, in load
    /// order. Triangles name their maker by its place in this list.
    pub makers: Vec<PathsMaker>,
    /// The navigation meshes, in the version the game uses.
    pub navmeshes: Vec<AreaNavmesh>,
    /// Where the objects asked about would leave paths over nothing: the
    /// paths the game uses that run over them.
    pub over_hidden: Vec<PathsOverHidden>,
}

/// A plugin that made a version of a navigation mesh in the area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathsMaker {
    /// The plugin, in the spelling of the load order.
    pub plugin: String,
    /// Who it is.
    pub kind: MakerKind,
}

/// Who made a version of a navigation mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MakerKind {
    /// The game itself: one of its own plugins.
    Game,
    /// A mod.
    Mod,
    /// Mod Mallet's plugin of repaired paths (ADR 0058).
    Repair,
}

/// One navigation mesh in the version the game uses, with who made each
/// of its triangles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AreaNavmesh {
    /// The mesh, by the plugin that first defined it, as `plugin:number`.
    pub navmesh: String,
    /// Where it lies, for people: an interior cell's internal name, or the
    /// worldspace and the cell's grid position.
    pub cell: Option<String>,
    /// The plugins with a version of it, in load order, by their place in
    /// [`AreaPaths::makers`]; the last is the one the game uses.
    pub versions: Vec<u16>,
    /// The corners of its triangles: three whole game units each.
    pub vertices: Vec<i32>,
    /// Its triangles: three corner numbers each.
    pub triangles: Vec<u32>,
    /// For each triangle, who made it as it is, by its place in
    /// [`AreaPaths::makers`]: the first plugin from which on every version
    /// has it unchanged.
    pub made_by: Vec<u16>,
    /// The triangles the version the game uses takes out of the paths:
    /// still there, but leading nowhere, as a repair leaves them.
    pub taken_out: Vec<u32>,
    /// How many triangles of the game's own last version this one no
    /// longer has in the same place.
    pub gone: u32,
    /// Where characters step through a door: the middle of each door
    /// triangle, in whole game units.
    pub doors: Vec<[i32; 3]>,
    /// Where the paths lead on into a neighbouring mesh: the middle of each
    /// linked edge, in whole game units.
    pub links: Vec<[i32; 3]>,
}

/// The paths the game uses over one object asked about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathsOverHidden {
    /// The object, as it was asked about.
    pub object: ObjectRef,
    /// The mesh, as in [`AreaNavmesh::navmesh`].
    pub navmesh: String,
    /// The triangles over it, by their numbers in that mesh. Triangles
    /// already taken out are left out.
    pub triangles: Vec<u32>,
}
