//! Whether characters still reach where they need to go once objects are
//! hidden and the paths over them are cut out or repaired (ADR 0059).
//!
//! The places characters walk to are destinations: doors, furniture to sit,
//! sleep or work at, the spots where they stand idle, map markers, and
//! objects a quest or a character's routine names. Each lies on a triangle
//! of the navigation meshes. Before and after a change of the paths, the
//! way to each destination is followed: one that no longer leads there is
//! cut off, one that leads there only by a much longer way is a detour.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectRef;

/// What kind of place characters walk to.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum DestinationKind {
    /// A door, which may lead into another cell.
    Door,
    /// Furniture: a bed, a chair, a work station.
    Furniture,
    /// A spot where characters stand or do something, an idle marker.
    IdleMarker,
    /// A place of the world map.
    MapMarker,
    /// An object a quest or a character's routine sends characters to.
    Used,
}

/// What hiding objects, with the paths over them cut out or repaired, does
/// to the ways characters take near them (ADR 0059).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WalkwayReach {
    /// The worldspace or interior cell asked about.
    pub place: String,
    /// What the paths are compared with: the paths over the objects cut
    /// out (ADR 0052), or repaired.
    pub after: ReachAfter,
    /// How many destinations near the objects were checked.
    pub checked: u32,
    /// The destinations whose way changes, the worst first: cut off before
    /// longer ways.
    pub changes: Vec<ReachChange>,
}

/// Whether characters reach the door kept where stacked doors are merged,
/// and step through it, on the paths as the patch leaves them: the paths
/// over what it hides repaired, the door triangles of the hidden doors
/// re-attached to the kept one, the doors on the other side leading to it
/// (ADR 0088).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorReach {
    /// The worldspace or interior cell asked about.
    pub place: String,
    /// The kept door.
    pub door: ObjectRef,
    /// How many door triangles of the hidden doors were found and are
    /// re-attached to it.
    pub relinked: u32,
    /// How many triangles of the paths name it then.
    pub triangles: u32,
    /// Whether a way leads to it from the rest of the paths.
    pub reached: bool,
    /// Whether characters step through it to the other side.
    pub through: bool,
    /// Whether the paths around were read at all; if not, nothing above
    /// is known.
    pub known: bool,
}

/// What the paths after hiding are.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReachAfter {
    /// The triangles over the hidden objects are taken out of the paths
    /// (ADR 0052): what characters would walk on if nothing is added.
    #[default]
    Cut,
    /// The paths are repaired with new triangles around or under the
    /// hidden objects.
    Repaired,
}

/// How the way to one destination changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReachChange {
    /// What happens to it.
    pub outcome: ReachOutcome,
    /// Where characters want to go.
    pub destination: Destination,
    /// For a longer way: where the way is measured from, another
    /// destination nearby.
    pub from: Option<Destination>,
    /// The way before. For a destination cut off, the shortest way from
    /// where the paths lead on into the rest of the world.
    pub before: Option<Route>,
    /// The way after, if there is one.
    pub after: Option<Route>,
}

/// What happens to the way to a destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReachOutcome {
    /// What characters stood on to use it is gone with the hidden object.
    Gone,
    /// No way leads there any more.
    CutOff,
    /// A way still leads there, but a much longer one, or one only
    /// through the places around that were not looked at.
    Longer,
}

/// A place characters walk to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    /// The placed object.
    pub object: ObjectRef,
    /// What kind of place it is.
    pub kind: DestinationKind,
    /// Its name: the one players see where known, else the internal one.
    pub name: Option<String>,
    /// For an object a quest or routine uses: that quest or routine, by its
    /// internal name.
    pub used_by: Option<String>,
    /// Where it stands, in whole game units.
    pub position: [i32; 3],
    /// Where it lies, for people: an interior cell's internal name, or the
    /// worldspace and the cell's grid position.
    pub cell: Option<String>,
}

/// A way characters walk, to draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Route {
    /// How long it is, in whole metres.
    pub metres: u32,
    /// Its points from start to end, in whole game units, in the
    /// coordinates of the worldspace or the interior cell.
    pub points: Vec<[i32; 3]>,
}
