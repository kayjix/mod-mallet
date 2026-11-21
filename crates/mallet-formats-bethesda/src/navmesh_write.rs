//! Reads every part of a Skyrim navigation mesh's field and writes it
//! again (ADR 0058), so that a changed mesh can be written into a plugin.
//!
//! [`navmesh`](crate::navmesh) reads what the analyses need; this module
//! reads everything the field carries, in Skyrim's layout (version 12),
//! after xEdit's definition. All numbers are little endian:
//!
//! ```text
//! u32 version                      12
//! u32 checksum                     kept as it is
//! u32 worldspace                   0 inside
//! u32 grid position or cell        outside: i16 row, i16 column;
//!                                  inside: the cell
//! u32 n, n × 12 bytes              vertices: three f32 each
//! u32 n, n × 16 bytes              triangles: three u16 vertices, three
//!                                  u16 edges, u16 flags, u16 cover flags
//! u32 n, n × 10 bytes              edge links: u32 kind, u32 mesh,
//!                                  u16 triangle
//! u32 n, n × 10 bytes              door triangles: u16 triangle,
//!                                  u32 door kind, u32 door reference
//! u32 n, n × 2 bytes               cover triangles: u16 triangle
//! u32 size                         the search grid: size × size squares
//! f32 × 2                          the side of a square, east and north
//! f32 × 6                          the box around the vertices: least
//!                                  x, y, z, then greatest x, y, z
//! size × size × (u32 n, n × u16)   the triangles in each square, row by
//!                                  row from the south, each row from the
//!                                  west
//! ```
//!
//! Skyrim keeps no waypoints in the field; Fallout 4's layout (version 15)
//! adds them and is not read here. Whatever follows the grid is kept as
//! it is. Floats are kept bit for bit, so reading a field and writing it
//! again gives the same bytes.

/// The format version of Skyrim's navigation meshes.
pub const SKYRIM_VERSION: u32 = 12;

/// An edge that leads nowhere.
pub const NONE: u16 = 0xFFFF;

/// Flags of a triangle, as xEdit names them.
pub mod flags {
    /// The first, second or third edge leads onto a neighbouring mesh:
    /// its number is that of an edge link.
    pub const EDGE_LINKS: u16 = 0b111;
    /// Characters must not stand here: xEdit's "Deleted".
    pub const DELETED: u16 = 0x0008;
    /// Large creatures cannot pass.
    pub const NO_LARGE_CREATURES: u16 = 0x0010;
    /// Lies over another triangle.
    pub const OVERLAPPING: u16 = 0x0020;
    /// Preferred by characters finding their way.
    pub const PREFERRED: u16 = 0x0040;
    /// Lies in water.
    pub const WATER: u16 = 0x0200;
    /// Lies in a door.
    pub const DOOR: u16 = 0x0400;
    /// Found by the Creation Kit's search.
    pub const FOUND: u16 = 0x0800;
}

/// Every part of a Skyrim navigation mesh's field.
#[derive(Debug, Clone, PartialEq)]
pub struct NavmeshData {
    /// The format version: [`SKYRIM_VERSION`].
    pub version: u32,
    /// A number the Creation Kit writes; kept as it is.
    pub checksum: u32,
    /// The worldspace, by its form ID as the plugin writes it; 0 inside.
    pub worldspace: u32,
    /// Outside, the cell's grid position (row in the low, column in the
    /// high half, both signed); inside, the cell by its form ID.
    pub parent: u32,
    /// The vertices, in the coordinates of the worldspace or the cell.
    pub vertices: Vec<[f32; 3]>,
    /// The triangles.
    pub triangles: Vec<Triangle>,
    /// The links to triangles of neighbouring meshes that edges name.
    pub edge_links: Vec<EdgeLink>,
    /// The triangles where characters step through a door.
    pub doors: Vec<DoorTriangle>,
    /// The triangles with cover beside them.
    pub cover_triangles: Vec<u16>,
    /// The search grid: which triangles lie in which square.
    pub grid: Grid,
    /// Whatever follows the grid, kept as it is.
    pub rest: Vec<u8>,
}

