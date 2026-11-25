//! Finding cells in a plugin: where a coordinate lies, which groups may
//! hold a cell, and which worldspace or cell a record belongs to.
//!
//! A plugin keeps the outside cells of a worldspace in blocks of 32 × 32
//! cells and sub-blocks of 8 × 8, each labelled with its place; a cell's
//! children (what is placed in it) follow the cell in groups of their
//! own. The readers of areas, walkways, moves and navigation meshes all
//! read only the cells they need, and decide which groups to enter here.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, GroupLabel, Plugin, Record, group_type};

use crate::records::record_id;

/// The groups that hold what is placed in a cell.
pub(crate) const CHILDREN: [u32; 3] = [
    group_type::CELL_CHILDREN,
    group_type::CELL_PERSISTENT_CHILDREN,
    group_type::CELL_TEMPORARY_CHILDREN,
];

/// The place of a block or sub-block of outside cells: its size in cells
/// and its column and row in blocks of that size. `None` for every other
/// group.
fn block(group: GroupLabel) -> Option<(i32, i32, i32)> {
    let size = match group.group_type {
        group_type::EXTERIOR_BLOCK => 32,
        group_type::EXTERIOR_SUB_BLOCK => 8,
        _ => return None,
    };
    let [y0, y1, x0, x1] = group.label.to_le_bytes();
    let x = i32::from(i16::from_le_bytes([x0, x1]));
    let y = i32::from(i16::from_le_bytes([y0, y1]));
    Some((size, x, y))
}

/// Whether a group may hold one of `cells` (column, row). Groups that are
/// not blocks are entered.
pub(crate) fn holds_any(group: GroupLabel, cells: &BTreeSet<(i32, i32)>) -> bool {
    let Some((size, x, y)) = block(group) else {
        return true;
    };
    cells
        .iter()
        .any(|(cx, cy)| cx.div_euclid(size) == x && cy.div_euclid(size) == y)
}

/// Whether a group may hold one of the cells `cells` (west, east, south,
/// north, inclusive). Groups that are not blocks are entered.
pub(crate) fn holds_within(group: GroupLabel, cells: (i32, i32, i32, i32)) -> bool {
    let Some((size, x, y)) = block(group) else {
        return true;
    };
    let (west, east, south, north) = cells;
    x <= east.div_euclid(size)
        && x >= west.div_euclid(size)
        && y <= north.div_euclid(size)
        && y >= south.div_euclid(size)
}

/// Whether a group may hold the cells around `points` (cell columns and
/// rows): each cell and its neighbours, for objects on an edge. Groups
/// that are not blocks are entered.
pub(crate) fn holds_near(group: GroupLabel, points: &[(i32, i32)]) -> bool {
    let Some((size, x, y)) = block(group) else {
        return true;
    };
    points.iter().any(|&(px, py)| {
        (px - 1..=px + 1).any(|cx| cx.div_euclid(size) == x)
            && (py - 1..=py + 1).any(|cy| cy.div_euclid(size) == y)
    })
}

/// The grid position of an outside cell (`XCLC`); `None` for a cell
/// inside.
pub(crate) fn cell_position(record: &Record) -> Option<(i32, i32)> {
    let position = record.first(b"XCLC")?;
    let x = i32::from_le_bytes(position.get(0..4)?.try_into().ok()?);
    let y = i32::from_le_bytes(position.get(4..8)?.try_into().ok()?);
    Some((x, y))
}

/// The worldspace a record lies in, if it is outside.
pub(crate) fn world_of(record: &Record, masters: &[String], plugin_name: &str) -> Option<RecordId> {
    record
        .groups
        .iter()
        .find(|group| group.group_type == group_type::WORLD_CHILDREN)
        .map(|group| record_id(group.label, masters, plugin_name))
}

/// The worldspace whose land the worldspace `record` uses (its parent,
/// `WNAM`, if its parent flags, `PNAM`, say so), if any.
pub(crate) fn land_parent(
    record: &Record,
    masters: &[String],
    plugin_name: &str,
) -> Option<RecordId> {
    let flags = u16::from_le_bytes(record.first(b"PNAM")?.get(..2)?.try_into().ok()?);
    let parent = u32::from_le_bytes(record.first(b"WNAM")?.get(..4)?.try_into().ok()?);
    (flags & 0x01 != 0 && parent != 0).then(|| record_id(parent, masters, plugin_name))
}

