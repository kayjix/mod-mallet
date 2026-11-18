//! The look of a worldspace from above, from the terrain LOD textures the
//! game and its mods ship (`textures/terrain/<world>/<world>.32.<x>.<y>.dds`).
//!
//! Each such tile covers 32 × 32 cells; its name gives the cell at its
//! south-west corner. They are read as the game finds them, from its
//! archives and loose files ([`world_image_in`]), or from loose files alone
//! ([`world_image`]). Nothing is written.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use mallet_formats_bethesda::dds;

use crate::lod_library::LodLibrary;

/// How many cells one tile covers in each direction.
const TILE_CELLS: i32 = 32;

/// A picture of a worldspace: rows from north to south.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldImage {
    /// The westmost cell column it shows.
    pub west: i32,
    /// The southmost cell row it shows.
    pub south: i32,
    /// How many cell columns it shows.
    pub columns: u32,
    /// How many cell rows it shows.
    pub rows: u32,
    /// Pixels per cell in each direction.
    pub pixels_per_cell: u32,
    /// Red, green and blue of each pixel.
    pub rgb: Vec<u8>,
}

impl WorldImage {
    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.columns * self.pixels_per_cell
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.rows * self.pixels_per_cell
    }
}

/// The tiles of `world` in `data_dir`, by the cell at their south-west
/// corner.
fn tiles(data_dir: &Path, world: &str) -> Vec<(i32, i32, PathBuf)> {
    let world = world.to_ascii_lowercase();
    let terrain = data_dir.join("textures").join("terrain");
    // Folder names differ in case between mods; Windows does not care,
    // but the listing is compared by name.
    let Some(folder) = std::fs::read_dir(&terrain).ok().and_then(|entries| {
        entries.filter_map(Result::ok).find_map(|entry| {
            (entry.file_name().to_string_lossy().to_ascii_lowercase() == world)
                .then(|| entry.path())
        })
    }) else {
        return Vec::new();
    };
    let prefix = format!("{world}.32.");
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            let rest = name.strip_prefix(&prefix)?.strip_suffix(".dds")?;
            let (x, y) = rest.split_once('.')?;
            Some((x.parse().ok()?, y.parse().ok()?, entry.path()))
        })
        .collect()
}

/// Reads one level of a tile without reading the rest of the file.
fn read_tile(path: &Path, pixels_per_cell: u32) -> Option<dds::Rgb> {
    let mut file = File::open(path).ok()?;
    let mut header = vec![0u8; dds::HEADER_SIZE + dds::DX10_HEADER_SIZE];
    let read = file.read(&mut header).ok()?;
    header.truncate(read);
    let size = dds::size(&header).ok()?;
    let wanted = u32::try_from(TILE_CELLS).ok()? * pixels_per_cell;
    // The level whose width is the wanted one; tiles are square powers of two.
    let level = (size.width / wanted.max(1)).max(1).ilog2();
    let located = dds::locate_level(&header, level).ok()?;
    file.seek(SeekFrom::Start(u64::try_from(located.offset).ok()?))
        .ok()?;
    let mut data = vec![0u8; located.length];
    file.read_exact(&mut data).ok()?;
    let image = dds::decode(&located, &data);
    (image.width == wanted && image.height == wanted).then_some(image)
}

/// The picture of `world` from the LOD tiles in `data_dir`, with
/// `pixels_per_cell` pixels per cell (a power of two up to 64), or `None`
/// if the worldspace has no tiles there.
pub fn world_image(data_dir: &Path, world: &str, pixels_per_cell: u32) -> Option<WorldImage> {
    let found = tiles(data_dir, world);
    assemble(&found, pixels_per_cell, |path| {
        read_tile(path, pixels_per_cell)
            .or_else(|| tile_from_bytes(&std::fs::read(path).ok()?, pixels_per_cell))
    })
}

/// The picture of the worldspace of `library` from its LOD tiles, in the
/// versions the game uses: from the archives as well as loose files. As
/// [`world_image`] otherwise.
pub fn world_image_in(library: &LodLibrary, pixels_per_cell: u32) -> Option<WorldImage> {
    let found = library.land_pictures();
    assemble(&found, pixels_per_cell, |path| {
        let bytes = library.read(path)?;
        tile_from_bytes(&bytes, pixels_per_cell)
    })
}

/// One level of a tile held in memory, `pixels_per_cell` to a cell. A
/// tile without the smaller levels, as some mods ship them, is shrunk from
/// its full size.
fn tile_from_bytes(bytes: &[u8], pixels_per_cell: u32) -> Option<dds::Rgb> {
    let size = dds::size(bytes).ok()?;
    let wanted = u32::try_from(TILE_CELLS).ok()? * pixels_per_cell;
    let level = (size.width / wanted.max(1)).max(1).ilog2();
    let image = match dds::decode_level(bytes, level) {
        Ok(image) => image,
        Err(_) => shrink(&dds::decode_level(bytes, 0).ok()?, wanted)?,
    };
    (image.width == wanted && image.height == wanted).then_some(image)
}

/// A square picture shrunk to `side` pixels a side, each pixel the average
/// of the pixels it covers; `None` unless its side is a multiple of `side`.
fn shrink(image: &dds::Rgb, side: u32) -> Option<dds::Rgb> {
    if side == 0 || image.width != image.height || !image.width.is_multiple_of(side) {
        return None;
    }
    let factor = (image.width / side) as usize;
    let (width, side) = (image.width as usize, side as usize);
    let mut pixels = Vec::with_capacity(side * side * 3);
    for row in 0..side {
        for column in 0..side {
            let mut sum = [0usize; 3];
            for y in row * factor..(row + 1) * factor {
                let start = (y * width + column * factor) * 3;
                let source = image.pixels.get(start..start + factor * 3)?;
                for (channel, value) in source.iter().enumerate() {
                    if let Some(total) = sum.get_mut(channel % 3) {
                        *total += usize::from(*value);
                    }
                }
            }
            let count = factor * factor;
            pixels.extend(sum.map(|total| u8::try_from(total / count).unwrap_or(u8::MAX)));
        }
    }
    let side = u32::try_from(side).ok()?;
    Some(dds::Rgb {
        width: side,
        height: side,
        alpha: vec![u8::MAX; pixels.len() / 3],
        pixels,
    })
}