/// One triangle of a navigation mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Triangle {
    /// Its corners, by vertex number.
    pub vertices: [u16; 3],
    /// What lies beyond its edges from the first corner to the second,
    /// the second to the third, the third to the first: a triangle's
    /// number, an edge link's number where [`flags::EDGE_LINKS`] says so,
    /// or [`NONE`].
    pub edges: [u16; 3],
    /// Its flags ([`flags`]).
    pub flags: u16,
    /// How high the cover along its edges is and on which side.
    pub cover: u16,
}

impl Triangle {
    /// Whether edge `edge` (0 to 2) names an edge link.
    pub fn is_link(&self, edge: usize) -> bool {
        edge < 3 && self.flags & (1 << edge) != 0
    }
}

/// A link from an edge to a triangle of a neighbouring mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeLink {
    /// What kind of step it is; kept as it is.
    pub kind: u32,
    /// The neighbouring mesh, by its form ID as the plugin writes it.
    pub navmesh: u32,
    /// The triangle there, by its number.
    pub triangle: u16,
}

/// A triangle where characters step through a door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorTriangle {
    /// The triangle, by its number.
    pub triangle: u16,
    /// What kind of door it is; kept as it is.
    pub kind: u32,
    /// The placed door, by its form ID as the plugin writes it.
    pub door: u32,
}

/// The search grid: the box around the mesh cut into squares, with the
/// triangles that lie in each.
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    /// How many squares a side.
    pub size: u32,
    /// The side of a square, east and north.
    pub square: [f32; 2],
    /// The least corner of the box around the vertices.
    pub min: [f32; 3],
    /// The greatest corner.
    pub max: [f32; 3],
    /// The triangles in each square, by number, row by row from the
    /// south, each row from the west.
    pub cells: Vec<Vec<u16>>,
}

impl NavmeshData {
    /// Reads every part of `nvnm`. `None` if the field is not in Skyrim's
    /// layout or does not hold together.
    pub fn read(nvnm: &[u8]) -> Option<Self> {
        let mut cursor = Cursor { data: nvnm, at: 0 };
        let version = cursor.u32()?;
        if version != SKYRIM_VERSION {
            return None;
        }
        let checksum = cursor.u32()?;
        let worldspace = cursor.u32()?;
        let parent = cursor.u32()?;
        let vertices = cursor.array(12, |cursor| {
            Some([cursor.f32()?, cursor.f32()?, cursor.f32()?])
        })?;
        let triangles = cursor.array(16, |cursor| {
            Some(Triangle {
                vertices: [cursor.u16()?, cursor.u16()?, cursor.u16()?],
                edges: [cursor.u16()?, cursor.u16()?, cursor.u16()?],
                flags: cursor.u16()?,
                cover: cursor.u16()?,
            })
        })?;
        let edge_links = cursor.array(10, |cursor| {
            Some(EdgeLink {
                kind: cursor.u32()?,
                navmesh: cursor.u32()?,
                triangle: cursor.u16()?,
            })
        })?;
        let doors = cursor.array(10, |cursor| {
            Some(DoorTriangle {
                triangle: cursor.u16()?,
                kind: cursor.u32()?,
                door: cursor.u32()?,
            })
        })?;
        let cover_triangles = cursor.array(2, Cursor::u16)?;
        let size = cursor.u32()?;
        let square = [cursor.f32()?, cursor.f32()?];
        let min = [cursor.f32()?, cursor.f32()?, cursor.f32()?];
        let max = [cursor.f32()?, cursor.f32()?, cursor.f32()?];
        let squares = usize::try_from(size).ok()?.checked_pow(2)?;
        // Each square needs at least its count.
        if squares > cursor.remaining() / 4 {
            return None;
        }
        let mut cells = Vec::with_capacity(squares);
        for _ in 0..squares {
            cells.push(cursor.array(2, Cursor::u16)?);
        }
        let rest = cursor.data.get(cursor.at..)?.to_vec();
        Some(Self {
            version,
            checksum,
            worldspace,
            parent,
            vertices,
            triangles,
            edge_links,
            doors,
            cover_triangles,
            grid: Grid {
                size,
                square,
                min,
                max,
                cells,
            },
            rest,
        })
    }

