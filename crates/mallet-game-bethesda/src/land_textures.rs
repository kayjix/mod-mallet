//! The textures of the land, as the game paints it (ADR 0047).
//!
//! Each quarter of an outside cell has a base texture (`BTXT`) and layers
//! on top of it (`ATXT`), each with the opacity of the layer at the points
//! of the quarter (`VTXT`). They name landscape textures (`LTEX`), which
//! name a texture set (`TXST`) whose first texture is the colour; older
//! games name the texture file in the landscape texture itself (`ICON`).
//! Only reads.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, Record};

use crate::game::Game;
use crate::ingest;
use crate::objects::record_id;

/// Points per side of a quarter of a cell: half of the cell's 33, sharing
/// the middle row and column.
pub const QUARTER_GRID: usize = 17;

/// The quarters of a cell in the order the game numbers them: south-west,
/// south-east, north-west, north-east.
pub const QUARTERS: usize = 4;

/// The largest number of layers kept on one quarter.
const MAX_LAYERS: usize = 16;

/// One quarter of a cell's ground.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Quarter {
    /// The landscape texture underneath, if the land names one.
    pub base: Option<RecordId>,
    /// The layers on top of it, the lowest first.
    pub layers: Vec<Layer>,
}

/// One layer of texture on a quarter of a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    /// The landscape texture.
    pub texture: RecordId,
    /// How much of it shows at each of the 17 × 17 points of the quarter,
    /// the southern row first, each from west to east; 0 for none, 255 for
    /// all.
    pub opacity: Vec<u8>,
}

/// The textures of the four quarters of the `LAND` record `record` of the
/// plugin `plugin_name` with the masters `masters`, south-west first. A
/// quarter the record says nothing about is empty.
pub fn quarters(record: &Record, masters: &[String], plugin_name: &str) -> Vec<Quarter> {
    let mut quarters = vec![Quarter::default(); QUARTERS];
    // Each `ATXT` is followed by the `VTXT` of its points.
    let mut current: Option<(usize, u16, Layer)> = None;
    let mut found: Vec<(usize, u16, Layer)> = Vec::new();
    for field in &record.subrecords {
        match &field.signature {
            b"BTXT" => {
                if let Some((texture, quarter)) = texture_and_quarter(&field.data)
                    && let Some(slot) = quarters.get_mut(quarter)
                {
                    slot.base = Some(record_id(texture, masters, plugin_name));
                }
            }
            b"ATXT" => {
                found.extend(current.take());
                if let Some((texture, quarter)) = texture_and_quarter(&field.data)
                    && quarter < QUARTERS
                {
                    let order = field
                        .data
                        .get(6..8)
                        .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
                        .map_or(0, u16::from_le_bytes);
                    current = Some((
                        quarter,
                        order,
                        Layer {
                            texture: record_id(texture, masters, plugin_name),
                            opacity: vec![0; QUARTER_GRID * QUARTER_GRID],
                        },
                    ));
                }
            }
            b"VTXT" => {
                if let Some((_, _, layer)) = current.as_mut() {
                    apply_opacities(&field.data, &mut layer.opacity);
                }
            }
            _ => {}
        }
    }
    found.extend(current.take());
    found.sort_by_key(|(quarter, order, _)| (*quarter, *order));
    for (quarter, _, layer) in found {
        if let Some(slot) = quarters.get_mut(quarter)
            && slot.layers.len() < MAX_LAYERS
            && layer.opacity.iter().any(|value| *value > 0)
        {
            slot.layers.push(layer);
        }
    }
    quarters
}

/// The landscape texture and the quarter of a `BTXT` or `ATXT` field.
fn texture_and_quarter(data: &[u8]) -> Option<(u32, usize)> {
    let texture = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
    let quarter = usize::from(*data.get(4)?);
    Some((texture, quarter))
}

