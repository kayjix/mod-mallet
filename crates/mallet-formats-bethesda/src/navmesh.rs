//! Reads which doors a navigation mesh connects to, its triangles and how
//! they hang together, and takes triangles out of the paths (ADR 0052).
//!
//! A navigation mesh (`NAVM`) keeps its geometry in one field, `NVNM`. In
//! Skyrim (format version 12) it is laid out as follows, all numbers little
//! endian, as xEdit defines it:
//!
//! ```text
//! u32 version                      12
//! u32 checksum, u32 worldspace     where the mesh lies
//! 4 bytes                          grid position or cell
//! u32 n, n × 12 bytes              vertices: three floats each
//! u32 n, n × 16 bytes              triangles: three u16 vertices,
//!                                  three u16 edges, u16 flags,
//!                                  u16 cover flags
//! u32 n, n × 10 bytes              links to neighbouring meshes:
//!                                  u32 kind, u32 mesh, u16 triangle
//! u32 n, n × 10 bytes              door triangles: u16 triangle,
//!                                  u32 checksum, u32 door reference
//! ...                              cover, then a search grid
//! ```
//!
//! An edge is the number of the triangle beyond it, or `0xFFFF` for none.
//! Where bit 0, 1 or 2 of the flags is set for the first, second or third
//! edge, the number is that of a link to a neighbouring mesh instead. A
//! door triangle is where characters step through a door. A door that has
//! none cannot be used by anyone but the player.
//!
//! Fallout 4 (format version 15) keeps the same parts in the same order,
//! with longer entries (ADR 0078):
//!
//! ```text
//! u32 n, n × 21 bytes              triangles: three u16 vertices,
//!                                  three u16 edges, f32 height, u8,
//!                                  u16 flags, u16 cover flags
//! u32 n, n × 11 bytes              links: u32 kind, u32 mesh,
//!                                  u16 triangle, u8
//! u32 n, n × 10 bytes              door triangles, as in Skyrim
//! ...                              cover, waypoints, then a search grid
//! ```
//!
//! The layout was checked against the 7,894 navigation meshes of
//! `Fallout4.esm`: every field ends exactly where its search grid does,
//! the flags' link bits agree with the edges of all 3.66 million
//! triangles, and all 283,269 links name a triangle their mesh has.
//!
//! Both are read; only Skyrim's can be cut ([`cut`], [`unlink`]).
//!
//! Fallout 3 and New Vegas (format version 11, ADR 0092) keep the same
//! parts in fields of their own, without counts in front; `DATA` holds
//! the counts:
//!
//! ```text
//! NVER  u32 version                11
//! DATA  u32 cell, u32 vertices,    the counts of the fields below
//!       u32 triangles, u32 links,
//!       u32 cover triangles, u32 doors
//! NVVX  n × 12 bytes               vertices, as in Skyrim
//! NVTR  n × 16 bytes               triangles, as in Skyrim
//! NVCA  n × 2 bytes                cover triangles
//! NVDP  n × 8 bytes                door triangles: u32 door reference,
//!                                  u16 triangle, 2 bytes unused
//! NVGD                             the search grid
//! NVEX  n × 10 bytes               links, as in Skyrim
//! ```
//!
//! They are read by [`record_geometry`], which takes either layout.

use std::collections::BTreeSet;

use crate::Bytes;
use crate::plugin::Record;

/// The format version of Skyrim's navigation meshes.
const SKYRIM_VERSION: u32 = 12;

/// The format version of Fallout 4's navigation meshes.
const FALLOUT_4_VERSION: u32 = 15;

/// Bytes per vertex and per door triangle, the same in both versions.
const VERTEX_LEN: usize = 12;
const DOOR_LEN: usize = 10;

/// How long the entries of one version of the field are.
#[derive(Clone, Copy)]
struct Sizes {
    /// Bytes per triangle.
    triangle: usize,
    /// Where in a triangle its flags lie.
    flags_at: usize,
    /// Bytes per link to a neighbouring mesh.
    link: usize,
}

/// Skyrim's entries.
const SKYRIM: Sizes = Sizes {
    triangle: 16,
    flags_at: 12,
    link: 10,
};

/// Fallout 4's entries: a height after the edges, and one more byte
/// before the flags and after each link.
const FALLOUT_4: Sizes = Sizes {
    triangle: 21,
    flags_at: 17,
    link: 11,
};

/// The sizes of the version a field starts with, if it is one this reader
/// knows.
fn sizes_of(version: u32) -> Option<Sizes> {
    match version {
        SKYRIM_VERSION => Some(SKYRIM),
        FALLOUT_4_VERSION => Some(FALLOUT_4),
        _ => None,
    }
}

/// The doors a navigation mesh connects to, by their form IDs as the
/// plugin writes them (relative to its masters), in file order. `None`
/// if the field is not in Skyrim's or Fallout 4's format or does not hold
/// together.
pub fn doors(nvnm: &[u8]) -> Option<Vec<u32>> {
    door_triangles(nvnm).map(|doors| doors.into_iter().map(|(_, door)| door).collect())
}

