//! The places of the world map, read from the plugins.
//!
//! A map marker is a reference (`REFR`) in the cells of a worldspace that
//! carries map marker data: an empty `XMRK` field, its flags in `FNAM`, the
//! name the map shows in `FULL` and its kind in the first byte of `TNAM`.
//! In a localized plugin `FULL` holds the number of a string in the
//! plugin's string table rather than the name itself.
//!
//! Every plugin is read for markers. The game's own plugins are large and
//! keep their markers in the persistent cells of their worldspaces; they
//! are read once, from those cells only, and then taken from the cache as
//! text ([`Contribution::to_text`]).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use mallet_core::GamePath;
use mallet_core::facts::RecordId;
use mallet_core::facts::markers::{MapMarkerFact, MarkerFacts, MarkerVersion};
use mallet_formats_bethesda::plugin::{self, GroupLabel, Plugin, Record, Signature, group_type};
use mallet_formats_bethesda::{bsa, strings};

use crate::meshes::LoadedArchive;

/// Record flag: the record is deleted.
const FLAG_DELETED: u32 = 0x0000_0020;

/// Record flag of references: the object is switched off when the game
/// starts.
const FLAG_INITIALLY_DISABLED: u32 = 0x0000_0800;

/// Positions farther out than this are not real places.
const MAX_COORDINATE: f32 = 1.0e7;

/// The top-level group read for markers in the game's own plugins.
pub const MARKER_GROUP: [Signature; 1] = [*b"WRLD"];

/// The records read for markers in the game's own plugins.
pub const MARKER_RECORDS: [Signature; 1] = [*b"REFR"];

/// Whether a group of the game's own plugins can hold markers: everything
/// but the temporary children of cells, which hold the bulk of the world.
pub fn may_hold_markers(group: GroupLabel) -> bool {
    group.group_type != group_type::CELL_TEMPORARY_CHILDREN
}

/// The language whose string tables are preferred when several are there.
const PREFERRED_LANGUAGE: &str = "english";

/// One entry of what a plugin says about markers.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    /// A marker in this plugin's version.
    Marker {
        id: RecordId,
        world: RecordId,
        kind: u8,
        name: Option<String>,
        position: [i32; 3],
        removed: bool,
    },
    /// A reference this plugin deletes or switches off, without marker
    /// data. If it is a marker, the marker is gone.
    Removal { id: RecordId },
}

/// What one plugin says about map markers. It can be kept as text, so
/// that a large plugin that has not changed need not be read again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Contribution {
    entries: Vec<Entry>,
}

impl Contribution {
    /// The markers of `plugin`, named `plugin_name`. `names` looks up the
    /// name of a localized plugin by its number.
    pub fn of(plugin_name: &str, plugin: &Plugin, names: &HashMap<u32, String>) -> Self {
        let masters = &plugin.header.masters;
        let id = |form_id: u32| {
            let (origin, local_id) = plugin::resolve_form_id(form_id, masters, plugin_name);
            RecordId {
                plugin: origin.to_lowercase(),
                local_id,
            }
        };
        let mut entries = Vec::new();
        for record in plugin
            .records
            .iter()
            .filter(|record| &record.signature == b"REFR")
        {
            let Some(world) = record
                .groups
                .iter()
                .find(|group| group.group_type == group_type::WORLD_CHILDREN)
                .map(|group| id(group.label))
            else {
                continue;
            };
            let removed = record.flags & (FLAG_DELETED | FLAG_INITIALLY_DISABLED) != 0;
            if record.first(b"XMRK").is_none() {
                if removed {
                    entries.push(Entry::Removal {
                        id: id(record.form_id),
                    });
                }
                continue;
            }
            let Some(position) = position(record) else {
                if removed {
                    entries.push(Entry::Removal {
                        id: id(record.form_id),
                    });
                }
                continue;
            };
            entries.push(Entry::Marker {
                id: id(record.form_id),
                world,
                kind: record
                    .first(b"TNAM")
                    .and_then(|data| data.first().copied())
                    .unwrap_or_default(),
                name: name(record, plugin.header.is_localized, names),
                position,
                removed,
            });
        }
        Self { entries }
    }

