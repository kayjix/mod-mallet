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