/// The door triangles of a navigation mesh: each triangle's number and the
/// door's form ID as the plugin writes it, in file order. `None` as for
/// [`doors`].
pub fn door_triangles(nvnm: &[u8]) -> Option<Vec<(u16, u32)>> {
    let mut bytes = Bytes::new(nvnm);
    let sizes = sizes_of(bytes.u32()?)?;
    // Checksum, worldspace, and grid position or cell.
    bytes.skip(12)?;
    skip_array(&mut bytes, VERTEX_LEN)?;
    let triangles = skip_array(&mut bytes, sizes.triangle)?;
    skip_array(&mut bytes, sizes.link)?;

    let count = usize::try_from(bytes.u32()?).ok()?;
    // Each entry needs ten bytes; a count that cannot fit is damage.
    if count > bytes.remaining() / DOOR_LEN {
        return None;
    }
    let mut doors = Vec::with_capacity(count);
    for _ in 0..count {
        let triangle = bytes.u16()?;
        bytes.skip(4)?;
        let door = bytes.u32()?;
        if usize::from(triangle) >= triangles {
            return None;
        }
        doors.push((triangle, door));
    }
    Some(doors)
}

/// What lies beyond one edge of a triangle of a navigation mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beyond {
    /// Nothing characters can step onto: a wall or a drop.
    Nothing,
    /// Another triangle of the same mesh, by its number.
    Triangle(u16),
    /// A triangle of a neighbouring mesh.
    Link {
        /// The neighbouring mesh, by its form ID as the plugin writes it
        /// (relative to its masters).
        navmesh: u32,
        /// The triangle, by its number there.
        triangle: u16,
    },
}

/// The surface of a navigation mesh: where characters can walk, and how
/// its triangles hang together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Geometry {
    /// The vertices, in game units, in the coordinates of the worldspace
    /// or the interior cell.
    pub vertices: Vec<[f32; 3]>,
    /// The triangles, three vertex numbers each.
    pub triangles: Vec<[u16; 3]>,
    /// For each triangle, what lies beyond its edges from the first to the
    /// second vertex, the second to the third, and the third to the first.
    pub edges: Vec<[Beyond; 3]>,
    /// The triangles where characters step through a door, with the door
    /// by its form ID as the plugin writes it.
    pub doors: Vec<(u16, u32)>,
}

/// The vertices, triangles, edges and doors of a navigation mesh. `None`
/// if the field is not in Skyrim's or Fallout 4's format or does not hold
/// together, as when a triangle names a vertex the mesh does not have. An
/// edge that names a triangle or link the mesh does not have leads
/// nowhere.
pub fn geometry(nvnm: &[u8]) -> Option<Geometry> {
    let layout = Layout::of(nvnm)?;
    let sizes = layout.sizes;
    let mut bytes = Bytes::new(nvnm.get(layout.vertices..)?);
    let mut vertices = Vec::with_capacity(layout.vertex_count);
    for _ in 0..layout.vertex_count {
        vertices.push([bytes.f32()?, bytes.f32()?, bytes.f32()?]);
    }
    let mut links = Vec::with_capacity(layout.link_count);
    for index in 0..layout.link_count {
        let start = layout.links.checked_add(index.checked_mul(sizes.link)?)?;
        let navmesh = read_u32(nvnm, start.checked_add(4)?)?;
        let triangle = read_u16(nvnm, start.checked_add(8)?)?;
        links.push(Beyond::Link { navmesh, triangle });
    }
    let mut triangles = Vec::with_capacity(layout.triangle_count);
    let mut edges = Vec::with_capacity(layout.triangle_count);
    for index in 0..layout.triangle_count {
        let start = layout
            .triangles
            .checked_add(index.checked_mul(sizes.triangle)?)?;
        let mut bytes = Bytes::new(nvnm.get(start..)?);
        let triangle = [bytes.u16()?, bytes.u16()?, bytes.u16()?];
        if triangle
            .iter()
            .any(|&vertex| usize::from(vertex) >= vertices.len())
        {
            return None;
        }
        let across = [bytes.u16()?, bytes.u16()?, bytes.u16()?];
        let flags = read_u16(nvnm, start.checked_add(sizes.flags_at)?)?;
        let mut beyond = [Beyond::Nothing; 3];
        for ((edge, value), slot) in across.iter().enumerate().zip(beyond.iter_mut()) {
            *slot = if *value == NONE {
                Beyond::Nothing
            } else if flags & (1 << edge) != 0 {
                links
                    .get(usize::from(*value))
                    .copied()
                    .unwrap_or(Beyond::Nothing)
            } else if usize::from(*value) < layout.triangle_count {
                Beyond::Triangle(*value)
            } else {
                Beyond::Nothing
            };
        }
        triangles.push(triangle);
        edges.push(beyond);
    }
    let mut doors = Vec::with_capacity(layout.door_count);
    let mut bytes = Bytes::new(nvnm.get(layout.doors..)?);
    for _ in 0..layout.door_count {
        let triangle = bytes.u16()?;
        bytes.skip(4)?;
        let door = bytes.u32()?;
        if usize::from(triangle) < layout.triangle_count {
            doors.push((triangle, door));
        }
    }
    Some(Geometry {
        vertices,
        triangles,
        edges,
        doors,
    })
}

/// The geometry of a navigation mesh record (`NAVM`) in any layout this
/// reader knows: Skyrim's and Fallout 4's in one field (`NVNM`, see
/// [`geometry`]), or Fallout 3's and New Vegas's in fields of their own
/// (see [`split_geometry`]). `None` as for those.
pub fn record_geometry(record: &Record) -> Option<Geometry> {
    match record.first(b"NVNM") {
        Some(nvnm) => geometry(nvnm),
        None => split_geometry(record),
    }
}