/// The picture put together from the tiles `found`, each read by `read`.
fn assemble<T>(
    found: &[(i32, i32, T)],
    pixels_per_cell: u32,
    read: impl Fn(&T) -> Option<dds::Rgb>,
) -> Option<WorldImage> {
    let west = found.iter().map(|(x, _, _)| *x).min()?;
    let south = found.iter().map(|(_, y, _)| *y).min()?;
    let east = found.iter().map(|(x, _, _)| *x).max()? + TILE_CELLS;
    let north = found.iter().map(|(_, y, _)| *y).max()? + TILE_CELLS;
    let columns = u32::try_from(east - west).ok()?;
    let rows = u32::try_from(north - south).ok()?;
    let width = (columns * pixels_per_cell) as usize;
    let height = (rows * pixels_per_cell) as usize;
    let mut rgb = vec![0u8; width.checked_mul(height)?.checked_mul(3)?];
    let tile_pixels = (TILE_CELLS as usize) * pixels_per_cell as usize;

    let mut any = false;
    for (x, y, path) in found {
        let Some(tile) = read(path) else {
            continue;
        };
        any = true;
        // The tile's top row is the north edge of its 32 cells.
        let left = usize::try_from(x - west).ok()? * pixels_per_cell as usize;
        let top = usize::try_from(north - (y + TILE_CELLS)).ok()? * pixels_per_cell as usize;
        for row in 0..tile_pixels {
            let source = tile
                .pixels
                .get(row * tile_pixels * 3..(row + 1) * tile_pixels * 3);
            let start = ((top + row) * width + left) * 3;
            let target = rgb.get_mut(start..start + tile_pixels * 3);
            if let (Some(source), Some(target)) = (source, target) {
                target.copy_from_slice(source);
            }
        }
    }
    any.then_some(WorldImage {
        west,
        south,
        columns,
        rows,
        pixels_per_cell,
        rgb,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A DXT1 tile of 64 × 64 pixels with all mipmaps, each level one colour.
    fn tile(color: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; dds::HEADER_SIZE];
        bytes[..4].copy_from_slice(b"DDS ");
        bytes[12..16].copy_from_slice(&64u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&64u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&7u32.to_le_bytes());
        bytes[84..88].copy_from_slice(b"DXT1");
        let mut side = 64u32;
        for _ in 0..7 {
            for _ in 0..side.div_ceil(4) * side.div_ceil(4) {
                bytes.extend(color.to_le_bytes());
                bytes.extend(0u16.to_le_bytes());
                bytes.extend(0u32.to_le_bytes());
            }
            side = (side / 2).max(1);
        }
        bytes
    }

    #[test]
    fn tiles_are_put_together_north_up() {
        let data = tempfile::tempdir().unwrap();
        let folder = data.path().join("textures/terrain/Tamriel");
        std::fs::create_dir_all(&folder).unwrap();
        // Red in the south-west, blue in the north-west.
        std::fs::write(folder.join("tamriel.32.0.0.dds"), tile(0xf800)).unwrap();
        std::fs::write(folder.join("tamriel.32.0.32.dds"), tile(0x001f)).unwrap();
        std::fs::write(folder.join("readme.txt"), b"not a tile").unwrap();

        let image = world_image(data.path(), "Tamriel", 1).unwrap();
        assert_eq!(
            (image.west, image.south, image.columns, image.rows),
            (0, 0, 32, 64)
        );
        assert_eq!((image.width(), image.height()), (32, 64));
        // The top row is the north: blue.
        assert_eq!(&image.rgb[..3], &[0, 0, 255]);
        let last = image.rgb.len() - 3;
        assert_eq!(&image.rgb[last..], &[255, 0, 0]);
    }

    #[test]
    fn a_world_without_tiles_has_no_image() {
        let data = tempfile::tempdir().unwrap();
        assert_eq!(world_image(data.path(), "Tamriel", 4), None);
    }

    #[test]
    fn a_tile_in_memory_gives_the_level_of_the_wanted_size() {
        let image = tile_from_bytes(&tile(0xf800), 1).unwrap();
        assert_eq!((image.width, image.height), (32, 32));
        assert_eq!(&image.pixels[..3], &[255, 0, 0]);
        // Larger than the tile itself: none.
        assert_eq!(tile_from_bytes(&tile(0xf800), 4), None);
    }

    #[test]
    fn a_tile_without_smaller_levels_is_shrunk() {
        // Only its full size, 64 pixels a side: shrunk to 32.
        let mut bytes = tile(0x001f);
        bytes[28..32].copy_from_slice(&1u32.to_le_bytes());
        bytes.truncate(dds::HEADER_SIZE + 16 * 16 * 8);
        let image = tile_from_bytes(&bytes, 1).unwrap();
        assert_eq!((image.width, image.height), (32, 32));
        assert_eq!(&image.pixels[..3], &[0, 0, 255]);
    }

    proptest::proptest! {
        #[test]
        fn a_tile_in_memory_never_panics(
            bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..400),
            pixels_per_cell in 0u32..8,
        ) {
            let _ = tile_from_bytes(&bytes, pixels_per_cell);
        }
    }
}
