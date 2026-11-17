//! Reads which doors a navigation mesh connects to.
//!
//! A navigation mesh (`NAVM`) keeps its geometry in one field, `NVNM`. In
//! Skyrim (format version 12) it is laid out as follows, all numbers little
//! endian:
//!
//! ```text
//! u32 version                      12
//! u32 checksum, u32 worldspace     where the mesh lies
//! 4 bytes                          grid position or cell
//! u32 n, n × 12 bytes              vertices: three floats each
//! u32 n, n × 16 bytes              triangles
//! u32 n, n × 10 bytes              links to neighbouring meshes
//! u32 n, n × 10 bytes              door triangles: u16 triangle,
//!                                  u32 checksum, u32 door reference
//! ...                              cover, then a search grid
//! ```
//!
//! A door triangle is where characters step through a door. A door that
//! has none cannot be used by anyone but the player.
//!
//! Other games lay the field out differently; for them nothing is read.

use crate::Bytes;

/// The format version of Skyrim's navigation meshes.
const SKYRIM_VERSION: u32 = 12;

/// Bytes per vertex, triangle, link and door triangle.
const VERTEX_LEN: usize = 12;
const TRIANGLE_LEN: usize = 16;
const LINK_LEN: usize = 10;

/// The doors a navigation mesh connects to, by their form IDs as the
/// plugin writes them (relative to its masters), in file order. `None`
/// if the field is not in Skyrim's format or does not hold together.
pub fn doors(nvnm: &[u8]) -> Option<Vec<u32>> {
    let mut bytes = Bytes::new(nvnm);
    if bytes.u32()? != SKYRIM_VERSION {
        return None;
    }
    // Checksum, worldspace, and grid position or cell.
    bytes.skip(12)?;
    skip_array(&mut bytes, VERTEX_LEN)?;
    let triangles = skip_array(&mut bytes, TRIANGLE_LEN)?;
    skip_array(&mut bytes, LINK_LEN)?;

    let count = usize::try_from(bytes.u32()?).ok()?;
    // Each entry needs ten bytes; a count that cannot fit is damage.
    if count > bytes.remaining() / LINK_LEN {
        return None;
    }
    let mut doors = Vec::with_capacity(count);
    for _ in 0..count {
        let triangle = usize::from(bytes.u16()?);
        bytes.skip(4)?;
        let door = bytes.u32()?;
        if triangle >= triangles {
            return None;
        }
        doors.push(door);
    }
    Some(doors)
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
        assert_eq!(doors(&field(15, 2, &[(0, 1)])), None);
        // A door on a triangle the mesh does not have.
        assert_eq!(doors(&field(12, 2, &[(2, 1)])), None);
        let mut cut = field(12, 2, &[(0, 1)]);
        cut.truncate(70);
        assert_eq!(doors(&cut), None);
        assert_eq!(doors(&[]), None);
    }

    proptest! {
        #[test]
        fn never_panics(data in prop::collection::vec(any::<u8>(), 0..256)) {
            let _ = doors(&data);
        }

        #[test]
        fn never_panics_on_a_valid_start(rest in prop::collection::vec(any::<u8>(), 0..256)) {
            let mut data = SKYRIM_VERSION.to_le_bytes().to_vec();
            data.extend(rest);
            let _ = doors(&data);
        }
    }
}