/// The door triangles of a navigation mesh record in any layout this
/// reader knows, as [`door_triangles`] gives them. `None` as for
/// [`record_geometry`].
pub fn record_door_triangles(record: &Record) -> Option<Vec<(u16, u32)>> {
    match record.first(b"NVNM") {
        Some(nvnm) => door_triangles(nvnm),
        None => split_geometry(record).map(|geometry| geometry.doors),
    }
}

/// Bytes per entry of the fields of Fallout 3's and New Vegas's layout.
const SPLIT_TRIANGLE_LEN: usize = 16;
const SPLIT_LINK_LEN: usize = 10;
const SPLIT_DOOR_LEN: usize = 8;

/// The geometry of a navigation mesh of Fallout 3's and New Vegas's
/// layout, whose vertices (`NVVX`), triangles (`NVTR`), links (`NVEX`) and
/// door triangles (`NVDP`) lie in fields of their own. `None` if it has no
/// triangles field, a field does not end on a whole entry, the counts in
/// `DATA` disagree with the fields, or a triangle names a vertex the mesh
/// does not have. An edge that names a triangle or link the mesh does not
/// have leads nowhere, and a door on a triangle it does not have is left
/// out.
pub fn split_geometry(record: &Record) -> Option<Geometry> {
    let field = |signature: &[u8; 4]| record.first(signature).unwrap_or_default();
    let triangles_field = record.first(b"NVTR")?;
    let (vertices_field, links_field, doors_field) =
        (field(b"NVVX"), field(b"NVEX"), field(b"NVDP"));
    let vertex_count = whole_entries(vertices_field, VERTEX_LEN)?;
    let triangle_count = whole_entries(triangles_field, SPLIT_TRIANGLE_LEN)?;
    let link_count = whole_entries(links_field, SPLIT_LINK_LEN)?;
    let door_count = whole_entries(doors_field, SPLIT_DOOR_LEN)?;
    if let Some(data) = record.first(b"DATA").filter(|data| data.len() >= 24) {
        let count = |at: usize| read_u32(data, at).and_then(|n| usize::try_from(n).ok());
        let agrees = [
            (4, vertex_count),
            (8, triangle_count),
            (12, link_count),
            (20, door_count),
        ]
        .iter()
        .all(|&(at, found)| count(at) == Some(found));
        if !agrees {
            return None;
        }
    }

    let mut bytes = Bytes::new(vertices_field);
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        vertices.push([bytes.f32()?, bytes.f32()?, bytes.f32()?]);
    }
    let mut links = Vec::with_capacity(link_count);
    let mut bytes = Bytes::new(links_field);
    for _ in 0..link_count {
        bytes.skip(4)?;
        let navmesh = bytes.u32()?;
        let triangle = bytes.u16()?;
        links.push(Beyond::Link { navmesh, triangle });
    }
    let mut triangles = Vec::with_capacity(triangle_count);
    let mut edges = Vec::with_capacity(triangle_count);
    let mut bytes = Bytes::new(triangles_field);
    for _ in 0..triangle_count {
        let triangle = [bytes.u16()?, bytes.u16()?, bytes.u16()?];
        if triangle
            .iter()
            .any(|&vertex| usize::from(vertex) >= vertices.len())
        {
            return None;
        }
        let across = [bytes.u16()?, bytes.u16()?, bytes.u16()?];
        let flags = bytes.u16()?;
        bytes.skip(2)?;
        let mut beyond = [Beyond::Nothing; 3];
        for ((edge, value), slot) in across.iter().enumerate().zip(beyond.iter_mut()) {
            *slot = if *value == NONE {
                Beyond::Nothing
            } else if flags & (1 << edge) != 0 {
                links
                    .get(usize::from(*value))
                    .copied()
                    .unwrap_or(Beyond::Nothing)
            } else if usize::from(*value) < triangle_count {
                Beyond::Triangle(*value)
            } else {
                Beyond::Nothing
            };
        }
        triangles.push(triangle);
        edges.push(beyond);
    }
    let mut doors = Vec::with_capacity(door_count);
    let mut bytes = Bytes::new(doors_field);
    for _ in 0..door_count {
        let door = bytes.u32()?;
        let triangle = bytes.u16()?;
        bytes.skip(2)?;
        if usize::from(triangle) < triangle_count {
            doors.push((triangle, door));
        }
    }
    Some(Geometry {
        vertices,
        triangles,
        edges,
        doors,
    })
}

/// How many entries of `len` bytes `field` holds, if it ends on a whole
/// one.
fn whole_entries(field: &[u8], len: usize) -> Option<usize> {
    field.len().is_multiple_of(len).then(|| field.len() / len)
}

