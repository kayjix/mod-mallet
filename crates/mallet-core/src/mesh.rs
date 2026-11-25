//! A mesh ready to be drawn in the window, from the user's own files.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One version of a mesh of the setup, as the window draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeshView {
    /// The mesh, below the data directory.
    pub path: String,
    /// The archive the version is in, or `None` for a loose file.
    pub archive: Option<String>,
    /// Its surfaces.
    pub shapes: Vec<MeshShape>,
    /// For a face, the face texture that colours the head, as a PNG
    /// `data:` address.
    pub tint: Option<String>,
    /// For a mesh with nothing to draw but bones, such as a skeleton, the
    /// bones as lines: six 32-bit floats each (where it starts, where it
    /// ends), little-endian and Base64-encoded as in [`MeshShape`]. Empty
    /// when the mesh has surfaces.
    #[serde(default)]
    pub bones: String,
}

/// One surface of a mesh. Numbers are little-endian and Base64-encoded,
/// which keeps a face of a few thousand points small.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeshShape {
    /// The points: three 32-bit floats each.
    pub positions: String,
    /// The texture coordinates: two 32-bit floats per point, or empty.
    pub uvs: String,
    /// The triangles: three 32-bit point numbers each.
    pub indices: String,
    /// The colour texture as a PNG `data:` address, if it could be read.
    pub texture: Option<String>,
    /// Whether the face texture colours this surface: the head itself.
    pub tinted: bool,
    /// Whether the transparent parts of the texture are left out, as
    /// between strands of hair.
    #[serde(default)]
    pub alpha_test: bool,
}

/// The level-of-detail tiles of a worldspace that the setup has: the
/// blocks of land the game shows from a distance (ADR 0037).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LodTiles {
    /// The worldspace, by its internal name.
    pub world: String,
    /// The tiles, coarsest first.
    pub tiles: Vec<LodTile>,
}

/// One block of level of detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LodTile {
    /// How many cells the block covers in each direction: 4, 8, 16 or 32.
    pub level: u32,
    /// The cell column at its south-west corner.
    pub x: i32,
    /// The cell row at its south-west corner.
    pub y: i32,
    /// Whether the block has land.
    pub terrain: bool,
    /// Whether the block has objects: towns, castles, bridges, rocks.
    pub objects: bool,
}

/// The surfaces of one block of level of detail, in game units. Numbers
/// are little-endian and Base64-encoded, as in [`MeshShape`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LodMesh {
    /// Its surfaces.
    pub shapes: Vec<LodShape>,
}

/// One surface of a block of level of detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LodShape {
    /// The points: three 32-bit floats each.
    pub positions: String,
    /// The texture coordinates: two 32-bit floats per point, or empty.
    pub uvs: String,
    /// The triangles: three 32-bit point numbers each.
    pub indices: String,
    /// The colour texture, below the data directory, to be fetched once
    /// for every surface that uses it. A surface of land without one is
    /// water.
    pub texture: Option<String>,
    /// Whether the transparent parts of the texture are left out.
    pub alpha_test: bool,
}

/// Everything placed around a point of a worldspace, as the game shows
/// it, for looking at the place closely (ADR 0038); or everything placed
/// in one interior cell (ADR 0042).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AreaObjects {
    /// The worldspace, by its internal name; for an interior cell, the
    /// cell's editor ID.
    pub world: String,
    /// The point, in game units; for an interior cell, the middle of what
    /// is placed in it.
    pub centre: [f32; 2],
    /// How far around the point, in game units, in each direction.
    pub radius: f32,
    /// The objects, with a model, in no particular order.
    pub objects: Vec<AreaObject>,
    /// The places where characters do something, drawn or not (ADR 0069),
    /// in no particular order.
    #[serde(default)]
    pub actions: Vec<crate::action_places::ActionPlace>,
}

/// One object near the point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AreaObject {
    /// The reference: the plugin that placed it first and its number there.
    pub id: crate::ObjectRef,
    /// The plugin whose version the game uses.
    pub plugin: String,
    /// Its model, below the data directory.
    pub model: String,
    /// The name players see, or else the internal one.
    pub name: Option<String>,
    /// Where, in game units.
    pub position: [f32; 3],
    /// Turned about the x, y and z axes, in radians.
    pub rotation: [f32; 3],
    /// The size relative to the base object.
    pub scale: f32,
    /// Whether the game relies on it being there: it is persistent (other
    /// things refer to it) or part of the distant view. Such an object is
    /// not offered for hiding.
    #[serde(default)]
    pub anchored: bool,
}

/// The game's own ground around a point of a worldspace, cell by cell, for
/// drawing the land under the objects up close (ADR 0041).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AreaLand {
    /// The worldspace, by its internal name.
    pub world: String,
    /// The outside cells with land, from the south-west, row by row.
    pub cells: Vec<LandCell>,
}