    /// Whether the plugin has nothing to say about markers.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The contribution as text, one line per entry, fields separated by
    /// tabs.
    pub fn to_text(&self) -> String {
        let lines: Vec<String> = self
            .entries
            .iter()
            .map(|entry| match entry {
                Entry::Marker {
                    id,
                    world,
                    kind,
                    name,
                    position: [x, y, z],
                    removed,
                } => format!(
                    "M\t{}\t{}\t{}\t{}\t{kind}\t{x}\t{y}\t{z}\t{}\t{}",
                    id.plugin,
                    id.local_id,
                    world.plugin,
                    world.local_id,
                    u8::from(*removed),
                    // Names are one line of text; tabs would split them.
                    name.as_deref()
                        .unwrap_or_default()
                        .replace(['\t', '\n', '\r'], " "),
                ),
                Entry::Removal { id } => format!("R\t{}\t{}", id.plugin, id.local_id),
            })
            .collect();
        lines.join("\n")
    }

    /// Reads a contribution written by [`Contribution::to_text`]. `None`
    /// if the text is not one.
    pub fn from_text(text: &str) -> Option<Self> {
        let id = |plugin: &str, local_id: &str| {
            Some(RecordId {
                plugin: plugin.to_owned(),
                local_id: local_id.parse().ok()?,
            })
        };
        let mut entries = Vec::new();
        for line in text.lines().filter(|line| !line.is_empty()) {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                [
                    "M",
                    plugin,
                    local_id,
                    world,
                    world_id,
                    kind,
                    x,
                    y,
                    z,
                    removed,
                    name,
                ] => entries.push(Entry::Marker {
                    id: id(plugin, local_id)?,
                    world: id(world, world_id)?,
                    kind: kind.parse().ok()?,
                    name: Some((*name).to_owned()).filter(|name| !name.is_empty()),
                    position: [x.parse().ok()?, y.parse().ok()?, z.parse().ok()?],
                    removed: *removed == "1",
                }),
                ["R", plugin, local_id] => entries.push(Entry::Removal {
                    id: id(plugin, local_id)?,
                }),
                _ => return None,
            }
        }
        Some(Self { entries })
    }
}

/// Gathers the markers of all plugins, which have to be added in load
/// order so that later versions replace earlier ones.
#[derive(Debug, Default)]
pub struct Collector {
    markers: HashMap<RecordId, MapMarkerFact>,
}

impl Collector {
    /// Takes in what the plugin `plugin_name` says about markers.
    pub fn merge(&mut self, plugin_name: &str, contribution: Contribution) {
        for entry in contribution.entries {
            match entry {
                Entry::Marker {
                    id,
                    world,
                    kind,
                    name,
                    position,
                    removed,
                } => {
                    let version = MarkerVersion {
                        plugin: plugin_name.to_owned(),
                        official: crate::objects::is_official(plugin_name),
                        kind,
                        name,
                        position,
                        removed,
                    };
                    match self.markers.get_mut(&id) {
                        Some(marker) => marker.last = version,
                        None => {
                            self.markers.insert(
                                id.clone(),
                                MapMarkerFact {
                                    id,
                                    worldspace: world,
                                    first: version.clone(),
                                    last: version,
                                },
                            );
                        }
                    }
                }
                Entry::Removal { id } => {
                    if let Some(marker) = self.markers.get_mut(&id) {
                        marker.last = MarkerVersion {
                            plugin: plugin_name.to_owned(),
                            official: crate::objects::is_official(plugin_name),
                            removed: true,
                            ..marker.last.clone()
                        };
                    }
                }
            }
        }
    }