/// The field with the triangles in `cut` taken out of the paths: nothing
/// lies beyond any of their edges any more, and no other triangle of the
/// mesh leads onto them. They keep their numbers, so that neighbouring
/// meshes, doors and the search grid, which name triangles by number,
/// stay valid; neighbouring meshes that lead onto them need [`unlink`].
/// Everything else stays byte for byte. `None` if the field is not in
/// Skyrim's format or does not hold together.
pub fn cut(nvnm: &[u8], cut: &BTreeSet<u16>) -> Option<Vec<u8>> {
    let layout = Layout::skyrim(nvnm)?;
    let mut data = nvnm.to_vec();
    for index in 0..layout.triangle_count {
        let number = u16::try_from(index).ok()?;
        let start = layout
            .triangles
            .checked_add(index.checked_mul(TRIANGLE_LEN)?)?;
        let flags = read_u16(&data, start.checked_add(FLAGS_AT)?)?;
        if cut.contains(&number) {
            for edge in 0..3 {
                write_u16(&mut data, start + EDGES_AT + 2 * edge, NONE)?;
            }
            write_u16(&mut data, start + FLAGS_AT, flags & !EDGE_LINK_FLAGS)?;
            continue;
        }
        for edge in 0..3 {
            let at = start + EDGES_AT + 2 * edge;
            let value = read_u16(&data, at)?;
            let inside = flags & (1 << edge) == 0;
            if inside && value != NONE && cut.contains(&value) {
                write_u16(&mut data, at, NONE)?;
            }
        }
    }
    Some(data)
}

/// The field with every edge that leads onto the triangles `cut` of the
/// neighbouring mesh `navmesh` (by its form ID as this plugin writes it)
/// leading nowhere: the other half of [`cut`]. Everything else stays byte
/// for byte. `None` if the field is not in Skyrim's format or does not
/// hold together.
pub fn unlink(nvnm: &[u8], navmesh: u32, cut: &BTreeSet<u16>) -> Option<Vec<u8>> {
    let layout = Layout::skyrim(nvnm)?;
    let mut data = nvnm.to_vec();
    for index in 0..layout.triangle_count {
        let start = layout
            .triangles
            .checked_add(index.checked_mul(TRIANGLE_LEN)?)?;
        let mut flags = read_u16(&data, start.checked_add(FLAGS_AT)?)?;
        for edge in 0..3 {
            let at = start + EDGES_AT + 2 * edge;
            let value = read_u16(&data, at)?;
            if flags & (1 << edge) == 0 || value == NONE || usize::from(value) >= layout.link_count
            {
                continue;
            }
            let link = layout.links + usize::from(value) * LINK_LEN;
            let target = read_u32(&data, link + 4)?;
            let triangle = read_u16(&data, link + 8)?;
            if target == navmesh && cut.contains(&triangle) {
                write_u16(&mut data, at, NONE)?;
                flags &= !(1 << edge);
            }
        }
        write_u16(&mut data, start + FLAGS_AT, flags)?;
    }
    Some(data)
}

/// An edge that leads nowhere.
const NONE: u16 = 0xFFFF;

/// Where in a triangle its three edges lie.
const EDGES_AT: usize = 6;

/// Skyrim's sizes, the only ones [`cut`] and [`unlink`] write.
const TRIANGLE_LEN: usize = SKYRIM.triangle;
const FLAGS_AT: usize = SKYRIM.flags_at;
const LINK_LEN: usize = SKYRIM.link;

/// The flags that say an edge leads onto a neighbouring mesh.
const EDGE_LINK_FLAGS: u16 = 0b111;

/// Where the parts of a field start, in bytes, and how many entries each
/// has. The parts lie within the field: [`Layout::of`] checks that.
struct Layout {
    sizes: Sizes,
    vertices: usize,
    vertex_count: usize,
    triangles: usize,
    triangle_count: usize,
    links: usize,
    link_count: usize,
    doors: usize,
    door_count: usize,
}

impl Layout {
    fn of(nvnm: &[u8]) -> Option<Self> {
        let mut bytes = Bytes::new(nvnm);
        let sizes = sizes_of(bytes.u32()?)?;
        // Checksum, worldspace, and grid position or cell.
        bytes.skip(12)?;
        let (vertices, vertex_count) = array(&mut bytes, nvnm, VERTEX_LEN)?;
        let (triangles, triangle_count) = array(&mut bytes, nvnm, sizes.triangle)?;
        let (links, link_count) = array(&mut bytes, nvnm, sizes.link)?;
        let (doors, door_count) = array(&mut bytes, nvnm, DOOR_LEN)?;
        Some(Self {
            sizes,
            vertices,
            vertex_count,
            triangles,
            triangle_count,
            links,
            link_count,
            doors,
            door_count,
        })
    }

    /// [`Layout::of`] for a field in Skyrim's format only.
    fn skyrim(nvnm: &[u8]) -> Option<Self> {
        (read_u32(nvnm, 0)? == SKYRIM_VERSION)
            .then(|| Self::of(nvnm))
            .flatten()
    }
}

/// Skips an array that starts with its length as `u32`, and returns where
/// its entries start in `data` and how many there are.
fn array(bytes: &mut Bytes<'_>, data: &[u8], item_len: usize) -> Option<(usize, usize)> {
    let count = usize::try_from(bytes.u32()?).ok()?;
    let start = data.len().checked_sub(bytes.remaining())?;
    bytes.skip(count.checked_mul(item_len)?)?;
    Some((start, count))
}