/// Writes the opacities of a `VTXT` field into `opacity`: entries of eight
/// bytes, the point's number, two unused bytes and the opacity from 0 to 1.
fn apply_opacities(data: &[u8], opacity: &mut [u8]) {
    for [p0, p1, _, _, v0, v1, v2, v3] in data.as_chunks::<8>().0 {
        let point = usize::from(u16::from_le_bytes([*p0, *p1]));
        let value = f32::from_le_bytes([*v0, *v1, *v2, *v3]);
        if let Some(slot) = opacity.get_mut(point)
            && value.is_finite()
        {
            *slot = to_byte(value);
        }
    }
}

/// An opacity from 0 to 1 as a byte.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(value: f32) -> u8 {
    // Clamped to 0 to 255 first, so the conversion is exact.
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// The colour texture of every landscape texture of a setup, in the
/// versions the game uses.
#[derive(Debug, Default)]
pub struct LandTextures {
    paths: HashMap<RecordId, String>,
}

impl LandTextures {
    /// Reads the landscape textures and texture sets of the active plugins
    /// of `game` in `game_dir`; `local_dir` holds `plugins.txt`.
    pub fn open(game: Game, game_dir: &Path, local_dir: Option<&Path>) -> Self {
        let data_dir = game_dir.join("Data");
        let plugins: Vec<(String, PathBuf)> =
            ingest::active_plugin_names(game, game_dir, local_dir)
                .unwrap_or_default()
                .into_iter()
                .map(|name| {
                    let path = data_dir.join(&name);
                    (name, path)
                })
                .collect();
        let mut sets: HashMap<RecordId, String> = HashMap::new();
        let mut landscape: HashMap<RecordId, Source> = HashMap::new();
        for (name, path) in &plugins {
            let Ok(file) = File::open(path) else {
                continue;
            };
            let Ok(read) = plugin::read(&mut BufReader::new(file), &[*b"LTEX", *b"TXST"]) else {
                continue;
            };
            let masters = &read.header.masters;
            for record in &read.records {
                let id = record_id(record.form_id, masters, name);
                match &record.signature {
                    b"TXST" => {
                        match record
                            .first(b"TX00")
                            .map(plugin::zstring)
                            .filter(|path| !path.is_empty())
                        {
                            Some(path) => {
                                sets.insert(id, path);
                            }
                            None => {
                                sets.remove(&id);
                            }
                        }
                    }
                    b"LTEX" => {
                        landscape.insert(id, source(record, masters, name));
                    }
                    _ => {}
                }
            }
        }
        let paths = landscape
            .into_iter()
            .filter_map(|(id, source)| {
                let path = match source {
                    Source::Set(set) => sets.get(&set)?.clone(),
                    Source::File(file) => format!("landscape/{file}"),
                    Source::Nothing => return None,
                };
                Some((id, path.replace('\\', "/").to_ascii_lowercase()))
            })
            .collect();
        Self { paths }
    }

    /// The colour texture of the landscape texture `id`, below the
    /// textures directory (`landscape/dirt01.dds`), if it has one.
    pub fn colour(&self, id: &RecordId) -> Option<&str> {
        self.paths.get(id).map(String::as_str)
    }
}

/// Where a landscape texture takes its colour from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    /// A texture set.
    Set(RecordId),
    /// A file in `textures/landscape`, as older games name it.
    File(String),
    Nothing,
}

