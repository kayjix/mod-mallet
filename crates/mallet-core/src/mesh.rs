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
