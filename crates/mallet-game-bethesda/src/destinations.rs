//! The places characters walk to, read alongside the navigation meshes of
//! chosen cells (ADR 0059): doors, furniture, idle markers and map markers.
//!
//! A placed object (`REFR`) names what it places in `NAME` and where in
//! `DATA`. A door that leads into another cell names the door on the other
//! side in the first four bytes of `XTEL`; a map marker carries an empty
//! `XMRK` field and its name in `FULL`. What is placed is a door (`DOOR`),
//! furniture such as a bed, a chair or a work station (`FURN`), or an idle
//! marker (`IDLM`), a spot where characters stand or do something.
//!
//! Later plugins replace earlier ones' versions, and a version that
//! deletes an object or switches it off at the start takes it away.

use std::collections::HashMap;

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::PlacedDestination;
use mallet_core::facts::objects::Place;
use mallet_core::reach::DestinationKind;
use mallet_formats_bethesda::plugin::{self, Record, Signature};

use crate::objects::{FLAG_DELETED, FLAG_INITIALLY_DISABLED, editor_id, floats, record_id};

/// The base records that make a placed object a destination.
pub(crate) const BASES: [Signature; 3] = [*b"DOOR", *b"FURN", *b"IDLM"];

/// Positions farther out than this are not real places.
const MAX_COORDINATE: f32 = 1.0e7;

/// What is known of a base object that makes a destination.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Base {
    kind: DestinationKind,
    name: Option<String>,
}

/// One placed object, in the last version read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Placed {
    plugin: String,
    base: RecordId,
    place: Place,
    position: [i32; 3],
    leads_to: Option<RecordId>,
    map_marker: bool,
    name: Option<String>,
}

/// Collects base objects and placed objects in load order, and makes
/// destinations of them once every plugin is read.
#[derive(Debug, Default)]
pub(crate) struct Collector {
    bases: HashMap<RecordId, Option<Base>>,
    placed: HashMap<RecordId, Option<Placed>>,
}

impl Collector {
    /// Takes in a base record of one of the [`BASES`] types.
    pub(crate) fn base(
        &mut self,
        record: &Record,
        masters: &[String],
        plugin_name: &str,
        localized: bool,
    ) {
        let id = record_id(record.form_id, masters, plugin_name);
        let kind = match &record.signature {
            b"DOOR" => DestinationKind::Door,
            b"FURN" => DestinationKind::Furniture,
            b"IDLM" => DestinationKind::IdleMarker,
            _ => return,
        };
        let base = (record.flags & FLAG_DELETED == 0).then(|| Base {
            kind,
            name: shown_name(record, localized).or_else(|| editor_id(record)),
        });
        self.bases.insert(id, base);
    }

    /// Takes in a placed object (`REFR`) in `place`, if `keep` accepts
    /// where it stands. A version that deletes or switches off an object
    /// known so far, or moves it away, takes it away.
    pub(crate) fn placed(
        &mut self,
        record: &Record,
        masters: &[String],
        plugin_name: &str,
        localized: bool,
        place: Place,
        keep: impl Fn([f32; 3]) -> bool,
    ) {
        let id = record_id(record.form_id, masters, plugin_name);
        let found = placed(record, masters, plugin_name, localized, place)
            .filter(|_| record.flags & (FLAG_DELETED | FLAG_INITIALLY_DISABLED) == 0)
            .filter(|(_, at)| keep(*at));
        match found {
            Some((placed, _)) => {
                self.placed.insert(id, Some(placed));
            }
            None => {
                if let Some(known) = self.placed.get_mut(&id) {
                    *known = None;
                }
            }
        }
    }

    /// The destinations among what was read, sorted by identity.
    pub(crate) fn finish(self) -> Vec<PlacedDestination> {
        let mut found: Vec<PlacedDestination> = self
            .placed
            .into_iter()
            .filter_map(|(id, placed)| {
                let placed = placed?;
                let base = self.bases.get(&placed.base).and_then(Option::as_ref);
                let kind = if placed.map_marker {
                    DestinationKind::MapMarker
                } else {
                    base?.kind
                };
                let name = placed
                    .name
                    .or_else(|| base.and_then(|base| base.name.clone()));
                Some(PlacedDestination {
                    id,
                    plugin: placed.plugin,
                    kind,
                    name,
                    place: placed.place,
                    position: placed.position,
                    leads_to: placed.leads_to,
                })
            })
            .collect();
        found.sort_by(|a, b| a.id.cmp(&b.id));
        found
    }
}

/// A placed object as a version writes it, with where it stands in game
/// units; `None` if it names nothing or stands nowhere.
fn placed(
    record: &Record,
    masters: &[String],
    plugin_name: &str,
    localized: bool,
    place: Place,
) -> Option<(Placed, [f32; 3])> {
    let base = u32::from_le_bytes(record.first(b"NAME")?.get(..4)?.try_into().ok()?);
    let data = floats(record.first(b"DATA")?.get(..12)?);
    let [x, y, z] = data.as_slice() else {
        return None;
    };
    let at = [*x, *y, *z];
    if at
        .iter()
        .any(|value| !value.is_finite() || value.abs() > MAX_COORDINATE)
    {
        return None;
    }
    let leads_to = record
        .first(b"XTEL")
        .and_then(|data| data.get(..4))
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .filter(|door| *door != 0)
        .map(|door| record_id(door, masters, plugin_name));
    let map_marker = record.first(b"XMRK").is_some();
    Some((
        Placed {
            plugin: plugin_name.to_owned(),
            base: record_id(base, masters, plugin_name),
            place,
            position: at.map(whole),
            leads_to,
            map_marker,
            name: if map_marker {
                shown_name(record, localized)
            } else {
                None
            },
        },
        at,
    ))
}