    /// The field as bytes. `None` if a list is too long for its count.
    pub fn write(&self) -> Option<Vec<u8>> {
        let mut out = Vec::with_capacity(self.len_hint());
        out.extend(self.version.to_le_bytes());
        out.extend(self.checksum.to_le_bytes());
        out.extend(self.worldspace.to_le_bytes());
        out.extend(self.parent.to_le_bytes());
        count(&mut out, self.vertices.len())?;
        for vertex in &self.vertices {
            for value in vertex {
                out.extend(value.to_bits().to_le_bytes());
            }
        }
        count(&mut out, self.triangles.len())?;
        for triangle in &self.triangles {
            for value in triangle.vertices.iter().chain(&triangle.edges) {
                out.extend(value.to_le_bytes());
            }
            out.extend(triangle.flags.to_le_bytes());
            out.extend(triangle.cover.to_le_bytes());
        }
        count(&mut out, self.edge_links.len())?;
        for link in &self.edge_links {
            out.extend(link.kind.to_le_bytes());
            out.extend(link.navmesh.to_le_bytes());
            out.extend(link.triangle.to_le_bytes());
        }
        count(&mut out, self.doors.len())?;
        for door in &self.doors {
            out.extend(door.triangle.to_le_bytes());
            out.extend(door.kind.to_le_bytes());
            out.extend(door.door.to_le_bytes());
        }
        count(&mut out, self.cover_triangles.len())?;
        for triangle in &self.cover_triangles {
            out.extend(triangle.to_le_bytes());
        }
        let grid = &self.grid;
        out.extend(grid.size.to_le_bytes());
        for value in grid.square.iter().chain(&grid.min).chain(&grid.max) {
            out.extend(value.to_bits().to_le_bytes());
        }
        let squares = usize::try_from(grid.size).ok()?.checked_pow(2)?;
        if grid.cells.len() != squares {
            return None;
        }
        for cell in &grid.cells {
            count(&mut out, cell.len())?;
            for triangle in cell {
                out.extend(triangle.to_le_bytes());
            }
        }
        out.extend(&self.rest);
        Some(out)
    }

    /// Every form ID the field names, to be rewritten for another
    /// plugin's masters: the worldspace, the cell inside, the meshes the
    /// edge links lead to and the doors.
    pub fn form_ids_mut(&mut self) -> impl Iterator<Item = &mut u32> {
        let inside = self.worldspace == 0;
        std::iter::once(&mut self.worldspace)
            .filter(|world| **world != 0)
            .chain(std::iter::once(&mut self.parent).filter(move |_| inside))
            .chain(self.edge_links.iter_mut().map(|link| &mut link.navmesh))
            .chain(self.doors.iter_mut().map(|door| &mut door.door))
    }

    /// The square of the grid a point lies in, as its number in
    /// [`Grid::cells`], clamped to the grid.
    pub fn square_of(&self, x: f32, y: f32) -> Option<usize> {
        let grid = &self.grid;
        let size = usize::try_from(grid.size).ok()?;
        if size == 0 {
            return None;
        }
        // At most a few dozen squares a side.
        #[allow(clippy::cast_precision_loss)]
        let last = (size - 1) as f32;
        let index = |value: f32, min: f32, side: f32| -> usize {
            if side.is_nan() || side <= 0.0 || !value.is_finite() {
                return 0;
            }
            let steps = ((value - min) / side).floor();
            // Clamped to the grid before converting.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let whole = steps.clamp(0.0, last) as usize;
            whole
        };
        let column = index(x, grid.min[0], grid.square[0]);
        let row = index(y, grid.min[1], grid.square[1]);
        row.checked_mul(size)?.checked_add(column)
    }

