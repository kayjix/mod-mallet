//! The look of a worldspace from above, from the terrain LOD textures the
//! game and its mods ship (`textures/terrain/<world>/<world>.32.<x>.<y>.dds`).
//!
//! Each such tile covers 32 × 32 cells; its name gives the cell at its
//! south-west corner. Only loose files are read: that is where mods that
//! generate LOD put them. Nothing is written.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use mallet_formats_bethesda::dds;

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
    for (x, y, path) in &found {
        let Some(tile) = read_tile(path, pixels_per_cell) else {
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
}
