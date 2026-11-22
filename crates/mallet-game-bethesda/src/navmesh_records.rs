//! The records of the navigation meshes Mod Mallet repairs (ADR 0058), to
//! be written into a patch plugin as overrides.
//!
//! A repaired mesh is a copy of the version the game uses, with its
//! `NVNM` field changed as the repair says: triangles taken out lead
//! nowhere and leave the search grid, new triangles follow the old ones,
//! and edges that lead onto other meshes get links. Everything the repair
//! does not change stays byte for byte. Nothing is written here: the
//! records are handed to the plugin writer, which places them in the
//! groups of their cell and worldspace and rewrites their form IDs for
//! the patch's masters with [`NavmeshRecord::fields_for`].

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{Beyond, NavmeshShape};
use mallet_core::facts::objects::Place;
use mallet_formats_bethesda::navmesh_write::{EdgeLink, NONE, NavmeshData, Triangle, flags};
use mallet_formats_bethesda::plugin::{self, GroupLabel, Signature, Subrecord, group_type};

use crate::navmeshes::NAVMESH;
use crate::objects::record_id;
use crate::walkways::{Around, CHILDREN, holds_any, in_world};

/// A navigation mesh repaired, ready to be written as an override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavmeshRecord {
    /// The mesh, by the plugin that first defined it.
    pub navmesh: RecordId,
    /// The plugin whose version was copied: the one the game used.
    pub plugin: String,
    /// That plugin's masters, which the form IDs in [`Self::fields`] refer
    /// to.
    pub masters: Vec<String>,
    /// The record's flags in that plugin. Bit `0x0004_0000` says that it
    /// stored the record compressed; [`Self::fields`] are not.
    pub flags: u32,
    /// The record format version it was written with.
    pub form_version: u16,
    /// Where the mesh lies: its worldspace outside, its cell inside.
    pub place: Place,
    /// The cell whose children the record belongs to.
    pub cell: RecordId,
    /// Outside, the cell's column and row.
    pub grid: Option<(i32, i32)>,
    /// The record's fields in order, the `NVNM` field repaired, with form
    /// IDs as [`Self::plugin`] writes them. A field may be longer than
    /// 65535 bytes; the writer then needs an `XXXX` field before it.
    pub fields: Vec<Subrecord>,
}

impl NavmeshRecord {
    /// The fields with every form ID rewritten for a plugin named `own`
    /// whose masters are `masters`. `None` if a record they name belongs
    /// to a plugin that is neither.
    pub fn fields_for(&self, masters: &[String], own: &str) -> Option<Vec<Subrecord>> {
        self.fields
            .iter()
            .map(|field| {
                if &field.signature != b"NVNM" {
                    return Some(field.clone());
                }
                let mut data = NavmeshData::read(&field.data)?;
                for form_id in data.form_ids_mut() {
                    let id = record_id(*form_id, &self.masters, &self.plugin);
                    *form_id = form_id_in(&id, masters, own)?;
                }
                Some(Subrecord {
                    signature: field.signature,
                    data: data.write()?,
                })
            })
            .collect()
    }
}

/// One mesh as the repair leaves it, in the analyses' representation.
#[derive(Debug, Clone, Copy)]
pub struct Repaired<'a> {
    /// The version the game uses, as read.
    pub before: &'a NavmeshShape,
    /// The same after the repair: the same triangles in the same order,
    /// new ones after them.
    pub after: &'a NavmeshShape,
    /// The triangles taken out.
    pub removed: &'a BTreeSet<u32>,
    /// The new triangles that lie in water.
    pub water: &'a BTreeSet<u32>,
}

/// A record as read, with its form version.
type Versioned = (plugin::Record, u16);

/// The records `request` asks for in the plugin at `path`, through the
/// groups `descend` enters, each with its form version, and the plugin's
/// masters.
fn read_versions(
    path: &std::path::Path,
    request: &plugin::Request<'_>,
    descend: &dyn Fn(GroupLabel) -> bool,
) -> Option<(Vec<String>, Vec<Versioned>)> {
    let file = std::fs::File::open(path).ok()?;
    let mut records = Vec::new();
    let (header, _) = plugin::read_request_versions(
        &mut std::io::BufReader::new(file),
        request,
        descend,
        &mut |_, record, version| records.push((record, version)),
    )
    .ok()?;
    Some((header.masters, records))
}