fn read_u16(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn write_u16(data: &mut [u8], at: usize, value: u16) -> Option<()> {
    data.get_mut(at..at.checked_add(2)?)?
        .copy_from_slice(&value.to_le_bytes());
    Some(())
}

/// Skips an array that starts with its length as `u32`, and returns the
/// length.
fn skip_array(bytes: &mut Bytes<'_>, item_len: usize) -> Option<usize> {
    let count = usize::try_from(bytes.u32()?).ok()?;
    bytes.skip(count.checked_mul(item_len)?)?;
    Some(count)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A field with `triangles` triangles and the given door triangles.
    fn field(version: u32, triangles: u32, doors: &[(u16, u32)]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(version.to_le_bytes());
        data.extend([0; 12]);
        data.extend(3u32.to_le_bytes());
        data.extend([0; 3 * VERTEX_LEN]);
        data.extend(triangles.to_le_bytes());
        data.extend(vec![0; triangles as usize * TRIANGLE_LEN]);
        data.extend(1u32.to_le_bytes());
        data.extend([0; LINK_LEN]);
        data.extend(u32::try_from(doors.len()).unwrap().to_le_bytes());
        for (triangle, door) in doors {
            data.extend(triangle.to_le_bytes());
            data.extend(0xDEAD_BEEFu32.to_le_bytes());
            data.extend(door.to_le_bytes());
        }
        // Cover triangles and the search grid follow.
        data.extend([0; 24]);
        data
    }

    #[test]
    fn the_doors_of_a_skyrim_navmesh_are_read() {
        let data = field(12, 2, &[(0, 0x0100_0810), (1, 0x0001_2345)]);
        assert_eq!(doors(&data), Some(vec![0x0100_0810, 0x0001_2345]));
    }

    #[test]
    fn a_navmesh_without_doors_has_none() {
        assert_eq!(doors(&field(12, 1, &[])), Some(Vec::new()));
    }

    #[test]
    fn other_versions_and_broken_fields_are_not_read() {
        assert_eq!(doors(&field(14, 2, &[(0, 1)])), None);
        // A door on a triangle the mesh does not have.
        assert_eq!(doors(&field(12, 2, &[(2, 1)])), None);
        let mut cut = field(12, 2, &[(0, 1)]);
        cut.truncate(70);
        assert_eq!(doors(&cut), None);
        assert_eq!(doors(&[]), None);
    }

    /// A triangle as written: its vertices, its edges and its flags.
    type Written = ([u16; 3], [u16; 3], u16);

    /// A field with the given vertices, triangles, links to neighbouring
    /// meshes and door triangles, and some cover after.
    fn mesh(
        vertices: &[[f32; 3]],
        triangles: &[Written],
        links: &[(u32, u16)],
        doors: &[(u16, u32)],
    ) -> Vec<u8> {
        mesh_in(SKYRIM_VERSION, vertices, triangles, links, doors)
    }

    /// [`mesh`] in the layout of `version`: Fallout 4's has a height and
    /// a byte more in each triangle, and a byte more after each link.
    fn mesh_in(
        version: u32,
        vertices: &[[f32; 3]],
        triangles: &[Written],
        links: &[(u32, u16)],
        doors: &[(u16, u32)],
    ) -> Vec<u8> {
        let fallout_4 = version == FALLOUT_4_VERSION;
        let mut data = version.to_le_bytes().to_vec();
        data.extend([0; 12]);
        data.extend(u32::try_from(vertices.len()).unwrap().to_le_bytes());
        for vertex in vertices {
            for value in vertex {
                data.extend(value.to_le_bytes());
            }
        }
        data.extend(u32::try_from(triangles.len()).unwrap().to_le_bytes());
        for (corners, edges, flags) in triangles {
            for value in corners.iter().chain(edges) {
                data.extend(value.to_le_bytes());
            }
            if fallout_4 {
                data.extend(12.5f32.to_le_bytes());
                data.push(0x5A);
            }
            data.extend(flags.to_le_bytes());
            data.extend(0x0101u16.to_le_bytes());
        }
        data.extend(u32::try_from(links.len()).unwrap().to_le_bytes());
        for (navmesh, triangle) in links {
            data.extend(0u32.to_le_bytes());
            data.extend(navmesh.to_le_bytes());
            data.extend(triangle.to_le_bytes());
            if fallout_4 {
                data.push(0xA5);
            }
        }
        data.extend(u32::try_from(doors.len()).unwrap().to_le_bytes());
        for (triangle, door) in doors {
            data.extend(triangle.to_le_bytes());
            data.extend(0xDEAD_BEEFu32.to_le_bytes());
            data.extend(door.to_le_bytes());
        }
        data.extend([7; 24]);
        data
    }

    /// A field with the given vertices and triangles, which touch nothing.
    fn surface(vertices: &[[f32; 3]], triangles: &[[u16; 3]]) -> Vec<u8> {
        let written: Vec<Written> = triangles
            .iter()
            .map(|corners| (*corners, [NONE; 3], 0))
            .collect();
        mesh(vertices, &written, &[], &[])
    }

    #[test]
    fn the_triangles_of_a_skyrim_navmesh_are_read() {
        let vertices = [[0.0, 0.0, 10.0], [100.0, 0.0, 10.0], [0.0, 100.0, 12.5]];
        let data = surface(&vertices, &[[0, 1, 2]]);
        assert_eq!(
            geometry(&data),
            Some(Geometry {
                vertices: vertices.to_vec(),
                triangles: vec![[0, 1, 2]],
                edges: vec![[Beyond::Nothing; 3]],
                doors: Vec::new(),
            })
        );
    }

    #[test]
    fn a_triangle_with_a_missing_vertex_breaks_the_surface() {
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        assert_eq!(geometry(&surface(&vertices, &[[0, 1, 3]])), None);
        assert_eq!(geometry(&field(14, 1, &[])), None);
    }

    /// The neighbouring mesh in [`strip`], as the plugin writes it.
    const NEIGHBOUR: u32 = 0x0100_0ABC;

    /// A strip of four triangles in a row, 0 to 3, each touching the next.
    /// Triangle 3 leads onto triangle 7 of [`NEIGHBOUR`], and onto
    /// triangle 2 of it through a second link; triangle 1 has a door.
    fn strip() -> Vec<u8> {
        strip_in(SKYRIM_VERSION)
    }

    /// [`strip`] in the layout of `version`.
    fn strip_in(version: u32) -> Vec<u8> {
        let vertices = [
            [0.0, 0.0, 0.0],
            [0.0, 100.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [200.0, 0.0, 0.0],
            [200.0, 100.0, 0.0],
        ];
        let triangles = [
            ([0, 1, 2], [NONE, 1, NONE], 0),
            ([1, 3, 2], [NONE, 2, 0], 0),
            ([2, 3, 4], [1, 3, NONE], 0),
            // Edge 1-2 leads to link 0, edge 2-0 to link 1.
            ([3, 5, 4], [NONE, 0, 1], 0b110 | 0x0040),
        ];
        mesh_in(
            version,
            &vertices,
            &triangles,
            &[(NEIGHBOUR, 7), (NEIGHBOUR, 2)],
            &[(1, 0x0000_1234)],
        )
    }

    /// Where the flags of triangle `index` of [`strip`] lie.
    fn flags_at(index: usize) -> usize {
        4 + 12 + 4 + 6 * VERTEX_LEN + 4 + index * TRIANGLE_LEN + FLAGS_AT
    }

    #[test]
    fn how_triangles_hang_together_is_read() {
        let geometry = geometry(&strip()).unwrap();
        assert_eq!(
            geometry.edges,
            vec![
                [Beyond::Nothing, Beyond::Triangle(1), Beyond::Nothing],
                [Beyond::Nothing, Beyond::Triangle(2), Beyond::Triangle(0)],
                [Beyond::Triangle(1), Beyond::Triangle(3), Beyond::Nothing],
                [
                    Beyond::Nothing,
                    Beyond::Link {
                        navmesh: NEIGHBOUR,
                        triangle: 7
                    },
                    Beyond::Link {
                        navmesh: NEIGHBOUR,
                        triangle: 2
                    },
                ],
            ]
        );
        assert_eq!(geometry.doors, vec![(1, 0x0000_1234)]);
    }

    #[test]
    fn a_fallout_4_navmesh_reads_like_the_same_skyrim_one() {
        let fallout_4 = strip_in(FALLOUT_4_VERSION);
        assert_eq!(fallout_4.len(), strip().len() + 4 * 5 + 2);
        assert_eq!(geometry(&fallout_4), geometry(&strip()));
        assert_eq!(doors(&fallout_4), Some(vec![0x0000_1234]));
        assert_eq!(door_triangles(&fallout_4), Some(vec![(1, 0x0000_1234)]));
    }

    #[test]
    fn a_fallout_4_navmesh_is_not_cut() {
        let fallout_4 = strip_in(FALLOUT_4_VERSION);
        assert_eq!(cut(&fallout_4, &BTreeSet::from([0])), None);
        assert_eq!(unlink(&fallout_4, NEIGHBOUR, &BTreeSet::from([7])), None);
    }

    #[test]
    fn edges_to_triangles_or_links_the_mesh_lacks_lead_nowhere() {
        let vertices = [[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let data = mesh(&vertices, &[([0, 1, 2], [5, 3, NONE], 0b010)], &[], &[]);
        assert_eq!(geometry(&data).unwrap().edges, vec![[Beyond::Nothing; 3]]);
    }

    #[test]
    fn cut_triangles_lead_nowhere_and_nothing_leads_onto_them() {
        let data = strip();
        let cut = cut(&data, &BTreeSet::from([2, 3])).unwrap();
        assert_eq!(cut.len(), data.len());
        let after = geometry(&cut).unwrap();
        assert_eq!(
            after.edges,
            vec![
                [Beyond::Nothing, Beyond::Triangle(1), Beyond::Nothing],
                [Beyond::Nothing, Beyond::Nothing, Beyond::Triangle(0)],
                [Beyond::Nothing; 3],
                [Beyond::Nothing; 3],
            ]
        );
        // The vertices, the triangles' corners and the doors stay.
        let before = geometry(&data).unwrap();
        assert_eq!(after.vertices, before.vertices);
        assert_eq!(after.triangles, before.triangles);
        assert_eq!(after.doors, before.doors);
        // Of triangle 3 the link flags are gone, its other flags stay.
        assert_eq!(read_u16(&cut, flags_at(3)), Some(0x0040));
        // Nothing after the triangles changed.
        let after_triangles = flags_at(3) + 4;
        assert_eq!(cut.get(after_triangles..), data.get(after_triangles..));
    }

    #[test]
    fn cutting_nothing_changes_nothing() {
        let data = strip();
        assert_eq!(cut(&data, &BTreeSet::new()), Some(data.clone()));
        assert_eq!(unlink(&data, NEIGHBOUR, &BTreeSet::new()), Some(data));
    }

    #[test]
    fn links_onto_cut_triangles_of_a_neighbour_lead_nowhere() {
        let data = strip();
        let unlinked = unlink(&data, NEIGHBOUR, &BTreeSet::from([7])).unwrap();
        let edges = geometry(&unlinked).unwrap().edges;
        assert_eq!(
            edges.get(3),
            Some(&[
                Beyond::Nothing,
                Beyond::Nothing,
                Beyond::Link {
                    navmesh: NEIGHBOUR,
                    triangle: 2
                },
            ])
        );
        assert_eq!(read_u16(&unlinked, flags_at(3)), Some(0b100 | 0x0040));
        // Another mesh's triangle 7 is another triangle.
        assert_eq!(
            unlink(&data, NEIGHBOUR + 1, &BTreeSet::from([7])),
            Some(data)
        );
    }

    #[test]
    fn broken_fields_are_not_cut() {
        assert_eq!(cut(&field(15, 1, &[]), &BTreeSet::from([0])), None);
        let mut short = strip();
        short.truncate(60);
        assert_eq!(cut(&short, &BTreeSet::from([0])), None);
        assert_eq!(unlink(&short, NEIGHBOUR, &BTreeSet::from([0])), None);
    }

    proptest! {
        #[test]
        fn never_panics(data in prop::collection::vec(any::<u8>(), 0..256)) {
            let _ = doors(&data);
            let _ = geometry(&data);
            let _ = cut(&data, &BTreeSet::from([0, 1, 2]));
            let _ = unlink(&data, 0, &BTreeSet::from([0, 1, 2]));
        }

        #[test]
        fn never_panics_on_a_valid_start(rest in prop::collection::vec(any::<u8>(), 0..256)) {
            let mut data = SKYRIM_VERSION.to_le_bytes().to_vec();
            data.extend(rest);
            let _ = doors(&data);
            let _ = geometry(&data);
            let _ = cut(&data, &BTreeSet::from([0, 1, 2]));
            let _ = unlink(&data, 0, &BTreeSet::from([0, 1, 2]));
        }

        #[test]
        fn never_panics_on_a_fallout_4_start(rest in prop::collection::vec(any::<u8>(), 0..256)) {
            let mut data = FALLOUT_4_VERSION.to_le_bytes().to_vec();
            data.extend(rest);
            let _ = doors(&data);
            let _ = geometry(&data);
            let _ = cut(&data, &BTreeSet::from([0, 1, 2]));
        }

        #[test]
        fn never_panics_on_a_damaged_fallout_4_mesh(at in 0usize..200, byte in any::<u8>(), keep in 0usize..200) {
            let mut data = strip_in(FALLOUT_4_VERSION);
            if let Some(slot) = data.get_mut(at) {
                *slot = byte;
            }
            data.truncate(keep.max(4));
            let _ = doors(&data);
            let _ = geometry(&data);
        }

        #[test]
        fn cutting_keeps_the_size_and_cuts_every_way_onto_the_cut(
            cut_set in prop::collection::btree_set(0u16..6, 0..6),
        ) {
            let data = strip();
            let cut = cut(&data, &cut_set).unwrap();
            prop_assert_eq!(cut.len(), data.len());
            let after = geometry(&cut).unwrap();
            for (index, edges) in after.edges.iter().enumerate() {
                let number = u16::try_from(index).unwrap();
                for edge in edges {
                    if cut_set.contains(&number) {
                        prop_assert_eq!(*edge, Beyond::Nothing);
                    }
                    if let Beyond::Triangle(other) = edge {
                        prop_assert!(!cut_set.contains(other));
                    }
                }
            }
        }
    }

    /// A navigation mesh record of Fallout 3's and New Vegas's layout with
    /// the given fields, and `DATA` counting them unless `counts` is false.
    fn split_record(
        vertices: &[[f32; 3]],
        triangles: &[Written],
        links: &[(u32, u16)],
        doors: &[(u16, u32)],
        counts: bool,
    ) -> Record {
        let field = |signature: &[u8; 4], data: Vec<u8>| crate::plugin::Subrecord {
            signature: *signature,
            data,
        };
        let count = |n: usize| u32::try_from(n).unwrap().to_le_bytes();
        let mut data = 0x0000_1A2Bu32.to_le_bytes().to_vec();
        data.extend(count(vertices.len()));
        data.extend(count(triangles.len()));
        data.extend(count(links.len()));
        data.extend(count(0));
        data.extend(count(doors.len()));
        let vertex_bytes: Vec<u8> = vertices
            .iter()
            .flat_map(|vertex| vertex.iter().flat_map(|value| value.to_le_bytes()))
            .collect();
        let mut triangle_bytes = Vec::new();
        for (corners, edges, flags) in triangles {
            for value in corners.iter().chain(edges) {
                triangle_bytes.extend(value.to_le_bytes());
            }
            triangle_bytes.extend(flags.to_le_bytes());
            triangle_bytes.extend(0x0101u16.to_le_bytes());
        }
        let mut door_bytes = Vec::new();
        for (triangle, door) in doors {
            door_bytes.extend(door.to_le_bytes());
            door_bytes.extend(triangle.to_le_bytes());
            door_bytes.extend([0; 2]);
        }
        let mut link_bytes = Vec::new();
        for (navmesh, triangle) in links {
            link_bytes.extend(0u32.to_le_bytes());
            link_bytes.extend(navmesh.to_le_bytes());
            link_bytes.extend(triangle.to_le_bytes());
        }
        let mut subrecords = vec![
            field(b"EDID", b"Mesh\0".to_vec()),
            field(b"NVER", 11u32.to_le_bytes().to_vec()),
        ];
        if counts {
            subrecords.push(field(b"DATA", data));
        }
        subrecords.extend([
            field(b"NVVX", vertex_bytes),
            field(b"NVTR", triangle_bytes),
            field(b"NVCA", Vec::new()),
            field(b"NVDP", door_bytes),
            field(b"NVGD", vec![7; 24]),
            field(b"NVEX", link_bytes),
        ]);
        Record {
            signature: *b"NAVM",
            form_id: 0x0001_0000,
            flags: 0,
            subrecords,
            groups: Vec::new(),
        }
    }

    /// [`strip`] in Fallout 3's and New Vegas's layout.
    fn split_strip(counts: bool) -> Record {
        let vertices = [
            [0.0, 0.0, 0.0],
            [0.0, 100.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [200.0, 0.0, 0.0],
            [200.0, 100.0, 0.0],
        ];
        let triangles = [
            ([0, 1, 2], [NONE, 1, NONE], 0),
            ([1, 3, 2], [NONE, 2, 0], 0),
            ([2, 3, 4], [1, 3, NONE], 0),
            ([3, 5, 4], [NONE, 0, 1], 0b110 | 0x0040),
        ];
        split_record(
            &vertices,
            &triangles,
            &[(NEIGHBOUR, 7), (NEIGHBOUR, 2)],
            &[(1, 0x0000_1234)],
            counts,
        )
    }

    #[test]
    fn the_older_layout_reads_like_skyrims() {
        let skyrim = geometry(&strip()).unwrap();
        assert_eq!(split_geometry(&split_strip(true)), Some(skyrim.clone()));
        // Without counts, the fields' lengths say how many entries there are.
        assert_eq!(split_geometry(&split_strip(false)), Some(skyrim.clone()));
        assert_eq!(record_geometry(&split_strip(true)), Some(skyrim));
        assert_eq!(
            split_geometry(&split_strip(true)).unwrap().doors,
            [(1, 0x0000_1234)]
        );
    }

    #[test]
    fn a_record_with_one_field_is_read_by_its_version() {
        let record = Record {
            signature: *b"NAVM",
            form_id: 1,
            flags: 0,
            subrecords: vec![crate::plugin::Subrecord {
                signature: *b"NVNM",
                data: strip_in(FALLOUT_4_VERSION),
            }],
            groups: Vec::new(),
        };
        assert_eq!(record_geometry(&record), geometry(&strip()));
    }

    #[test]
    fn an_older_mesh_that_does_not_hold_together_is_not_read() {
        // Counts that disagree with the fields.
        let mut record = split_strip(true);
        if let Some(data) = record
            .subrecords
            .iter_mut()
            .find(|field| &field.signature == b"DATA")
        {
            data.data[8] = 9;
        }
        assert_eq!(split_geometry(&record), None);
        // A triangles field that ends inside a triangle.
        let mut record = split_strip(false);
        if let Some(triangles) = record
            .subrecords
            .iter_mut()
            .find(|field| &field.signature == b"NVTR")
        {
            triangles.data.pop();
        }
        assert_eq!(split_geometry(&record), None);
        // A triangle on a vertex the mesh does not have.
        let vertices = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let broken = split_record(&vertices, &[([0, 1, 3], [NONE; 3], 0)], &[], &[], true);
        assert_eq!(split_geometry(&broken), None);
        // No triangles at all: not a mesh of this layout.
        let mut record = split_strip(false);
        record
            .subrecords
            .retain(|field| &field.signature != b"NVTR");
        assert_eq!(split_geometry(&record), None);
    }

    proptest! {
        #[test]
        fn the_older_layout_never_panics(
            fields in prop::collection::vec(
                (prop::sample::select(vec![*b"DATA", *b"NVVX", *b"NVTR", *b"NVEX", *b"NVDP"]),
                 prop::collection::vec(any::<u8>(), 0..96)),
                0..8),
        ) {
            let record = Record {
                signature: *b"NAVM",
                form_id: 1,
                flags: 0,
                subrecords: fields
                    .into_iter()
                    .map(|(signature, data)| crate::plugin::Subrecord { signature, data })
                    .collect(),
                groups: Vec::new(),
            };
            let _ = split_geometry(&record);
            let _ = record_geometry(&record);
        }

        #[test]
        fn a_damaged_older_mesh_never_panics(
            which in 0usize..9, at in 0usize..120, byte in any::<u8>(), keep in 0usize..120,
        ) {
            let mut record = split_strip(true);
            if let Some(field) = record.subrecords.get_mut(which) {
                if let Some(slot) = field.data.get_mut(at) {
                    *slot = byte;
                }
                field.data.truncate(keep);
            }
            let _ = split_geometry(&record);
        }
    }
}