    /// The markers of the setup.
    pub fn finish(self) -> MarkerFacts {
        let mut markers: Vec<MapMarkerFact> = self.markers.into_values().collect();
        markers.sort_by(|a, b| a.id.cmp(&b.id));
        MarkerFacts { markers }
    }
}

/// The position of a reference, in whole game units.
fn position(record: &Record) -> Option<[i32; 3]> {
    let data = record.first(b"DATA")?.get(..12)?;
    let mut position = [0i32; 3];
    for (value, bytes) in position.iter_mut().zip(data.as_chunks::<4>().0) {
        *value = whole(f32::from_le_bytes(*bytes))?;
    }
    Some(position)
}

/// The name of a marker: its `FULL` field, looked up in the string table
/// for a localized plugin, else the reference's internal name.
fn name(record: &Record, localized: bool, names: &HashMap<u32, String>) -> Option<String> {
    let full = record.first(b"FULL").and_then(|data| {
        if localized {
            let number = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
            names.get(&number).cloned()
        } else {
            Some(plugin::zstring(data))
        }
    });
    full.or_else(|| record.first(b"EDID").map(plugin::zstring))
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// A coordinate rounded to a whole number, or `None` if it is not one a
/// real place could have.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> Option<i32> {
    // The range check makes the conversion exact up to rounding.
    (value.is_finite() && value.abs() <= MAX_COORDINATE).then(|| value.round() as i32)
}

/// The names of the string table of `plugin_name`, from a loose file or an
/// archive, in English if the setup has several languages. Empty if there
/// is none.
pub(crate) fn string_table(
    plugin_name: &str,
    loose: &[(GamePath, PathBuf)],
    archives: &[LoadedArchive],
) -> HashMap<u32, String> {
    let stem = plugin_name
        .rsplit_once('.')
        .map_or(plugin_name, |(stem, _)| stem)
        .to_lowercase();
    let prefix = format!("data/strings/{stem}_");
    // The language of a table path, if it is one of this plugin.
    let language = |path: &str| -> Option<String> {
        path.strip_prefix(&prefix)?
            .strip_suffix(".strings")
            .map(str::to_owned)
    };
    let rank = |language: &str| u8::from(language != PREFERRED_LANGUAGE);

    // A loose table takes precedence over the same table in an archive.
    let best_loose = loose
        .iter()
        .filter_map(|(path, file)| Some((language(path.as_str())?, file)))
        .min_by_key(|(language, _)| rank(language));
    if let Some((_, file)) = best_loose
        && let Ok(data) = std::fs::read(file)
    {
        return strings::parse_strings(&data);
    }
    let best_packed = archives
        .iter()
        .rev()
        .flat_map(|archive| {
            archive.directory.entries.iter().filter_map(move |entry| {
                // Paths in an archive lie below the data directory.
                let path = GamePath::new(crate::DATA_DIR).join(&entry.path);
                Some((language(path.as_str())?, archive, entry))
            })
        })
        .min_by_key(|(language, _, _)| rank(language));
    let Some((_, archive, entry)) = best_packed else {
        return HashMap::new();
    };
    read_packed(archive, entry).map_or_else(HashMap::new, |data| strings::parse_strings(&data))
}