/// The name players see, where the plugin spells it out rather than
/// numbering it in a string table.
fn shown_name(record: &Record, localized: bool) -> Option<String> {
    (!localized)
        .then(|| record.first(b"FULL").map(plugin::zstring))
        .flatten()
        .filter(|name| !name.is_empty())
}

/// A coordinate in whole game units.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> i32 {
    // Bounded by MAX_COORDINATE before converting.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::plugin::Subrecord;
    use proptest::prelude::*;

    use super::*;

    fn record(signature: Signature, form_id: u32, fields: Vec<(&[u8; 4], Vec<u8>)>) -> Record {
        Record {
            signature,
            form_id,
            flags: 0,
            subrecords: fields
                .into_iter()
                .map(|(signature, data)| Subrecord {
                    signature: *signature,
                    data,
                })
                .collect(),
            groups: Vec::new(),
        }
    }

    fn position(x: f32, y: f32, z: f32) -> Vec<u8> {
        [x, y, z, 0.0, 0.0, 0.0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn reference(form_id: u32, base: u32, extra: Vec<(&[u8; 4], Vec<u8>)>) -> Record {
        let mut fields = vec![
            (b"NAME", base.to_le_bytes().to_vec()),
            (b"DATA", position(100.0, 200.0, 10.0)),
        ];
        fields.extend(extra);
        record(*b"REFR", form_id, fields)
    }

    fn world() -> Place {
        Place::Exterior {
            worldspace: RecordId {
                plugin: "main.esm".to_owned(),
                local_id: 0x3C,
            },
        }
    }

    fn id(local_id: u32) -> RecordId {
        RecordId {
            plugin: "main.esm".to_owned(),
            local_id,
        }
    }

    #[test]
    fn doors_furniture_and_map_markers_are_destinations() {
        let masters: Vec<String> = Vec::new();
        let mut collector = Collector::default();
        let bed = record(
            *b"FURN",
            0x10,
            vec![
                (b"EDID", b"BedDouble\0".to_vec()),
                (b"FULL", b"Bed\0".to_vec()),
            ],
        );
        let door = record(*b"DOOR", 0x11, vec![(b"EDID", b"HouseDoor\0".to_vec())]);
        let idle = record(*b"IDLM", 0x12, vec![(b"EDID", b"LeanMarker\0".to_vec())]);
        for base in [&bed, &door, &idle] {
            collector.base(base, &masters, "Main.esm", false);
        }
        let all = |_: [f32; 3]| true;
        let refs = [
            reference(0x100, 0x10, Vec::new()),
            reference(
                0x101,
                0x11,
                vec![(b"XTEL", vec![0x02, 0x01, 0, 0, 0, 0, 0, 0])],
            ),
            reference(0x102, 0x12, Vec::new()),
            // A rock is no destination.
            reference(0x103, 0x99, Vec::new()),
            reference(
                0x104,
                0x10,
                vec![(b"XMRK", Vec::new()), (b"FULL", b"Riverwood\0".to_vec())],
            ),
        ];
        for placed in &refs {
            collector.placed(placed, &masters, "Main.esm", false, world(), all);
        }
        let found = collector.finish();
        let summary: Vec<_> = found
            .iter()
            .map(|found| {
                (
                    found.id.local_id,
                    found.kind,
                    found.name.as_deref(),
                    found.leads_to.clone(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (0x100, DestinationKind::Furniture, Some("Bed"), None),
                (
                    0x101,
                    DestinationKind::Door,
                    Some("HouseDoor"),
                    Some(id(0x102))
                ),
                (0x102, DestinationKind::IdleMarker, Some("LeanMarker"), None),
                (0x104, DestinationKind::MapMarker, Some("Riverwood"), None),
            ]
        );
        assert_eq!(
            found.first().map(|found| found.position),
            Some([100, 200, 10])
        );
    }

    #[test]
    fn a_later_version_that_deletes_or_moves_away_takes_it_away() {
        let masters: Vec<String> = Vec::new();
        let mut collector = Collector::default();
        collector.base(
            &record(*b"FURN", 0x10, Vec::new()),
            &masters,
            "Main.esm",
            true,
        );
        let bed = reference(0x100, 0x10, Vec::new());
        let mut deleted = bed.clone();
        deleted.flags = FLAG_DELETED;
        collector.placed(&bed, &masters, "Main.esm", false, world(), |_| true);
        collector.placed(&deleted, &masters, "Main.esm", false, world(), |_| true);
        let chair = reference(0x101, 0x10, Vec::new());
        collector.placed(&chair, &masters, "Main.esm", false, world(), |_| true);
        collector.placed(&chair, &masters, "Main.esm", false, world(), |at| {
            at[0] > 500.0
        });
        assert_eq!(collector.finish(), Vec::new());
    }

    proptest! {
        #[test]
        fn never_panics(
            name in prop::collection::vec(any::<u8>(), 0..8),
            data in prop::collection::vec(any::<u8>(), 0..32),
            xtel in prop::collection::vec(any::<u8>(), 0..8),
            full in prop::collection::vec(any::<u8>(), 0..8),
        ) {
            let masters = vec!["Main.esm".to_owned()];
            let mut collector = Collector::default();
            collector.base(
                &record(*b"DOOR", 0x0100_0010, vec![(b"FULL", full.clone())]),
                &masters,
                "Mod.esp",
                false,
            );
            let placed = record(
                *b"REFR",
                0x0100_0100,
                vec![(b"NAME", name), (b"DATA", data), (b"XTEL", xtel), (b"XMRK", Vec::new()), (b"FULL", full)],
            );
            collector.placed(&placed, &masters, "Mod.esp", false, world(), |_| true);
            let _ = collector.finish();
        }
    }
}
