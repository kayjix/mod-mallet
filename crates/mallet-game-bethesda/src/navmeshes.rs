//! Which navigation meshes of other plugins a plugin changes.
//!
//! A navigation mesh (`NAVM`) lies in the children of its cell, like the
//! objects placed there. Only its identity is read, never its triangles,
//! so it is not even unpacked ([`HEADERS`]). Its place is named after its
//! cell: the cell's internal name inside, the worldspace's internal name
//! and the cell's grid position (`XCLC`) outside, which is what the game's
//! console takes to go there.

use std::collections::HashMap;

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{NavmeshChange, NavmeshChanges};
use mallet_formats_bethesda::plugin::{self, Plugin, Record, Signature, group_type};

/// The record type of navigation meshes.
pub const NAVMESH: Signature = *b"NAVM";

/// The records to read with their header only from the groups of places.
pub const HEADERS: [Signature; 1] = [NAVMESH];

/// Record flag: the record is deleted.
const FLAG_DELETED: u32 = 0x0000_0020;

/// The navigation meshes of other plugins that `plugin`, named
/// `plugin_name`, changes; `None` if it changes none. The plugin must have
/// been read with its cells, worldspaces and navigation meshes.
pub fn changes(plugin_name: &str, plugin: &Plugin) -> Option<NavmeshChanges> {
    let masters = &plugin.header.masters;
    let worlds: HashMap<u32, String> = plugin
        .records
        .iter()
        .filter(|record| &record.signature == b"WRLD")
        .filter_map(|record| Some((record.form_id, editor_id(record)?)))
        .collect();
    let cells: HashMap<u32, String> = plugin
        .records
        .iter()
        .filter(|record| &record.signature == b"CELL")
        .filter_map(|record| Some((record.form_id, cell_name(record, &worlds)?)))
        .collect();

    let navmeshes: Vec<NavmeshChange> = plugin
        .records
        .iter()
        .filter(|record| record.signature == NAVMESH && record.flags & FLAG_DELETED == 0)
        .filter_map(|record| {
            let (origin, local_id) = plugin::resolve_form_id(record.form_id, masters, plugin_name);
            if origin.eq_ignore_ascii_case(plugin_name) {
                return None;
            }
            let cell = record
                .groups
                .iter()
                .find(|group| group.group_type == group_type::CELL_CHILDREN)
                .map(|group| group.label);
            Some(NavmeshChange {
                id: RecordId {
                    plugin: origin.to_lowercase(),
                    local_id,
                },
                place: cell.and_then(|cell| cells.get(&cell)).cloned(),
            })
        })
        .collect();
    (!navmeshes.is_empty()).then(|| NavmeshChanges {
        plugin: plugin_name.to_owned(),
        navmeshes,
    })
}

/// A cell's name for people: the worldspace and grid position outside, the
/// internal name inside.
fn cell_name(cell: &Record, worlds: &HashMap<u32, String>) -> Option<String> {
    let world = cell
        .groups
        .iter()
        .find(|group| group.group_type == group_type::WORLD_CHILDREN)
        .and_then(|group| worlds.get(&group.label));
    let grid = cell.first(b"XCLC").and_then(|data| {
        let x = i32::from_le_bytes(data.get(0..4)?.try_into().ok()?);
        let y = i32::from_le_bytes(data.get(4..8)?.try_into().ok()?);
        Some((x, y))
    });
    match (world, grid) {
        (Some(world), Some((x, y))) => Some(format!("{world} {x},{y}")),
        _ => editor_id(cell),
    }
}

fn editor_id(record: &Record) -> Option<String> {
    record
        .first(b"EDID")
        .map(plugin::zstring)
        .filter(|name| !name.is_empty())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use mallet_formats_bethesda::plugin::Request;
    use proptest::prelude::*;

    use super::*;
    use crate::objects::{PLACE_GROUPS, PLACE_RECORDS};

    fn read(data: &[u8]) -> Plugin {
        plugin::read_request(
            &mut Cursor::new(data),
            &Request {
                flat: &[],
                nested: &PLACE_GROUPS,
                nested_records: &PLACE_RECORDS,
                nested_headers: &HEADERS,
            },
        )
        .unwrap()
    }

    fn grid(x: i32, y: i32) -> Vec<u8> {
        let mut data = x.to_le_bytes().to_vec();
        data.extend(y.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data
    }

    /// A mod that changes a navigation mesh of the game inside and one
    /// outside, and adds one of its own.
    fn a_mod() -> Vec<u8> {
        let inn = RecordSpec::new(*b"CELL", 0x0000_0100).with(*b"EDID", fixture::zstring("Inn"));
        let world =
            RecordSpec::new(*b"WRLD", 0x0000_003C).with(*b"EDID", fixture::zstring("Tamriel"));
        let field = RecordSpec::new(*b"CELL", 0x0000_0200).with(*b"XCLC", grid(5, -3));
        fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[
                fixture::interior_cells(&[(
                    inn,
                    vec![
                        RecordSpec::new(NAVMESH, 0x0000_0110)
                            .with(*b"NVNM", vec![7; 64])
                            .compressed(),
                    ],
                )]),
                fixture::worldspace(
                    &world,
                    &[(
                        field,
                        vec![
                            RecordSpec::new(NAVMESH, 0x0000_0210),
                            RecordSpec::new(NAVMESH, 0x0100_0800),
                        ],
                    )],
                ),
            ],
        )
    }

    #[test]
    fn navmeshes_of_other_plugins_are_named_by_their_cell() {
        let changes = changes("Village.esp", &read(&a_mod())).unwrap();

        assert_eq!(changes.plugin, "Village.esp");
        let ids: Vec<(&str, u32, Option<&str>)> = changes
            .navmeshes
            .iter()
            .map(|change| {
                (
                    change.id.plugin.as_str(),
                    change.id.local_id,
                    change.place.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            ids,
            vec![
                ("skyrim.esm", 0x110, Some("Inn")),
                ("skyrim.esm", 0x210, Some("Tamriel 5,-3")),
            ]
        );
    }

    #[test]
    fn a_plugin_that_changes_no_navmesh_has_no_changes() {
        let plugin = fixture::plugin(&["Skyrim.esm"], 0, &[]);
        assert_eq!(changes("Quiet.esp", &read(&plugin)), None);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_records(
            signature in prop::sample::select(vec![*b"CELL", *b"WRLD", NAVMESH]),
            form_id in any::<u32>(),
            flags in any::<u32>(),
            data in prop::collection::vec(any::<u8>(), 0..32),
        ) {
            let plugin = Plugin {
                records: vec![Record {
                    signature,
                    form_id,
                    flags,
                    subrecords: vec![plugin::Subrecord { signature: *b"XCLC", data }],
                    groups: Vec::new(),
                }],
                ..Plugin::default()
            };
            let _ = changes("Any.esp", &plugin);
        }
    }
}