/// One pass over a plugin: the top-level group, which nested groups to
/// enter, and whether it is needed.
type Pass<'a> = (Signature, &'a dyn Fn(GroupLabel) -> bool, bool);

/// The records of the meshes in `repaired`, read from `plugins` (the
/// active plugins in load order, by name and file) in the cells `around`,
/// and repaired. A mesh whose record cannot be read, or whose field does
/// not hold together with what the analyses read, is left out.
pub fn repaired_records(
    plugins: &[(String, PathBuf)],
    around: &Around,
    repaired: &[Repaired<'_>],
) -> Vec<NavmeshRecord> {
    let sources: BTreeSet<String> = repaired
        .iter()
        .map(|mesh| mesh.before.plugin.to_lowercase())
        .collect();
    let all_cells: BTreeSet<(i32, i32)> = around.worlds.values().flatten().copied().collect();
    let outside = |group: GroupLabel| holds_any(group, &all_cells);
    // A plugin labels a cell's children by its form ID as it writes it:
    // the number within the plugin decides which children are entered.
    let numbers: BTreeSet<u32> = around.interiors.iter().map(|cell| cell.local_id).collect();
    let inside = |group: GroupLabel| {
        !CHILDREN.contains(&group.group_type) || numbers.contains(&(group.label & 0x00FF_FFFF))
    };
    let passes: [Pass<'_>; 2] = [
        (*b"WRLD", &outside, !all_cells.is_empty()),
        (*b"CELL", &inside, !numbers.is_empty()),
    ];
    let mut records = Vec::new();
    for (name, path) in plugins {
        if !sources.contains(&name.to_lowercase()) {
            continue;
        }
        for (top, descend, wanted) in &passes {
            if !wanted {
                continue;
            }
            let request = plugin::Request {
                flat: &[],
                nested: std::slice::from_ref(top),
                nested_records: &[*b"CELL", NAVMESH],
                nested_overrides: &[],
            };
            let Some((masters, read)) = read_versions(path, &request, *descend) else {
                continue;
            };
            records.extend(records_of(name, &masters, &read, repaired));
        }
    }
    records
}

/// The repaired records of `repaired` among those `read` from the plugin
/// `name`.
fn records_of(
    name: &str,
    masters: &[String],
    read: &[Versioned],
    repaired: &[Repaired<'_>],
) -> Vec<NavmeshRecord> {
    let mut grids: HashMap<RecordId, (i32, i32)> = HashMap::new();
    let mut records = Vec::new();
    for (record, form_version) in read {
        if &record.signature == b"CELL" {
            if let Some(position) = crate::terrain::cell_position(record) {
                grids.insert(record_id(record.form_id, masters, name), position);
            }
            continue;
        }
        let id = record_id(record.form_id, masters, name);
        let Some(mesh) = repaired
            .iter()
            .find(|mesh| mesh.before.id == id && mesh.before.plugin.eq_ignore_ascii_case(name))
        else {
            continue;
        };
        let Some(cell) = record
            .groups
            .iter()
            .find(|group| group.group_type == group_type::CELL_CHILDREN)
            .map(|group| record_id(group.label, masters, name))
        else {
            continue;
        };
        let Some(mut data) = record.first(b"NVNM").and_then(NavmeshData::read) else {
            continue;
        };
        let own_id = |id: &RecordId| form_id_in(id, masters, name);
        if apply(&mut data, mesh, &own_id).is_none() {
            continue;
        }
        let Some(written) = data.write() else {
            continue;
        };
        let fields = record
            .subrecords
            .iter()
            .map(|field| Subrecord {
                signature: field.signature,
                data: if &field.signature == b"NVNM" {
                    written.clone()
                } else {
                    field.data.clone()
                },
            })
            .collect();
        let place = match in_world(record, masters, name) {
            Some(worldspace) => Place::Exterior { worldspace },
            None => Place::Interior { cell: cell.clone() },
        };
        let grid = matches!(place, Place::Exterior { .. })
            .then(|| grids.get(&cell).copied())
            .flatten();
        records.push(NavmeshRecord {
            navmesh: id,
            plugin: name.to_owned(),
            masters: masters.to_vec(),
            flags: record.flags,
            form_version: *form_version,
            place,
            cell,
            grid,
            fields,
        });
    }
    records
}

/// The form ID of `id` in a plugin named `own` with `masters`.
pub(crate) fn form_id_in(id: &RecordId, masters: &[String], own: &str) -> Option<u32> {
    let index = masters
        .iter()
        .position(|master| master.eq_ignore_ascii_case(&id.plugin))
        .or_else(|| {
            own.eq_ignore_ascii_case(&id.plugin)
                .then_some(masters.len())
        })?;
    let index = u32::try_from(index).ok().filter(|index| *index < 0xFF)?;
    Some((index << 24) | (id.local_id & 0x00FF_FFFF))
}

/// Changes `data`, the field of `mesh.before`, as the repair says. `None`
/// if the field and the analyses' reading of it do not agree.
fn apply(
    data: &mut NavmeshData,
    mesh: &Repaired<'_>,
    form_id: &dyn Fn(&RecordId) -> Option<u32>,
) -> Option<()> {
    let (before, after) = (mesh.before, mesh.after);
    if data.triangles.len() != before.triangles.len()
        || data.vertices.len() != before.vertices.len()
        || after.triangles.len() < before.triangles.len()
        || after.vertices.len() < before.vertices.len()
        || after.edges.len() != after.triangles.len()
    {
        return None;
    }
    // Vertices: those that moved, and the new ones.
    for (index, vertex) in after.vertices.iter().enumerate() {
        let value = vertex.map(as_f32);
        match data.vertices.get_mut(index) {
            Some(slot) => {
                if before.vertices.get(index) != Some(vertex) {
                    *slot = value;
                }
            }
            None => data.vertices.push(value),
        }
    }
    // Triangles: edges that changed, the removed ones, the new ones.
    let old_count = before.triangles.len();
    for (index, (corners, edges)) in after.triangles.iter().zip(&after.edges).enumerate() {
        let number = u32::try_from(index).ok()?;
        let vertices = corners.map(|corner| u16::try_from(corner).unwrap_or(NONE));
        if vertices.contains(&NONE) {
            return None;
        }
        if index < old_count {
            let unchanged = before.edges.get(index) == Some(edges)
                && before.triangles.get(index) == Some(corners);
            if unchanged {
                continue;
            }
            let (raw, links) = encode(data, edges, form_id)?;
            let triangle = data.triangles.get_mut(index)?;
            triangle.vertices = vertices;
            triangle.edges = raw;
            triangle.flags = (triangle.flags & !flags::EDGE_LINKS) | links;
        } else {
            let (raw, links) = encode(data, edges, form_id)?;
            let water = if mesh.water.contains(&number) {
                flags::WATER
            } else {
                0
            };
            data.triangles.push(Triangle {
                vertices,
                edges: raw,
                flags: flags::FOUND | water | links,
                cover: 0,
            });
        }
    }
    let removed: BTreeSet<u16> = mesh
        .removed
        .iter()
        .filter_map(|triangle| u16::try_from(*triangle).ok())
        .collect();
    data.doors.retain(|door| !removed.contains(&door.triangle));
    data.cover_triangles
        .retain(|triangle| !removed.contains(triangle));
    for cell in &mut data.grid.cells {
        cell.retain(|triangle| !removed.contains(triangle));
    }
    for index in old_count..after.triangles.len() {
        data.add_to_grid(u16::try_from(index).ok()?)?;
    }
    data.fit_grid_height();
    Some(())
}

/// The three edges as written, and the flags that say which are links;
/// links the field lacks are added.
fn encode(
    data: &mut NavmeshData,
    edges: &[Beyond; 3],
    form_id: &dyn Fn(&RecordId) -> Option<u32>,
) -> Option<([u16; 3], u16)> {
    let mut raw = [NONE; 3];
    let mut links = 0u16;
    for ((edge, beyond), slot) in edges.iter().enumerate().zip(raw.iter_mut()) {
        *slot = match beyond {
            Beyond::Nothing => NONE,
            Beyond::Triangle(triangle) => u16::try_from(*triangle)
                .ok()
                .filter(|value| *value != NONE)?,
            Beyond::Link { navmesh, triangle } => {
                let navmesh = form_id(navmesh)?;
                let triangle = u16::try_from(*triangle).ok()?;
                let known = data
                    .edge_links
                    .iter()
                    .position(|link| link.navmesh == navmesh && link.triangle == triangle);
                let index = known.unwrap_or(data.edge_links.len());
                if known.is_none() {
                    data.edge_links.push(EdgeLink {
                        kind: 0,
                        navmesh,
                        triangle,
                    });
                }
                links |= 1 << edge;
                u16::try_from(index).ok().filter(|value| *value != NONE)?
            }
        };
    }
    Some((raw, links))
}

/// A whole game unit as a float.
#[allow(clippy::cast_precision_loss)]
fn as_f32(value: i32) -> f32 {
    // Coordinates of the games are far below 2^24.
    value as f32
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
pub(crate) mod tests {
    use std::fs::File;
    use std::io::Write;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use mallet_formats_bethesda::navmesh_write::{DoorTriangle, Grid, SKYRIM_VERSION};
    use proptest::prelude::*;

    use super::*;
    use crate::walkways::read_plugins;

    /// The docks' mesh as `Docks.esp` writes it (masters: `Skyrim.esm`):
    /// two triangles side by side on the ground, a door on the second,
    /// which also leads onto triangle 5 of the game's mesh `0x0000_0AAA`.
    pub(crate) fn docks() -> NavmeshData {
        NavmeshData {
            version: SKYRIM_VERSION,
            checksum: 0xA5E9_A03C,
            worldspace: 0x0000_003C,
            parent: u32::from_le_bytes([3, 0, 5, 0]),
            vertices: vec![
                [20_480.0, 12_288.0, 10.5],
                [20_580.0, 12_288.0, 10.0],
                [20_580.0, 12_388.0, 10.0],
                [20_480.0, 12_388.0, 10.0],
            ],
            triangles: vec![
                Triangle {
                    vertices: [0, 1, 2],
                    edges: [NONE, NONE, 1],
                    flags: flags::FOUND,
                    cover: 0x0011,
                },
                Triangle {
                    vertices: [0, 2, 3],
                    edges: [0, 0, NONE],
                    flags: flags::FOUND | 0b010,
                    cover: 0,
                },
            ],
            edge_links: vec![EdgeLink {
                kind: 0,
                navmesh: 0x0000_0AAA,
                triangle: 5,
            }],
            doors: vec![DoorTriangle {
                triangle: 1,
                kind: 0xE48A_0A33,
                door: 0x0100_0900,
            }],
            cover_triangles: vec![0],
            grid: Grid {
                size: 1,
                square: [100.0, 100.0],
                min: [20_480.0, 12_288.0, 10.0],
                max: [20_580.0, 12_388.0, 10.5],
                cells: vec![vec![0, 1]],
            },
            rest: Vec::new(),
        }
    }

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    fn grid(x: i32, y: i32) -> Vec<u8> {
        let mut data = x.to_le_bytes().to_vec();
        data.extend(y.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data
    }

    /// The game and `Docks.esp`, which changes the game's mesh 0x210 in
    /// cell 5,3 of Tamriel; written into a folder.
    pub(crate) fn setup() -> (tempfile::TempDir, Vec<(String, PathBuf)>) {
        let world =
            RecordSpec::new(*b"WRLD", 0x0000_003C).with(*b"EDID", fixture::zstring("Tamriel"));
        let cell = || RecordSpec::new(*b"CELL", 0x0000_0200).with(*b"XCLC", grid(5, 3));
        let mesh = |data: Vec<u8>| {
            RecordSpec::new(*b"NAVM", 0x0000_0210)
                .with(*b"NVNM", data)
                .compressed()
        };
        let mut game_mesh = docks();
        game_mesh.doors.clear();
        game_mesh.edge_links.clear();
        game_mesh.triangles[1].edges = [0, NONE, NONE];
        game_mesh.triangles[1].flags = flags::FOUND;
        let game = fixture::plugin_with_raw_groups(
            &[],
            0,
            &[fixture::worldspace(
                &world,
                &[(cell(), vec![mesh(game_mesh.write().unwrap())])],
            )],
        );
        let docks = fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[fixture::worldspace(
                &world,
                &[(cell(), vec![mesh(docks().write().unwrap())])],
            )],
        );
        let directory = tempfile::tempdir().unwrap();
        let mut plugins = Vec::new();
        for (name, data) in [("Skyrim.esm", game), ("Docks.esp", docks)] {
            let path = directory.path().join(name);
            File::create(&path).unwrap().write_all(&data).unwrap();
            plugins.push((name.to_owned(), path));
        }
        (directory, plugins)
    }

    pub(crate) fn around() -> Around {
        Around {
            worlds: std::collections::BTreeMap::from([(
                "Tamriel".to_owned(),
                BTreeSet::from([(5, 3)]),
            )]),
            interiors: BTreeSet::new(),
        }
    }

    #[test]
    fn a_repaired_mesh_is_a_copy_of_the_used_version_with_its_repair() {
        let (_directory, plugins) = setup();
        let near = read_plugins(&plugins, &around());
        let before = near.navmeshes.iter().find(|shape| shape.used).unwrap();
        assert_eq!(before.plugin, "Docks.esp");

        // Triangle 1 is taken out; a new triangle beyond triangle 0's
        // first edge leads onto triangle 0 and onto triangle 9 of the
        // game's mesh 0xBBB.
        let mut after = before.clone();
        after.edges[0][2] = Beyond::Nothing;
        after.edges[1] = [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing];
        after.doors.clear();
        after.vertices.push([20_530, 12_200, 0]);
        after.triangles.push([1, 0, 4]);
        after.edges[0][0] = Beyond::Triangle(2);
        after.edges.push([
            Beyond::Triangle(0),
            Beyond::Link {
                navmesh: id("skyrim.esm", 0xBBB),
                triangle: 9,
            },
            Beyond::Nothing,
        ]);
        let removed = BTreeSet::from([1]);
        let water = BTreeSet::from([2]);
        let records = repaired_records(
            &plugins,
            &around(),
            &[Repaired {
                before,
                after: &after,
                removed: &removed,
                water: &water,
            }],
        );
        assert_eq!(records.len(), 1);
        let record = &records[0];
        assert_eq!(record.navmesh, id("skyrim.esm", 0x210));
        assert_eq!(record.plugin, "Docks.esp");
        assert_eq!(record.masters, vec!["Skyrim.esm".to_owned()]);
        assert_eq!(record.cell, id("skyrim.esm", 0x200));
        assert_eq!(record.grid, Some((5, 3)));
        assert_eq!(
            record.place,
            Place::Exterior {
                worldspace: id("skyrim.esm", 0x3C)
            }
        );
        assert_ne!(record.flags & 0x0004_0000, 0);

        let field = record
            .fields
            .iter()
            .find(|field| &field.signature == b"NVNM")
            .unwrap();
        let data = NavmeshData::read(&field.data).unwrap();
        let original = docks();
        // Untouched: the first vertex bit for bit, the cover of triangle 0.
        assert_eq!(data.vertices[0], original.vertices[0]);
        assert_eq!(data.vertices[4], [20_530.0, 12_200.0, 0.0]);
        assert_eq!(data.triangles[0].cover, 0x0011);
        assert_eq!(data.triangles[0].edges, [2, NONE, NONE]);
        // Taken out: no edges, no link flag, no door, in no square.
        assert_eq!(data.triangles[1].edges, [NONE; 3]);
        assert_eq!(data.triangles[1].flags & flags::EDGE_LINKS, 0);
        assert_eq!(data.doors, Vec::new());
        assert!(data.grid.cells.iter().all(|cell| !cell.contains(&1)));
        // New: water, a new link, in the grid; the box reaches down to it.
        let new = data.triangles[2];
        assert_eq!(new.vertices, [1, 0, 4]);
        assert_eq!(new.flags, flags::FOUND | flags::WATER | 0b010);
        assert_eq!(new.edges, [0, 1, NONE]);
        assert_eq!(
            data.edge_links[1],
            EdgeLink {
                kind: 0,
                navmesh: 0x0000_0BBB,
                triangle: 9
            }
        );
        assert!(data.grid.cells[0].contains(&2));
        assert_eq!(data.grid.min[2], 0.0);
        // What the analyses read back is the repair.
        let geometry = mallet_formats_bethesda::navmesh::geometry(&field.data).unwrap();
        assert_eq!(geometry.triangles.len(), 3);

        // For a patch whose masters are the game and the docks, the docks'
        // own records move to the second master.
        let fields = record
            .fields_for(
                &["Skyrim.esm".to_owned(), "Docks.esp".to_owned()],
                "Patch.esp",
            )
            .unwrap();
        let patched = NavmeshData::read(&fields[0].data).unwrap();
        assert_eq!(patched.worldspace, 0x0000_003C);
        assert_eq!(patched.edge_links[1].navmesh, 0x0000_0BBB);
        // A plugin without the game among its masters cannot hold it.
        assert_eq!(record.fields_for(&[], "Patch.esp"), None);
    }

    #[test]
    fn a_repair_that_does_not_fit_the_field_is_left_out() {
        let (_directory, plugins) = setup();
        let near = read_plugins(&plugins, &around());
        let before = near.navmeshes.iter().find(|shape| shape.used).unwrap();
        let mut after = before.clone();
        after.triangles.pop();
        after.edges.pop();
        let empty = BTreeSet::new();
        let records = repaired_records(
            &plugins,
            &around(),
            &[Repaired {
                before,
                after: &after,
                removed: &empty,
                water: &empty,
            }],
        );
        assert_eq!(records, Vec::new());
    }

    #[test]
    fn form_ids_name_the_plugin_by_its_place_among_the_masters() {
        let masters = ["Skyrim.esm".to_owned(), "Docks.esp".to_owned()];
        assert_eq!(
            form_id_in(&id("docks.esp", 0x900), &masters, "Patch.esp"),
            Some(0x0100_0900)
        );
        assert_eq!(
            form_id_in(&id("patch.esp", 0x800), &masters, "Patch.esp"),
            Some(0x0200_0800)
        );
        assert_eq!(
            form_id_in(&id("other.esp", 0x800), &masters, "Patch.esp"),
            None
        );
    }

    proptest! {
        #[test]
        fn never_panics_on_odd_repairs(
            removed in prop::collection::btree_set(0u32..4, 0..4),
            corners in prop::collection::vec(any::<[u8; 3]>(), 0..4),
            edges in prop::collection::vec(0u32..6, 0..12),
        ) {
            let mut data = docks();
            let written = data.write().unwrap();
            let geometry = mallet_formats_bethesda::navmesh::geometry(&written).unwrap();
            let before = NavmeshShape {
                id: id("skyrim.esm", 0x210),
                plugin: "Docks.esp".to_owned(),
                used: true,
                place: Place::Exterior { worldspace: id("skyrim.esm", 0x3C) },
                place_name: None,
                vertices: geometry.vertices.iter().map(|v| v.map(|value| value.round() as i32)).collect(),
                triangles: geometry.triangles.iter().map(|t| t.map(u32::from)).collect(),
                edges: vec![[Beyond::Nothing, Beyond::Nothing, Beyond::Nothing]; 2],
                doors: Vec::new(),
            };
            let mut after = before.clone();
            for corner in &corners {
                after.triangles.push(corner.map(|value| u32::from(value % 6)));
                after.edges.push([Beyond::Nothing, Beyond::Nothing, Beyond::Nothing]);
            }
            for (slot, edge) in after.edges.iter_mut().flatten().zip(&edges) {
                *slot = Beyond::Triangle(*edge);
            }
            let water = BTreeSet::new();
            let mesh = Repaired { before: &before, after: &after, removed: &removed, water: &water };
            let _ = apply(&mut data, &mesh, &|id| form_id_in(id, &["Skyrim.esm".to_owned()], "Docks.esp"));
            let _ = data.write();
        }
    }
}
