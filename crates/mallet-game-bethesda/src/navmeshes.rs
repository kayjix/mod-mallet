//! Which navigation meshes of other plugins a plugin changes, and which
//! doors its own connect.
//!
//! A navigation mesh (`NAVM`) lies in the children of its cell, like the
//! objects placed there. Of its geometry only the doors characters walk
//! through are read ([`navmesh::doors`]). A mesh's place is named after
//! its cell: the cell's internal name inside, the worldspace's
//! internal name and the cell's grid position (`XCLC`) outside, which is
//! what the game's console takes to go there.

use std::collections::HashMap;

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{NavmeshChange, NavmeshChanges};
use mallet_formats_bethesda::navmesh;
use mallet_formats_bethesda::plugin::{self, Plugin, Record, Signature, group_type};

use crate::records::{FLAG_DELETED, editor_id, record_id};

/// The record type of navigation meshes.
pub const NAVMESH: Signature = *b"NAVM";

/// The records read from the groups of places for the navigation meshes.
pub const RECORDS: [Signature; 1] = [NAVMESH];

/// The navigation meshes of other plugins that `plugin`, named
/// `plugin_name`, changes, and the doors its own connect; `None` if there
/// are neither. The plugin must have
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

    let mut connected = Vec::new();
    let navmeshes: Vec<NavmeshChange> = plugin
        .records
        .iter()
        .filter(|record| record.signature == NAVMESH && record.flags & FLAG_DELETED == 0)
        .filter_map(|record| {
            let (origin, local_id) = plugin::resolve_form_id(record.form_id, masters, plugin_name);
            if origin.eq_ignore_ascii_case(plugin_name) {
                // Its own: only the doors matter.
                connected.extend(doors(record, masters, plugin_name).into_iter().flatten());
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
                doors: doors(record, masters, plugin_name),
            })
        })
        .collect();
    (!navmeshes.is_empty() || !connected.is_empty()).then(|| NavmeshChanges {
        plugin: plugin_name.to_owned(),
        navmeshes,
        connected,
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

/// The doors a navigation mesh lets characters walk through, if its
/// geometry can be read.
fn doors(record: &Record, masters: &[String], plugin_name: &str) -> Option<Vec<RecordId>> {
    let doors = navmesh::doors(record.first(b"NVNM")?)?;
    Some(
        doors
            .into_iter()
            .map(|door| record_id(door, masters, plugin_name))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use mallet_formats_bethesda::plugin::Request;
    use proptest::prelude::*;

    use super::*;
    use crate::objects::PLACE_GROUPS;

    fn read(data: &[u8]) -> Plugin {
        plugin::read_request(
            &mut Cursor::new(data),
            &Request {
                flat: &[],
                nested: &PLACE_GROUPS,
                nested_records: &[*b"REFR", *b"CELL", *b"WRLD", *b"LAND", NAVMESH],
                nested_overrides: &[],
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

    /// The geometry of a Skyrim navigation mesh with one triangle, through
    /// whose door characters reach `door`.
    fn geometry(door: u32) -> Vec<u8> {
        let mut data = 12u32.to_le_bytes().to_vec();
        data.extend([0; 12]);
        data.extend(1u32.to_le_bytes());
        data.extend([0; 12]);
        data.extend(1u32.to_le_bytes());
        data.extend([0; 16]);
        data.extend(0u32.to_le_bytes());
        data.extend(1u32.to_le_bytes());
        data.extend(0u16.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data.extend(door.to_le_bytes());
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
                            .with(*b"NVNM", geometry(0x0100_0900))
                            .compressed(),
                    ],
                )]),
                fixture::worldspace(
                    &world,
                    &[(
                        field,
                        vec![
                            RecordSpec::new(NAVMESH, 0x0000_0210).with(*b"NVNM", vec![7; 64]),
                            RecordSpec::new(NAVMESH, 0x0100_0800)
                                .with(*b"NVNM", geometry(0x0100_0901)),
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
        // The doors are those of the geometry, by the plugin that placed
        // them; a geometry of another format leaves them unknown.
        let doors: Vec<_> = changes
            .navmeshes
            .iter()
            .map(|change| change.doors.clone())
            .collect();
        assert_eq!(
            doors,
            vec![
                Some(vec![RecordId {
                    plugin: "village.esp".to_owned(),
                    local_id: 0x900,
                }]),
                None,
            ]
        );
        // Its own mesh counts only for the door it connects.
        assert_eq!(
            changes.connected,
            vec![RecordId {
                plugin: "village.esp".to_owned(),
                local_id: 0x901,
            }]
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