    /// Puts triangle `triangle` into every square of the grid that the
    /// box around its corners touches, unless it is there already.
    pub fn add_to_grid(&mut self, triangle: u16) -> Option<()> {
        let corners = self.triangles.get(usize::from(triangle))?.vertices;
        let mut low = [f32::MAX; 2];
        let mut high = [f32::MIN; 2];
        for corner in corners {
            let vertex = self.vertices.get(usize::from(corner))?;
            for axis in 0..2 {
                let value = *vertex.get(axis)?;
                let (lo, hi) = (low.get_mut(axis)?, high.get_mut(axis)?);
                *lo = lo.min(value);
                *hi = hi.max(value);
            }
        }
        let size = usize::try_from(self.grid.size).ok()?;
        let first = self.square_of(low[0], low[1])?;
        let last = self.square_of(high[0], high[1])?;
        let (first_row, first_column) = (first / size.max(1), first % size.max(1));
        let (last_row, last_column) = (last / size.max(1), last % size.max(1));
        for row in first_row..=last_row {
            for column in first_column..=last_column {
                let cell = self.grid.cells.get_mut(row * size + column)?;
                if !cell.contains(&triangle) {
                    cell.push(triangle);
                }
            }
        }
        Some(())
    }

    /// Widens the grid's box in height to hold every vertex. The squares
    /// only depend on east and north, which stay.
    pub fn fit_grid_height(&mut self) {
        for vertex in &self.vertices {
            if vertex[2].is_finite() {
                self.grid.min[2] = self.grid.min[2].min(vertex[2]);
                self.grid.max[2] = self.grid.max[2].max(vertex[2]);
            }
        }
    }

    fn len_hint(&self) -> usize {
        64 + self.vertices.len() * 12
            + self.triangles.len() * 16
            + (self.edge_links.len() + self.doors.len()) * 10
            + self.cover_triangles.len() * 2
            + self
                .grid
                .cells
                .iter()
                .map(|cell| 4 + cell.len() * 2)
                .sum::<usize>()
            + self.rest.len()
    }
}

/// Writes a list's count, `None` if it does not fit.
fn count(out: &mut Vec<u8>, len: usize) -> Option<()> {
    out.extend(u32::try_from(len).ok()?.to_le_bytes());
    Some(())
}

