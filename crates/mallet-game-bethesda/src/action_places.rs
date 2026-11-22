//! What characters do at furniture and idle markers (ADR 0069).
//!
//! Furniture (`FURN`) is what characters sit, lean, sleep or work at; an
//! idle marker (`IDLM`) is a spot where they stand and do something. The
//! game tells them apart by keywords (`KWDA`, naming `KYWD` records such
//! as `FurnitureWoodChoppingBlock` or `CraftingSmithingForge`) and by the
//! animations they play; their internal names and models say the same in
//! words, such as `WallLeanMarker` or `ChairMarker`. A place is sorted by
//! the words of its internal name, its keywords' internal names and its
//! model's file name.
//!
//! Read once per setup, in the versions the load order gives the records.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use mallet_core::action_places::ActionKind;
use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, Record, Signature};

use crate::objects::{FLAG_DELETED, editor_id, model_path, record_id};

/// The records read: furniture, idle markers and the keywords they name.
const TYPES: [Signature; 3] = [*b"FURN", *b"IDLM", *b"KYWD"];

/// What is known of a base object characters do something at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionBase {
    /// What characters do there.
    pub kind: ActionKind,
    /// Whether it is an idle marker rather than furniture.
    pub idle_marker: bool,
    /// Its internal name.
    pub editor_id: Option<String>,
    /// The name players see, where the plugin spells it out.
    pub name: Option<String>,
}

/// A base record as read, before its keywords are named.
struct Read {
    idle_marker: bool,
    editor_id: Option<String>,
    name: Option<String>,
    model: Option<String>,
    keywords: Vec<RecordId>,
}

/// The furniture and idle markers of `plugins` (name and file, in load
/// order), in the versions the game uses. A plugin that cannot be read is
/// skipped.
pub fn read(plugins: &[(String, PathBuf)]) -> HashMap<RecordId, ActionBase> {
    let mut bases: HashMap<RecordId, Option<Read>> = HashMap::new();
    let mut keywords: HashMap<RecordId, String> = HashMap::new();
    for (name, path) in plugins {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let Ok(found) = plugin::read(&mut BufReader::new(file), &TYPES) else {
            continue;
        };
        let masters = &found.header.masters;
        let localized = found.header.is_localized;
        for record in &found.records {
            let id = record_id(record.form_id, masters, name);
            match &record.signature {
                b"KYWD" => {
                    if let Some(editor_id) = editor_id(record) {
                        keywords.insert(id, editor_id);
                    }
                }
                b"FURN" | b"IDLM" => {
                    let read = (record.flags & FLAG_DELETED == 0)
                        .then(|| read_base(record, masters, name, localized));
                    bases.insert(id, read);
                }
                _ => {}
            }
        }
    }
    bases
        .into_iter()
        .filter_map(|(id, read)| {
            let read = read?;
            let mut words: Vec<&str> = Vec::new();
            words.extend(read.editor_id.as_deref());
            words.extend(
                read.keywords
                    .iter()
                    .filter_map(|keyword| keywords.get(keyword).map(String::as_str)),
            );
            let model = read
                .model
                .as_deref()
                .and_then(|model| model.rsplit('/').next());
            words.extend(model);
            let kind = kind_of(&words, read.idle_marker);
            Some((
                id,
                ActionBase {
                    kind,
                    idle_marker: read.idle_marker,
                    editor_id: read.editor_id,
                    name: read.name,
                },
            ))
        })
        .collect()
}

/// A furniture or idle marker record as this version writes it.
fn read_base(record: &Record, masters: &[String], plugin_name: &str, localized: bool) -> Read {
    let keywords = record.first(b"KWDA").map_or_else(Vec::new, |data| {
        data.as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_le_bytes(*bytes))
            .filter(|id| *id != 0)
            .map(|id| record_id(id, masters, plugin_name))
            .collect()
    });
    Read {
        idle_marker: &record.signature == b"IDLM",
        editor_id: editor_id(record),
        name: (!localized)
            .then(|| record.first(b"FULL").map(plugin::zstring))
            .flatten()
            .filter(|name| !name.is_empty()),
        model: model_path(record),
        keywords,
    }
}

/// The words that tell what characters do, the most telling first: a
/// workbench for potions is crafting, not smithing; a workbench is
/// smithing, not sitting on a bench.
const WORDS: [(ActionKind, &[&str]); 10] = [
    (
        ActionKind::ChopWood,
        &["woodchop", "chopping", "choppingblock"],
    ),
    (ActionKind::Mine, &["pickaxe", "mining", "oremine"]),
    (
        ActionKind::Craft,
        &["alchemy", "enchant", "staffenchant", "alchemylab"],
    ),
    (
        ActionKind::Smith,
        &[
            "forge",
            "anvil",
            "smelter",
            "tanning",
            "sharpening",
            "grindstone",
            "smithing",
            "workbench",
            "armortable",
        ],
    ),
    (
        ActionKind::Cook,
        &["cook", "spit", "oven", "churn", "stew", "kitchen"],
    ),
    (ActionKind::Sleep, &["bed", "sleep", "coffin", "hammock"]),
    (ActionKind::Lean, &["lean", "railing"]),
    (
        ActionKind::Pray,
        &["pray", "kneel", "worship", "shrine", "meditat", "mourn"],
    ),
    (
        ActionKind::Sit,
        &[
            "chair", "bench", "stool", "throne", "sit", "seat", "pew", "lounge", "sofa",
        ],
    ),
    (
        ActionKind::Work,
        &[
            "broom", "sweep", "rake", "hoe", "farm", "garden", "wheat", "hammer", "shovel", "dig",
            "mill", "wash", "scrub", "fish", "harvest", "carry", "write", "read", "book", "drink",
            "woodpile", "lute", "drum", "flute", "play",
        ],
    ),
];