/// The content of one file in an archive.
fn read_packed(archive: &LoadedArchive, entry: &bsa::Entry) -> Option<Vec<u8>> {
    let mut file = File::open(&archive.file).ok()?;
    file.seek(SeekFrom::Start(u64::from(entry.offset))).ok()?;
    let mut stored = Vec::new();
    file.take(u64::from(entry.size))
        .read_to_end(&mut stored)
        .ok()?;
    bsa::unpack(&stored, entry, &archive.directory).ok()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use mallet_formats_bethesda::plugin::Request;
    use proptest::prelude::*;

    use super::*;

    /// The game's own marker object.
    const MAP_MARKER: u32 = 0x10;

    fn marker(form_id: u32, kind: u8, full: Vec<u8>, position: [f32; 3]) -> RecordSpec {
        fixture::reference(form_id, MAP_MARKER, position, [0.0; 3], None)
            .with(*b"XMRK", Vec::new())
            .with(*b"FNAM", vec![1])
            .with(*b"FULL", full)
            .with(*b"TNAM", vec![kind, 0])
    }

    fn read(data: &[u8]) -> Plugin {
        plugin::read_request(
            &mut Cursor::new(data),
            &Request {
                flat: &[],
                nested: &MARKER_GROUP,
                nested_records: &MARKER_RECORDS,
            },
        )
        .unwrap()
    }

    fn world() -> RecordSpec {
        RecordSpec::new(*b"WRLD", 0x3C).with(*b"EDID", fixture::zstring("Tamriel"))
    }

    /// A localized game with two markers, a mod that moves one of them and
    /// adds one of its own, and a patch that removes the other.
    fn setup() -> MarkerFacts {
        const LOCALIZED: u32 = 0x80;
        let game = fixture::plugin_with_raw_groups(
            &[],
            1 | LOCALIZED,
            &[fixture::worldspace(
                &world(),
                &[(
                    RecordSpec::new(*b"CELL", 0x20),
                    vec![
                        marker(
                            0x100,
                            2,
                            7u32.to_le_bytes().to_vec(),
                            [1000.0, 2000.0, 30.0],
                        ),
                        marker(0x101, 4, 8u32.to_le_bytes().to_vec(), [-500.0, 0.0, 0.0]),
                        fixture::reference(0x102, 0x1234, [0.0; 3], [0.0; 3], None),
                    ],
                )],
            )],
        );
        let names: HashMap<u32, String> =
            [(7, "Millbrook".to_owned()), (8, "Echo Hollow".to_owned())]
                .into_iter()
                .collect();
        let a_mod = fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[fixture::worldspace(
                &world(),
                &[(
                    RecordSpec::new(*b"CELL", 0x20),
                    vec![
                        marker(
                            0x100,
                            2,
                            fixture::zstring("Millbrook"),
                            [9000.0, 2000.0, 30.0],
                        ),
                        marker(
                            0x0100_0800,
                            5,
                            fixture::zstring("Bandit Camp"),
                            [4.4, 5.6, 6.0],
                        ),
                    ],
                )],
            )],
        );
        let mut gone = fixture::reference(0x101, MAP_MARKER, [0.0; 3], [0.0; 3], None);
        gone.flags = FLAG_DELETED;
        let patch = fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[fixture::worldspace(
                &world(),
                &[(RecordSpec::new(*b"CELL", 0x20), vec![gone])],
            )],
        );
        let mut collector = Collector::default();
        for (name, data, names) in [
            ("Skyrim.esm", game, names),
            ("Village.esp", a_mod, HashMap::new()),
            ("Patch.esp", patch, HashMap::new()),
        ] {
            collector.merge(name, Contribution::of(name, &read(&data), &names));
        }
        collector.finish()
    }

    #[test]
    fn markers_are_read_with_their_names_from_the_string_table() {
        let facts = setup();
        let ids: Vec<_> = facts
            .markers
            .iter()
            .map(|marker| marker.id.to_string())
            .collect();
        assert_eq!(facts.markers.len(), 3, "{ids:?}");
        let riverwood = &facts.markers[0];
        assert_eq!(riverwood.first.plugin, "Skyrim.esm");
        assert_eq!(riverwood.first.name.as_deref(), Some("Millbrook"));
        assert_eq!(riverwood.first.kind, 2);
        assert_eq!(riverwood.first.position, [1000, 2000, 30]);
        assert_eq!(riverwood.worldspace.local_id, 0x3C);
    }

    #[test]
    fn later_versions_move_and_remove_markers() {
        let facts = setup();
        let riverwood = &facts.markers[0];
        assert_eq!(riverwood.last.plugin, "Village.esp");
        assert_eq!(riverwood.last.position, [9000, 2000, 30]);
        assert!(!riverwood.last.removed);

        let cave = &facts.markers[1];
        assert_eq!(cave.first.name.as_deref(), Some("Echo Hollow"));
        assert_eq!(cave.last.plugin, "Patch.esp");
        assert!(cave.last.removed);
        // The removal keeps what was known of the marker.
        assert_eq!(cave.last.kind, 4);

        let camp = &facts.markers[2];
        assert_eq!(camp.id.plugin, "village.esp");
        assert_eq!(camp.first.plugin, "Village.esp");
        assert_eq!(camp.first.position, [4, 6, 6]);
    }

    #[test]
    fn only_persistent_groups_need_to_be_read() {
        let temporary = GroupLabel {
            group_type: group_type::CELL_TEMPORARY_CHILDREN,
            label: 1,
        };
        let persistent = GroupLabel {
            group_type: group_type::CELL_PERSISTENT_CHILDREN,
            label: 1,
        };
        assert!(!may_hold_markers(temporary));
        assert!(may_hold_markers(persistent));
    }

    #[test]
    fn a_contribution_survives_being_kept_as_text() {
        let game = fixture::plugin_with_raw_groups(
            &[],
            1,
            &[fixture::worldspace(
                &world(),
                &[(
                    RecordSpec::new(*b"CELL", 0x20),
                    vec![marker(0x100, 6, fixture::zstring("Fort\tAshen"), [1.0; 3])],
                )],
            )],
        );
        let mut contribution = Contribution::of("Skyrim.esm", &read(&game), &HashMap::new());
        contribution.entries.push(Entry::Removal {
            id: RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id: 5,
            },
        });
        let text = contribution.to_text();
        let back = Contribution::from_text(&text).unwrap();
        let Entry::Marker { name, .. } = &back.entries[0] else {
            panic!("expected a marker, got {back:?}");
        };
        assert_eq!(name.as_deref(), Some("Fort Ashen"));
        assert_eq!(back.entries[1], contribution.entries[1]);
        assert_eq!(Contribution::from_text("nonsense"), None);
    }

    #[test]
    fn string_tables_are_found_loose_with_english_preferred() {
        let directory = tempfile::tempdir().unwrap();
        let table = |text: &str| {
            let mut data = 1u32.to_le_bytes().to_vec();
            data.extend(u32::try_from(text.len() + 1).unwrap().to_le_bytes());
            data.extend(7u32.to_le_bytes());
            data.extend(0u32.to_le_bytes());
            data.extend(text.as_bytes());
            data.push(0);
            data
        };
        let german = directory.path().join("german.strings");
        let english = directory.path().join("english.strings");
        std::fs::write(&german, table("Mühlbach")).unwrap();
        std::fs::write(&english, table("Millbrook")).unwrap();
        let loose = vec![
            (
                GamePath::new("data/strings/skyrim_german.strings"),
                german.clone(),
            ),
            (
                GamePath::new("data/strings/skyrim_english.strings"),
                english,
            ),
            (GamePath::new("data/strings/other_english.strings"), german),
        ];
        let names = string_table("Skyrim.esm", &loose, &[]);
        assert_eq!(names.get(&7).map(String::as_str), Some("Millbrook"));
        assert!(string_table("Unknown.esp", &loose, &[]).is_empty());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_fields(
            data in proptest::collection::vec(any::<u8>(), 0..16),
            full in proptest::collection::vec(any::<u8>(), 0..8),
            localized in any::<bool>(),
        ) {
            let record = Record {
                signature: *b"REFR",
                form_id: 1,
                flags: 0,
                subrecords: vec![
                    plugin::Subrecord { signature: *b"DATA", data },
                    plugin::Subrecord { signature: *b"FULL", data: full },
                ],
                groups: Vec::new(),
            };
            let _ = position(&record);
            let _ = name(&record, localized, &HashMap::new());
        }

        #[test]
        fn never_panics_on_arbitrary_text(text in ".{0,200}") {
            let _ = Contribution::from_text(&text);
        }
    }
}