/// The plugin at `path`, read as `request` asks but only through the
/// nested groups `descend` accepts; `None` if it cannot be read.
pub(crate) fn read_where(
    path: &Path,
    request: &plugin::Request<'_>,
    descend: &dyn Fn(GroupLabel) -> bool,
) -> Option<Plugin> {
    let file = File::open(path).ok()?;
    plugin::read_request_where(&mut BufReader::new(file), request, descend).ok()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn block_label(group_type: u32, x: i16, y: i16) -> GroupLabel {
        let [y0, y1] = y.to_le_bytes();
        let [x0, x1] = x.to_le_bytes();
        GroupLabel {
            group_type,
            label: u32::from_le_bytes([y0, y1, x0, x1]),
        }
    }

    #[test]
    fn only_blocks_around_the_point_are_entered() {
        // Cells 4 to 6 east, -2 to 0 north: block (0, -1) and (0, 0).
        let cells = (4, 6, -2, 0);
        assert!(holds_within(
            block_label(group_type::EXTERIOR_BLOCK, 0, -1),
            cells
        ));
        assert!(holds_within(
            block_label(group_type::EXTERIOR_BLOCK, 0, 0),
            cells
        ));
        assert!(!holds_within(
            block_label(group_type::EXTERIOR_BLOCK, 1, 0),
            cells
        ));
        assert!(!holds_within(
            block_label(group_type::EXTERIOR_BLOCK, -1, -1),
            cells
        ));
        // Sub-blocks of 8 cells: x 0 (cells 0 to 7), y -1 (cells -8 to -1).
        assert!(holds_within(
            block_label(group_type::EXTERIOR_SUB_BLOCK, 0, -1),
            cells
        ));
        assert!(!holds_within(
            block_label(group_type::EXTERIOR_SUB_BLOCK, 1, -1),
            cells
        ));
        // Cells and their children are always entered.
        assert!(holds_within(
            block_label(group_type::CELL_CHILDREN, 99, 99),
            cells
        ));
    }

    #[test]
    fn the_three_block_tests_agree_on_one_cell() {
        let cell = (40, -9);
        let set = BTreeSet::from([cell]);
        for (group_type, x, y) in [
            (group_type::EXTERIOR_BLOCK, 1, -1),
            (group_type::EXTERIOR_SUB_BLOCK, 5, -2),
        ] {
            let group = block_label(group_type, x, y);
            assert!(holds_any(group, &set));
            assert!(holds_within(group, (cell.0, cell.0, cell.1, cell.1)));
            assert!(holds_near(group, &[cell]));
        }
        let elsewhere = block_label(group_type::EXTERIOR_BLOCK, 3, 3);
        assert!(!holds_any(elsewhere, &set));
        assert!(!holds_within(elsewhere, (cell.0, cell.0, cell.1, cell.1)));
        assert!(!holds_near(elsewhere, &[cell]));
    }

    #[test]
    fn a_point_on_the_edge_of_a_block_enters_its_neighbour() {
        // Cell 31 lies in block 0; its neighbour 32 in block 1.
        assert!(holds_near(
            block_label(group_type::EXTERIOR_BLOCK, 1, 0),
            &[(31, 0)]
        ));
        assert!(!holds_any(
            block_label(group_type::EXTERIOR_BLOCK, 1, 0),
            &BTreeSet::from([(31, 0)])
        ));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_groups(
            group_type in any::<u32>(),
            label in any::<u32>(),
            x in any::<i32>(),
            y in any::<i32>(),
        ) {
            let group = GroupLabel { group_type, label };
            let (x, y) = (x / 2, y / 2);
            let _ = holds_any(group, &BTreeSet::from([(x, y)]));
            let _ = holds_within(group, (x, x, y, y));
            let _ = holds_near(group, &[(x, y)]);
        }
    }

    #[test]
    fn a_city_uses_its_parents_land_only_if_its_flags_say_so() {
        let world = |fields: Vec<(&[u8; 4], Vec<u8>)>| Record {
            signature: *b"WRLD",
            form_id: 0x1A26F,
            flags: 0,
            subrecords: fields
                .into_iter()
                .map(|(signature, data)| plugin::Subrecord {
                    signature: *signature,
                    data,
                })
                .collect(),
            groups: Vec::new(),
        };
        let masters = vec!["Skyrim.esm".to_owned()];
        let tamriel = RecordId {
            plugin: "skyrim.esm".to_owned(),
            local_id: 0x3C,
        };
        let city = world(vec![
            (b"WNAM", 0x3Cu32.to_le_bytes().to_vec()),
            (b"PNAM", 0x5Fu16.to_le_bytes().to_vec()),
        ]);
        assert_eq!(land_parent(&city, &masters, "Mod.esp"), Some(tamriel));
        let own_land = world(vec![
            (b"WNAM", 0x3Cu32.to_le_bytes().to_vec()),
            (b"PNAM", 0x04u16.to_le_bytes().to_vec()),
        ]);
        assert_eq!(land_parent(&own_land, &masters, "Mod.esp"), None);
        assert_eq!(land_parent(&world(Vec::new()), &masters, "Mod.esp"), None);
    }

    proptest::proptest! {
        #[test]
        fn a_parent_never_panics_on_arbitrary_fields(
            wnam in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..6),
            pnam in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..3),
        ) {
            let record = Record {
                signature: *b"WRLD",
                form_id: 1,
                flags: 0,
                subrecords: vec![
                    plugin::Subrecord { signature: *b"WNAM", data: wnam },
                    plugin::Subrecord { signature: *b"PNAM", data: pnam },
                ],
                groups: Vec::new(),
            };
            let _ = land_parent(&record, &[], "Mod.esp");
        }
    }
}
