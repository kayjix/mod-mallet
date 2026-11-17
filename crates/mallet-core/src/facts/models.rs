//! The shapes of models, for telling whether two placed objects really
//! touch.
//!
//! The box around a model (ADR 0023) is a rough guess at its shape: an
//! arch or an L-shaped house leaves much of its box empty. Where two
//! objects of different mods meet by their boxes, the triangles of their
//! models decide (ADR 0039). Only the models of such objects are read.

use std::collections::BTreeMap;

/// The models whose shapes were read, by their path as the base objects
/// name them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelFacts {
    /// The shape of each model that could be read.
    pub meshes: BTreeMap<String, ModelMesh>,
}

/// The surface of a model: triangles between points, in the model's own
/// space and units, with every part of the model placed as the model
/// places it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelMesh {
    /// The points, in whole game units.
    pub points: Vec<[i32; 3]>,
    /// The triangles, three point numbers each.
    pub triangles: Vec<[u32; 3]>,
}