/// A cursor that reads floats bit for bit.
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let end = self.at.checked_add(N)?;
        let bytes = self.data.get(self.at..end)?.try_into().ok()?;
        self.at = end;
        Some(bytes)
    }

    fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_le_bytes)
    }

    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.at)
    }

    /// A list that starts with its count; `item_len` bytes per entry
    /// bound the count by what is left.
    fn array<T>(
        &mut self,
        item_len: usize,
        mut item: impl FnMut(&mut Self) -> Option<T>,
    ) -> Option<Vec<T>> {
        let count = usize::try_from(self.u32()?).ok()?;
        if count > self.remaining() / item_len.max(1) {
            return None;
        }
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(item(self)?);
        }
        Some(items)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A small mesh outside: two triangles, a link, a door, cover and a
    /// grid of two by two squares.
    fn sample() -> NavmeshData {
        NavmeshData {
            version: SKYRIM_VERSION,
            checksum: 0x1234_5678,
            worldspace: 0x0000_003C,
            parent: u32::from_le_bytes([0x1A, 0, 0x06, 0]),
            vertices: vec![
                [0.0, 0.0, 10.0],
                [100.0, 0.0, 10.5],
                [0.0, 100.0, -3.25],
                [100.0, 100.0, 0.0],
            ],
            triangles: vec![
                Triangle {
                    vertices: [0, 1, 2],
                    edges: [NONE, 1, 0],
                    flags: 0b100 | flags::PREFERRED,
                    cover: 0x0013,
                },
                Triangle {
                    vertices: [1, 3, 2],
                    edges: [NONE, NONE, 0],
                    flags: flags::WATER,
                    cover: 0,
                },
            ],
            edge_links: vec![EdgeLink {
                kind: 0,
                navmesh: 0x0010_2F50,
                triangle: 17,
            }],
            doors: vec![DoorTriangle {
                triangle: 1,
                kind: 0xA4C8_0D1B,
                door: 0x0100_0814,
            }],
            cover_triangles: vec![0],
            grid: Grid {
                size: 2,
                square: [50.0, 50.0],
                min: [0.0, 0.0, -3.25],
                max: [100.0, 100.0, 10.5],
                cells: vec![vec![0], vec![0, 1], vec![0, 1], vec![1]],
            },
            rest: Vec::new(),
        }
    }

    #[test]
    fn a_mesh_written_and_read_again_is_the_same() {
        let mesh = sample();
        let bytes = mesh.write().unwrap();
        assert_eq!(NavmeshData::read(&bytes), Some(mesh));
    }

    #[test]
    fn reading_and_writing_gives_the_same_bytes() {
        let mut mesh = sample();
        mesh.rest = vec![1, 2, 3];
        let bytes = mesh.write().unwrap();
        assert_eq!(NavmeshData::read(&bytes).unwrap().write(), Some(bytes));
    }

    #[test]
    fn floats_are_kept_bit_for_bit() {
        let mut mesh = sample();
        mesh.vertices[0] = [f32::from_bits(0x7FC0_0001), -0.0, f32::INFINITY];
        let bytes = mesh.write().unwrap();
        let again = NavmeshData::read(&bytes).unwrap().write().unwrap();
        assert_eq!(again, bytes);
    }

    #[test]
    fn the_analysis_reader_reads_what_is_written() {
        let bytes = sample().write().unwrap();
        let geometry = crate::navmesh::geometry(&bytes).unwrap();
        assert_eq!(geometry.triangles, vec![[0, 1, 2], [1, 3, 2]]);
        assert_eq!(geometry.doors, vec![(1, 0x0100_0814)]);
        assert_eq!(
            geometry.edges.first().map(|edges| edges[2]),
            Some(crate::navmesh::Beyond::Link {
                navmesh: 0x0010_2F50,
                triangle: 17
            })
        );
    }

    #[test]
    fn other_versions_and_broken_fields_are_not_read() {
        let mut mesh = sample();
        mesh.version = 15;
        assert_eq!(NavmeshData::read(&mesh.write().unwrap()), None);
        let bytes = sample().write().unwrap();
        for len in 0..bytes.len() {
            assert_eq!(NavmeshData::read(bytes.get(..len).unwrap()), None, "{len}");
        }
    }

    #[test]
    fn a_grid_of_the_wrong_size_is_not_written() {
        let mut mesh = sample();
        mesh.grid.cells.pop();
        assert_eq!(mesh.write(), None);
    }

    #[test]
    fn form_ids_are_the_world_links_and_doors_outside_and_the_cell_inside() {
        let mut mesh = sample();
        let outside: Vec<u32> = mesh.form_ids_mut().map(|id| *id).collect();
        assert_eq!(outside, vec![0x3C, 0x0010_2F50, 0x0100_0814]);
        mesh.worldspace = 0;
        mesh.parent = 0x0000_0D00;
        for id in mesh.form_ids_mut() {
            *id |= 0x0100_0000;
        }
        assert_eq!(mesh.worldspace, 0);
        assert_eq!(mesh.parent, 0x0100_0D00);
        assert_eq!(mesh.doors[0].door, 0x0100_0814);
    }

    #[test]
    fn a_new_triangle_goes_into_the_squares_it_touches() {
        let mut mesh = sample();
        for cell in &mut mesh.grid.cells {
            cell.clear();
        }
        mesh.vertices.push([60.0, 10.0, 0.0]);
        mesh.vertices.push([90.0, 10.0, 0.0]);
        mesh.vertices.push([90.0, 40.0, 0.0]);
        mesh.triangles.push(Triangle {
            vertices: [4, 5, 6],
            edges: [NONE; 3],
            flags: 0,
            cover: 0,
        });
        mesh.add_to_grid(2).unwrap();
        mesh.add_to_grid(2).unwrap();
        assert_eq!(mesh.grid.cells, vec![vec![], vec![2], vec![], vec![]]);
        assert_eq!(mesh.square_of(-500.0, 1e9), Some(2));
    }

    #[test]
    fn the_box_grows_to_hold_lower_vertices() {
        let mut mesh = sample();
        mesh.vertices.push([10.0, 10.0, -200.0]);
        mesh.fit_grid_height();
        assert_eq!(mesh.grid.min, [0.0, 0.0, -200.0]);
        assert_eq!(mesh.grid.max, [100.0, 100.0, 10.5]);
    }

    proptest! {
        #[test]
        fn never_panics(data in prop::collection::vec(any::<u8>(), 0..512)) {
            if let Some(mut mesh) = NavmeshData::read(&data) {
                let _ = mesh.write();
                let _ = mesh.square_of(0.0, 0.0);
                let _ = mesh.add_to_grid(0);
                mesh.fit_grid_height();
                for id in mesh.form_ids_mut() {
                    *id = 0;
                }
            }
        }

        #[test]
        fn never_panics_on_a_valid_start(rest in prop::collection::vec(any::<u8>(), 0..512)) {
            let mut data = SKYRIM_VERSION.to_le_bytes().to_vec();
            data.extend(rest);
            if let Some(mut mesh) = NavmeshData::read(&data) {
                prop_assert_eq!(mesh.write(), Some(data));
                let _ = mesh.add_to_grid(0);
            }
        }

        #[test]
        fn any_mesh_read_writes_its_own_bytes(
            vertices in prop::collection::vec(any::<[u32; 3]>(), 0..8),
            triangles in prop::collection::vec(any::<([u16; 6], u16, u16)>(), 0..8),
            links in prop::collection::vec(any::<(u32, u32, u16)>(), 0..4),
            doors in prop::collection::vec(any::<(u16, u32, u32)>(), 0..4),
            cover in prop::collection::vec(any::<u16>(), 0..4),
            size in 0u32..4,
            cells in prop::collection::vec(prop::collection::vec(any::<u16>(), 0..4), 16),
            rest in prop::collection::vec(any::<u8>(), 0..8),
        ) {
            let squares = (size * size) as usize;
            let mesh = NavmeshData {
                version: SKYRIM_VERSION,
                checksum: 7,
                worldspace: 0,
                parent: 0x0D00,
                vertices: vertices.iter().map(|v| v.map(f32::from_bits)).collect(),
                triangles: triangles
                    .iter()
                    .map(|(corners, flags, cover)| Triangle {
                        vertices: [corners[0], corners[1], corners[2]],
                        edges: [corners[3], corners[4], corners[5]],
                        flags: *flags,
                        cover: *cover,
                    })
                    .collect(),
                edge_links: links
                    .iter()
                    .map(|&(kind, navmesh, triangle)| EdgeLink { kind, navmesh, triangle })
                    .collect(),
                doors: doors
                    .iter()
                    .map(|&(triangle, kind, door)| DoorTriangle { triangle, kind, door })
                    .collect(),
                cover_triangles: cover,
                grid: Grid {
                    size,
                    square: [64.0, 64.0],
                    min: [0.0; 3],
                    max: [1.0; 3],
                    cells: cells.into_iter().take(squares).collect(),
                },
                rest,
            };
            let bytes = mesh.write().unwrap();
            let read = NavmeshData::read(&bytes).unwrap();
            prop_assert_eq!(read.write(), Some(bytes));
        }
    }
}