/// What characters do at a place whose internal names and model file are
/// `words`. Without a telling word, an idle marker is a spot to stand and
/// furniture is other furniture.
pub fn kind_of(words: &[&str], idle_marker: bool) -> ActionKind {
    let lower: Vec<String> = words.iter().map(|word| word.to_ascii_lowercase()).collect();
    WORDS
        .iter()
        .find(|(_, parts)| {
            lower
                .iter()
                .any(|word| parts.iter().any(|part| word.contains(part)))
        })
        .map_or(
            if idle_marker {
                ActionKind::Stand
            } else {
                ActionKind::Other
            },
            |(kind, _)| *kind,
        )
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn places_are_sorted_by_their_words() {
        let cases: [(&[&str], bool, ActionKind); 12] = [
            (&["WallLeanMarker"], false, ActionKind::Lean),
            (&["ChairMarker"], false, ActionKind::Sit),
            (&["SitGroundMarker"], false, ActionKind::Sit),
            (&["BedrollGround"], false, ActionKind::Sleep),
            (
                &["WoodChoppingBlock", "FurnitureWoodChoppingBlock"],
                false,
                ActionKind::ChopWood,
            ),
            (
                &["AlchemyWorkbench", "CraftingAlchemyWorkbench"],
                false,
                ActionKind::Craft,
            ),
            (&["CraftingSmithingArmorTable"], false, ActionKind::Smith),
            (&["Something", "CraftingCookpot"], false, ActionKind::Cook),
            (&["IdleBroomSweepMarker"], true, ActionKind::Work),
            (&["PickaxeMiningFloorMarker"], false, ActionKind::Mine),
            (&["ICAI_Marker01"], true, ActionKind::Stand),
            (&["ICAI_Furniture01"], false, ActionKind::Other),
        ];
        for (words, idle, kind) in cases {
            assert_eq!(kind_of(words, idle), kind, "{words:?}");
        }
    }

    #[test]
    fn keywords_and_later_versions_count() {
        let dir = tempfile::tempdir().unwrap();
        let main = dir.path().join("Main.esm");
        let keyword = RecordSpec::new(*b"KYWD", 0x800)
            .with(*b"EDID", fixture::zstring("FurnitureWoodChoppingBlock"));
        let block = RecordSpec::new(*b"FURN", 0x801)
            .with(*b"EDID", fixture::zstring("ChoppingThing"))
            .with(*b"KWDA", 0x800u32.to_le_bytes().to_vec());
        let plain = RecordSpec::new(*b"FURN", 0x802).with(*b"EDID", fixture::zstring("Thing"));
        let idle =
            RecordSpec::new(*b"IDLM", 0x803).with(*b"EDID", fixture::zstring("WallLeanMarkerIdle"));
        std::fs::write(
            &main,
            fixture::plugin(
                &[],
                0,
                &[
                    (*b"KYWD", vec![keyword]),
                    (*b"FURN", vec![block, plain]),
                    (*b"IDLM", vec![idle]),
                ],
            ),
        )
        .unwrap();
        // A later plugin gives the plain furniture the keyword.
        let later = dir.path().join("Later.esp");
        let changed = RecordSpec::new(*b"FURN", 0x802)
            .with(*b"EDID", fixture::zstring("Thing"))
            .with(*b"KWDA", 0x800u32.to_le_bytes().to_vec());
        std::fs::write(
            &later,
            fixture::plugin(&["Main.esm"], 0, &[(*b"FURN", vec![changed])]),
        )
        .unwrap();

        let found = read(&[
            ("Main.esm".to_owned(), main),
            ("Later.esp".to_owned(), later),
        ]);
        let kind = |local_id: u32| {
            found
                .get(&RecordId {
                    plugin: "main.esm".to_owned(),
                    local_id,
                })
                .map(|base| (base.kind, base.idle_marker))
        };
        assert_eq!(kind(0x801), Some((ActionKind::ChopWood, false)));
        assert_eq!(kind(0x802), Some((ActionKind::ChopWood, false)));
        assert_eq!(kind(0x803), Some((ActionKind::Lean, true)));
    }

    proptest! {
        #[test]
        fn never_panics(
            kwda in prop::collection::vec(any::<u8>(), 0..16),
            full in prop::collection::vec(any::<u8>(), 0..8),
            words in prop::collection::vec(".{0,12}", 0..4),
        ) {
            let record = Record {
                signature: *b"FURN",
                form_id: 0x0100_0010,
                flags: 0,
                subrecords: vec![
                    plugin::Subrecord { signature: *b"KWDA", data: kwda },
                    plugin::Subrecord { signature: *b"FULL", data: full },
                ],
                groups: Vec::new(),
            };
            let _ = read_base(&record, &["Main.esm".to_owned()], "Mod.esp", false);
            let words: Vec<&str> = words.iter().map(String::as_str).collect();
            let _ = kind_of(&words, true);
        }
    }
}