fn source(record: &Record, masters: &[String], plugin_name: &str) -> Source {
    if let Some(set) = record
        .first(b"TNAM")
        .and_then(|data| data.get(..4))
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .map(u32::from_le_bytes)
        .filter(|form_id| *form_id != 0)
    {
        return Source::Set(record_id(set, masters, plugin_name));
    }
    match record
        .first(b"ICON")
        .map(plugin::zstring)
        .filter(|file| !file.is_empty())
    {
        Some(file) => Source::File(file),
        None => Source::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::plugin::Subrecord;
    use proptest::prelude::*;

    use super::*;

    fn land(fields: Vec<(&[u8; 4], Vec<u8>)>) -> Record {
        Record {
            signature: *b"LAND",
            form_id: 0x20,
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

    fn texture(form_id: u32, quarter: u8, order: u16) -> Vec<u8> {
        let mut data = form_id.to_le_bytes().to_vec();
        data.extend([quarter, 0]);
        data.extend(order.to_le_bytes());
        data
    }

    fn points(entries: &[(u16, f32)]) -> Vec<u8> {
        entries
            .iter()
            .flat_map(|(point, value)| {
                let mut entry = point.to_le_bytes().to_vec();
                entry.extend([0, 0]);
                entry.extend(value.to_le_bytes());
                entry
            })
            .collect()
    }

    fn id(local_id: u32) -> RecordId {
        RecordId {
            plugin: "skyrim.esm".to_owned(),
            local_id,
        }
    }

    #[test]
    fn layers_are_read_by_quarter_in_their_order() {
        let masters = vec!["Skyrim.esm".to_owned()];
        let record = land(vec![
            (b"BTXT", texture(0x10, 0, 0xFFFF)),
            (b"BTXT", texture(0x11, 3, 0xFFFF)),
            // Read before the layer under it: sorted by its number.
            (b"ATXT", texture(0x13, 0, 1)),
            (b"VTXT", points(&[(0, 1.0), (288, 0.5)])),
            (b"ATXT", texture(0x12, 0, 0)),
            (b"VTXT", points(&[(18, 0.25)])),
            // A layer that shows nowhere is left out.
            (b"ATXT", texture(0x14, 1, 0)),
            (b"VTXT", points(&[(5, 0.0)])),
        ]);
        let quarters = quarters(&record, &masters, "Mod.esp");
        assert_eq!(quarters.len(), 4);
        assert_eq!(quarters[0].base, Some(id(0x10)));
        assert_eq!(quarters[3].base, Some(id(0x11)));
        assert_eq!(quarters[1], Quarter::default());
        let layers = &quarters[0].layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].texture, id(0x12));
        assert_eq!(layers[0].opacity[18], 64);
        assert_eq!(layers[1].texture, id(0x13));
        assert_eq!(layers[1].opacity[0], 255);
        assert_eq!(layers[1].opacity[288], 128);
        assert_eq!(layers[1].opacity[1], 0);
    }

    #[test]
    fn a_quarter_out_of_range_is_ignored() {
        let record = land(vec![
            (b"BTXT", texture(0x10, 7, 0)),
            (b"ATXT", texture(0x10, 9, 0)),
            (b"VTXT", points(&[(0, 1.0)])),
        ]);
        assert!(
            quarters(&record, &[], "Mod.esp")
                .iter()
                .all(|quarter| *quarter == Quarter::default())
        );
    }

    #[test]
    fn a_landscape_texture_takes_its_colour_from_its_set_or_its_file() {
        let masters = vec!["Skyrim.esm".to_owned()];
        let with_set = Record {
            signature: *b"LTEX",
            form_id: 1,
            flags: 0,
            subrecords: vec![Subrecord {
                signature: *b"TNAM",
                data: 0x55u32.to_le_bytes().to_vec(),
            }],
            groups: Vec::new(),
        };
        assert_eq!(
            source(&with_set, &masters, "Mod.esp"),
            Source::Set(id(0x55))
        );
        let with_file = Record {
            subrecords: vec![Subrecord {
                signature: *b"ICON",
                data: b"Dirt01.dds\0".to_vec(),
            }],
            ..with_set.clone()
        };
        assert_eq!(
            source(&with_file, &masters, "Mod.esp"),
            Source::File("Dirt01.dds".to_owned())
        );
        let bare = Record {
            subrecords: Vec::new(),
            ..with_set
        };
        assert_eq!(source(&bare, &masters, "Mod.esp"), Source::Nothing);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_fields(
            btxt in proptest::collection::vec(any::<u8>(), 0..12),
            atxt in proptest::collection::vec(any::<u8>(), 0..12),
            vtxt in proptest::collection::vec(any::<u8>(), 0..400),
            tnam in proptest::collection::vec(any::<u8>(), 0..6),
        ) {
            let record = land(vec![(b"BTXT", btxt), (b"ATXT", atxt), (b"VTXT", vtxt)]);
            let _ = quarters(&record, &[], "Mod.esp");
            let texture = Record { signature: *b"LTEX", ..land(vec![(b"TNAM", tnam)]) };
            let _ = source(&texture, &[], "Mod.esp");
        }
    }
}