/// The ground of one outside cell. Numbers are little-endian and
/// Base64-encoded, as in [`MeshShape`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LandCell {
    /// The cell's column.
    pub x: i32,
    /// The cell's row.
    pub y: i32,
    /// The heights of 33 rows of 33 points, 128 game units apart, as
    /// 32-bit floats in game units: the southern row first, each from west
    /// to east. Neighbouring cells share their edge.
    pub heights: String,
    /// The colours of the same points, three bytes (red, green, blue)
    /// each, or empty.
    pub colours: String,
    /// The textures of its four quarters, south-west, south-east,
    /// north-west, north-east (ADR 0047); empty where the land names none.
    #[serde(default)]
    pub quarters: Vec<LandQuarter>,
}

/// The textures of one quarter of a cell, as the game paints it: a base
/// and layers over it, each showing as much as its opacity at each point
/// says (ADR 0047).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LandQuarter {
    /// The colour texture underneath, below the data directory, if any.
    pub base: Option<String>,
    /// The layers over it, the lowest first.
    pub layers: Vec<LandTextureLayer>,
}

/// One layer of texture on a quarter of a cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LandTextureLayer {
    /// The colour texture, below the data directory.
    pub texture: String,
    /// How much of it shows at each of the 17 × 17 points of the quarter,
    /// one byte each (0 to 255), the southern row first, each from west to
    /// east; Base64-encoded. The quarter's points are the cell's, from its
    /// corner to its middle.
    pub opacity: String,
}

/// The paths characters walk on that run over objects about to be hidden
/// (ADR 0051): hiding an object through Base Object Swapper takes away
/// what characters stand on, but not the paths they follow over it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HiddenWalkways {
    /// The worldspace or interior cell asked about, as in
    /// [`AreaObjects::world`].
    pub place: String,
    /// The objects asked about that were found there.
    pub found: Vec<crate::ObjectRef>,
    /// Each version of a navigation mesh that runs over one of them.
    pub walkways: Vec<Walkway>,
    /// What taking the paths over them out of the versions the game uses
    /// would leave (ADR 0052).
    pub repair: PathRepair,
}

/// What taking the paths over hidden objects out of the navigation meshes
/// would leave: whether the game's characters still find a way to every
/// part of the paths that is left (ADR 0052).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathRepair {
    /// How many triangles would be taken out, of every version the game
    /// uses.
    pub triangles: u32,
    /// Whether every part of the paths that is left stays reachable, and
    /// no door loses its way through: then the repair is safe.
    pub safe: bool,
    /// The parts of the paths nobody could reach any more, the largest
    /// first.
    pub cut_off: Vec<CutOffPath>,
    /// The doors characters could no longer step through, because their
    /// paths lie on what is hidden: by plugin and number, as
    /// [`Walkway::navmesh`].
    pub lost_doors: Vec<String>,
}

/// A part of the paths that nobody could reach any more once the paths
/// over hidden objects are taken out, such as a jetty beyond a bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CutOffPath {
    /// Where it lies, for people, as [`Walkway::cell`].
    pub cell: Option<String>,
    /// Its middle, in whole game units, in the coordinates of the
    /// worldspace or the cell.
    pub centre: [i32; 3],
    /// How large it is, in square metres.
    pub square_metres: u32,
    /// The doors in it, which characters could no longer reach, as in
    /// [`PathRepair::lost_doors`].
    pub doors: Vec<String>,
}

/// One plugin's version of a navigation mesh that runs over a hidden
/// object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Walkway {
    /// The object it runs over.
    pub object: crate::ObjectRef,
    /// The navigation mesh, by the plugin that first defined it and its
    /// number there, such as `skyrim.esm:102F4F`.
    pub navmesh: String,
    /// The plugin of this version, as the load order spells it.
    pub plugin: String,
    /// Whether the game uses this version. A version another plugin
    /// replaces does no harm.
    pub used: bool,
    /// Where the navigation mesh lies, for people: an interior cell's
    /// internal name, or the worldspace and the cell's grid position.
    pub cell: Option<String>,
    /// How many of its triangles lie on the object.
    pub triangles: u32,
}

/// One version of a mesh asked for as a small picture in a list
/// (ADR 0085).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeshThumbnailRequest {
    /// The mesh, below the data directory.
    pub path: String,
    /// The version, as the catalogue names versions (`AssetVersion::id`);
    /// `None` for the one the game uses.
    #[serde(default)]
    pub version: Option<String>,
}

/// A small picture of one version of a mesh, seen from a fixed angle
/// under a neutral light, or why there is none (ADR 0085).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MeshThumbnail {
    /// The mesh, as asked for.
    pub path: String,
    /// The version, as asked for.
    #[serde(default)]
    pub version: Option<String>,
    /// The picture, as a PNG `data:` address with a transparent
    /// background; `None` when there is nothing to show.
    pub image: Option<String>,
    /// Why there is no picture; `None` when there is one.
    pub gap: Option<MeshGap>,
}

/// Why a mesh has no picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum MeshGap {
    /// The setup has no such version of the file.
    Missing,
    /// The file is not a mesh Mod Mallet can read: damaged, or of a
    /// version it does not know.
    Unreadable,
    /// A skeleton: bones and nothing to draw around them.
    Skeleton,
    /// Light, not a body: glow, mist, flames or particles, which the game
    /// makes in its own way.
    Effects,
    /// Nothing the game shows: a marker, a camera or an empty node.
    Empty,
    /// It has surfaces of a kind Mod Mallet cannot draw yet.
    Unsupported,
}
