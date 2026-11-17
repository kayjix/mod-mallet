//! `DirectDraw` Surface textures, as far as Mod Mallet needs them: the
//! colours of one mipmap level of a block-compressed texture.
//!
//! The game's terrain LOD textures are BC1 (`DXT1`) images, 2048 pixels
//! square with a full chain of mipmaps. A map needs a few pixels per cell,
//! so a smaller level is decoded, and only that level has to be read from
//! the file: [`locate_level`] says where it is, [`decode`] turns its bytes
//! into colours. BC3 (`DXT5`) and BC7 textures are read for their colour
//! as well; their alpha is ignored.

use thiserror::Error;

/// The size of the file header including the magic number. Reading this
/// many bytes, plus [`DX10_HEADER_SIZE`], is enough for [`locate_level`].
pub const HEADER_SIZE: usize = 128;
/// The size of the extended header that follows when the format is `DX10`.
pub const DX10_HEADER_SIZE: usize = 20;

/// What can be wrong with a texture file.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DdsError {
    /// The file does not start like a DDS file or is cut short.
    #[error("not a DDS texture, or cut short")]
    NotDds,
    /// The texture uses a format this reader does not decode.
    #[error("the texture format `{0}` is not supported")]
    Unsupported(String),
    /// The requested mipmap level is not in the file.
    #[error("the texture has no mipmap level {0}")]
    NoSuchLevel(u32),
}

/// A decoded image: `width × height` pixels, three bytes each, rows from
/// top to bottom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Red, green and blue of each pixel.
    pub pixels: Vec<u8>,
    /// The opacity of each pixel, 255 for opaque.
    pub alpha: Vec<u8>,
}

/// The block-compressed formats this reader decodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 8 bytes per block of 4 × 4 pixels: colour only.
    Bc1,
    /// 16 bytes per block: 4 bits of alpha per pixel, then a BC1 colour block.
    Bc2,
    /// 16 bytes per block: 8 bytes of alpha, then a BC1 colour block.
    Bc3,
    /// 16 bytes per block in one of eight modes; see `crate::bc7`.
    Bc7,
    /// Not compressed: four bytes per pixel, red, green, blue, alpha.
    Rgba8,
    /// Not compressed: four bytes per pixel, blue, green, red, alpha.
    Bgra8,
}

impl Format {
    fn block_size(self) -> usize {
        match self {
            Self::Bc1 => 8,
            Self::Bc2 | Self::Bc3 | Self::Bc7 => 16,
            // A "block" of an uncompressed texture is one pixel.
            Self::Rgba8 | Self::Bgra8 => 4,
        }
    }

    fn compressed(self) -> bool {
        !matches!(self, Self::Rgba8 | Self::Bgra8)
    }
}

/// The width and height of a texture, and how many mipmap levels it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Width of the largest level in pixels.
    pub width: u32,
    /// Height of the largest level in pixels.
    pub height: u32,
    /// Number of mipmap levels, at least 1.
    pub levels: u32,
}

/// Where one mipmap level lies in a texture file, and what it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// The format of its blocks.
    pub format: Format,
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// Where its bytes start, from the beginning of the file.
    pub offset: usize,
    /// How many bytes it has.
    pub length: usize,
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset.checked_add(4)?)?;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

/// Reads the header: the size, the format and where the pixel data starts.
fn header(bytes: &[u8]) -> Result<(Size, Format, usize), DdsError> {
    if bytes.get(..4) != Some(b"DDS ") || bytes.len() < HEADER_SIZE {
        return Err(DdsError::NotDds);
    }
    let height = u32_at(bytes, 12).ok_or(DdsError::NotDds)?;
    let width = u32_at(bytes, 16).ok_or(DdsError::NotDds)?;
    let levels = u32_at(bytes, 28).ok_or(DdsError::NotDds)?.max(1);
    let four_cc = bytes.get(84..88).ok_or(DdsError::NotDds)?;
    let (format, start) = match four_cc {
        b"DXT1" => (Format::Bc1, HEADER_SIZE),
        b"DXT3" | b"DXT2" => (Format::Bc2, HEADER_SIZE),
        b"DXT5" | b"DXT4" => (Format::Bc3, HEADER_SIZE),
        b"DX10" => {
            let dxgi = u32_at(bytes, HEADER_SIZE).ok_or(DdsError::NotDds)?;
            let format = match dxgi {
                // BC1_TYPELESS, BC1_UNORM, BC1_UNORM_SRGB
                70..=72 => Format::Bc1,
                // BC3_TYPELESS, BC3_UNORM, BC3_UNORM_SRGB
                // BC2_TYPELESS, BC2_UNORM, BC2_UNORM_SRGB
                73..=75 => Format::Bc2,
                76..=78 => Format::Bc3,
                // BC7_TYPELESS, BC7_UNORM, BC7_UNORM_SRGB
                97..=99 => Format::Bc7,
                other => return Err(DdsError::Unsupported(format!("DXGI {other}"))),
            };
            (format, HEADER_SIZE + DX10_HEADER_SIZE)
        }
        // No compression code: plain pixels, described by bit masks. Only
        // 32-bit pixels with red in the first or the third byte are read.
        [0, 0, 0, 0] if uncompressed_32(bytes) => {
            let red_mask = u32_at(bytes, 92).ok_or(DdsError::NotDds)?;
            match red_mask {
                0x0000_00ff => (Format::Rgba8, HEADER_SIZE),
                0x00ff_0000 => (Format::Bgra8, HEADER_SIZE),
                other => {
                    return Err(DdsError::Unsupported(format!("red mask {other:#x}")));
                }
            }
        }
        other => {
            return Err(DdsError::Unsupported(
                String::from_utf8_lossy(other).into_owned(),
            ));
        }
    };
    if width == 0 || height == 0 {
        return Err(DdsError::NotDds);
    }
    Ok((
        Size {
            width,
            height,
            levels,
        },
        format,
        start,
    ))
}

/// The size of a texture, from its first bytes.
///
/// # Errors
///
/// If the bytes are not the start of a DDS texture of a supported format.
pub fn size(bytes: &[u8]) -> Result<Size, DdsError> {
    header(bytes).map(|(size, _, _)| size)
}

/// Whether the header describes plain pixels of 32 bits: the pixel format
/// says "RGB" and 32 bits per pixel.
fn uncompressed_32(bytes: &[u8]) -> bool {
    const RGB: u32 = 0x40;
    u32_at(bytes, 80).is_some_and(|flags| flags & RGB != 0) && u32_at(bytes, 88) == Some(32)
}

/// The number of bytes of a level of `width × height` pixels.
fn level_bytes(width: u32, height: u32, format: Format) -> usize {
    if !format.compressed() {
        return (width.max(1) as usize)
            .saturating_mul(height.max(1) as usize)
            .saturating_mul(4);
    }
    let blocks_wide = width.div_ceil(4).max(1) as usize;
    let blocks_high = height.div_ceil(4).max(1) as usize;
    blocks_wide
        .saturating_mul(blocks_high)
        .saturating_mul(format.block_size())
}

/// Where mipmap level `level` (0 is the full size) lies in the file whose
/// first bytes are `header`.
///
/// # Errors
///
/// If the bytes are not the start of a DDS texture of a supported format,
/// or the texture has fewer levels.
pub fn locate_level(header_bytes: &[u8], level: u32) -> Result<Level, DdsError> {
    let (size, format, mut offset) = header(header_bytes)?;
    if level >= size.levels || level >= 32 {
        return Err(DdsError::NoSuchLevel(level));
    }
    let mut width = size.width;
    let mut height = size.height;
    for _ in 0..level {
        offset = offset.saturating_add(level_bytes(width, height, format));
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    Ok(Level {
        format,
        width,
        height,
        offset,
        length: level_bytes(width, height, format),
    })
}

/// Decodes the bytes of `level`, as read from where [`locate_level`] says.
/// Blocks missing at the end stay black.
pub fn decode(level: &Level, data: &[u8]) -> Rgb {
    let width = level.width as usize;
    let height = level.height as usize;
    let mut pixels = vec![0u8; width.saturating_mul(height).saturating_mul(3)];
    let mut alpha = vec![255u8; width.saturating_mul(height)];
    if !level.format.compressed() {
        let order = if level.format == Format::Bgra8 {
            [2, 1, 0]
        } else {
            [0, 1, 2]
        };
        for (index, pixel) in data
            .as_chunks::<4>()
            .0
            .iter()
            .take(width * height)
            .enumerate()
        {
            let channel = |at: usize| pixel.get(at).copied().unwrap_or(0);
            if let Some(target) = pixels.get_mut(index * 3..index * 3 + 3) {
                target.copy_from_slice(&order.map(channel));
            }
            if let Some(target) = alpha.get_mut(index) {
                *target = channel(3);
            }
        }
        return Rgb {
            width: level.width,
            height: level.height,
            pixels,
            alpha,
        };
    }
    let blocks_wide = width.div_ceil(4).max(1);
    for (index, block) in data.chunks_exact(level.format.block_size()).enumerate() {
        let (texels, opacity) = match level.format {
            Format::Bc1 => (bc1_block(block), bc1_alpha(block)),
            Format::Bc2 => (
                bc1_block(block.get(8..).unwrap_or_default()),
                bc2_alpha(block),
            ),
            Format::Bc3 => (
                bc1_block(block.get(8..).unwrap_or_default()),
                bc3_alpha(block),
            ),
            Format::Bc7 => {
                let decoded = crate::bc7::decode_block(block);
                (
                    decoded.map(|[r, g, b, _]| [r, g, b]),
                    decoded.map(|[_, _, _, a]| a),
                )
            }
            // Read above, pixel by pixel.
            Format::Rgba8 | Format::Bgra8 => ([[0; 3]; 16], [255; 16]),
        };
        let block_x = (index % blocks_wide) * 4;
        let block_y = (index / blocks_wide) * 4;
        for (texel, rgb) in texels.iter().enumerate() {
            let x = block_x + texel % 4;
            let y = block_y + texel / 4;
            if x >= width || y >= height {
                continue;
            }
            let at = (y * width + x) * 3;
            if let Some(target) = pixels.get_mut(at..at + 3) {
                target.copy_from_slice(rgb);
            }
            if let (Some(target), Some(value)) = (alpha.get_mut(y * width + x), opacity.get(texel))
            {
                *target = *value;
            }
        }
    }
    Rgb {
        width: level.width,
        height: level.height,
        pixels,
        alpha,
    }
}

/// The opacity of a BC1 block: in its three-colour mode, index 3 is clear.
fn bc1_alpha(block: &[u8]) -> [u8; 16] {
    let word = |at: usize| {
        block
            .get(at..at + 2)
            .and_then(|bytes| bytes.try_into().ok())
            .map_or(0, u16::from_le_bytes)
    };
    let mut opacity = [255u8; 16];
    if word(0) <= word(2) {
        let indices = block
            .get(4..8)
            .and_then(|bytes| bytes.try_into().ok())
            .map_or(0, u32::from_le_bytes);
        for (texel, value) in opacity.iter_mut().enumerate() {
            if (indices >> (texel * 2)) & 0b11 == 3 {
                *value = 0;
            }
        }
    }
    opacity
}

/// The opacity of a BC2 block: four bits per pixel, written out.
fn bc2_alpha(block: &[u8]) -> [u8; 16] {
    let mut opacity = [255u8; 16];
    for (texel, value) in opacity.iter_mut().enumerate() {
        let byte = block.get(texel / 2).copied().unwrap_or(0xff);
        let nibble = if texel % 2 == 0 {
            byte & 0x0f
        } else {
            byte >> 4
        };
        *value = nibble * 17;
    }
    opacity
}

/// The opacity of a BC3 block: two endpoints and a 3-bit index per pixel.
fn bc3_alpha(block: &[u8]) -> [u8; 16] {
    let (first, second) = (
        u16::from(block.first().copied().unwrap_or(255)),
        u16::from(block.get(1).copied().unwrap_or(255)),
    );
    let palette: [u16; 8] = if first > second {
        [
            first,
            second,
            (6 * first + second) / 7,
            (5 * first + 2 * second) / 7,
            (4 * first + 3 * second) / 7,
            (3 * first + 4 * second) / 7,
            (2 * first + 5 * second) / 7,
            (first + 6 * second) / 7,
        ]
    } else {
        [
            first,
            second,
            (4 * first + second) / 5,
            (3 * first + 2 * second) / 5,
            (2 * first + 3 * second) / 5,
            (first + 4 * second) / 5,
            0,
            255,
        ]
    };
    let mut bits = [0u8; 8];
    if let Some(stored) = block.get(2..8) {
        bits[..6].copy_from_slice(stored);
    }
    let indices = u64::from_le_bytes(bits);
    let mut opacity = [255u8; 16];
    for (texel, value) in opacity.iter_mut().enumerate() {
        let index = ((indices >> (texel * 3)) & 0b111) as usize;
        *value = palette
            .get(index)
            .and_then(|level| u8::try_from(*level).ok())
            .unwrap_or(255);
    }
    opacity
}

/// Decodes mipmap level `level` of a whole texture file.
///
/// # Errors
///
/// If the file is not a DDS texture of a supported format, or the level is
/// not in it.
pub fn decode_level(bytes: &[u8], level: u32) -> Result<Rgb, DdsError> {
    let located = locate_level(bytes, level)?;
    let data = bytes
        .get(located.offset..located.offset.saturating_add(located.length))
        .ok_or(DdsError::NoSuchLevel(level))?;
    Ok(decode(&located, data))
}

/// The 16 colours of one BC1 block, row by row.
fn bc1_block(block: &[u8]) -> [[u8; 3]; 16] {
    let word = |at: usize| {
        block
            .get(at..at + 2)
            .and_then(|bytes| bytes.try_into().ok())
            .map_or(0, u16::from_le_bytes)
    };
    let first = word(0);
    let second = word(2);
    let (a, b) = (rgb565(first), rgb565(second));
    let mix = |weight_a: u16, weight_b: u16, total: u16| -> [u8; 3] {
        let channel = |index: usize| {
            let from = |colour: [u8; 3]| u16::from(colour.get(index).copied().unwrap_or(0));
            let value = (from(a) * weight_a + from(b) * weight_b) / total;
            u8::try_from(value).unwrap_or(u8::MAX)
        };
        [channel(0), channel(1), channel(2)]
    };
    let palette = if first > second {
        [a, b, mix(2, 1, 3), mix(1, 2, 3)]
    } else {
        // The fourth colour is transparent black; black it is here.
        [a, b, mix(1, 1, 2), [0, 0, 0]]
    };
    let indices = block
        .get(4..8)
        .and_then(|bytes| bytes.try_into().ok())
        .map_or(0, u32::from_le_bytes);
    let mut texels = [[0u8; 3]; 16];
    for (texel, target) in texels.iter_mut().enumerate() {
        let index = (indices >> (texel * 2)) & 0b11;
        *target = palette.get(index as usize).copied().unwrap_or_default();
    }
    texels
}

/// A 16-bit colour, 5 bits red, 6 green, 5 blue, as 8-bit channels.
fn rgb565(value: u16) -> [u8; 3] {
    let red = (value >> 11) & 0x1f;
    let green = (value >> 5) & 0x3f;
    let blue = value & 0x1f;
    let widen = |channel: u16, bits: u32| {
        let max = (1u16 << bits) - 1;
        u8::try_from(u32::from(channel) * 255 / u32::from(max)).unwrap_or(u8::MAX)
    };
    [widen(red, 5), widen(green, 6), widen(blue, 5)]
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A DXT1 texture of `width × height` with the given levels, each level
    /// filled with blocks of one colour.
    fn texture(width: u32, height: u32, colors: &[u16]) -> Vec<u8> {
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[..4].copy_from_slice(b"DDS ");
        bytes[4..8].copy_from_slice(&124u32.to_le_bytes());
        bytes[12..16].copy_from_slice(&height.to_le_bytes());
        bytes[16..20].copy_from_slice(&width.to_le_bytes());
        bytes[28..32].copy_from_slice(&u32::try_from(colors.len()).unwrap().to_le_bytes());
        bytes[84..88].copy_from_slice(b"DXT1");
        let (mut w, mut h) = (width, height);
        for color in colors {
            let blocks = w.div_ceil(4) * h.div_ceil(4);
            for _ in 0..blocks {
                bytes.extend(color.to_le_bytes());
                bytes.extend(0u16.to_le_bytes());
                // Every texel takes the first colour.
                bytes.extend(0u32.to_le_bytes());
            }
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        bytes
    }

    /// An uncompressed 32-bit texture of 2 × 2 with two levels; red in the
    /// first byte unless `bgra`.
    fn plain(bgra: bool) -> Vec<u8> {
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[..4].copy_from_slice(b"DDS ");
        bytes[12..16].copy_from_slice(&2u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&2u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&2u32.to_le_bytes());
        bytes[80..84].copy_from_slice(&0x41u32.to_le_bytes());
        bytes[88..92].copy_from_slice(&32u32.to_le_bytes());
        let red: u32 = if bgra { 0x00ff_0000 } else { 0x0000_00ff };
        bytes[92..96].copy_from_slice(&red.to_le_bytes());
        for _ in 0..4 {
            bytes.extend([10, 20, 30, 128]);
        }
        bytes.extend([1, 2, 3, 255]);
        bytes
    }

    #[test]
    fn a_dxt3_block_keeps_its_written_out_alpha() {
        let mut bytes = vec![0u8; HEADER_SIZE];
        bytes[..4].copy_from_slice(b"DDS ");
        bytes[12..16].copy_from_slice(&4u32.to_le_bytes());
        bytes[16..20].copy_from_slice(&4u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&1u32.to_le_bytes());
        bytes[84..88].copy_from_slice(b"DXT3");
        // Alpha: in every row, one clear pixel, then three opaque ones.
        bytes.extend([0xf0u8, 0xff].repeat(4));
        bytes.extend(0xf800u16.to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        bytes.extend(0u32.to_le_bytes());
        let decoded = decode_level(&bytes, 0).unwrap();
        assert_eq!(&decoded.pixels[..3], &[255, 0, 0]);
        assert_eq!(&decoded.alpha[..4], &[0, 255, 255, 255]);
    }

    #[test]
    fn uncompressed_pixels_are_read_in_either_byte_order() {
        let rgba = decode_level(&plain(false), 0).unwrap();
        assert_eq!((rgba.width, rgba.height), (2, 2));
        assert_eq!(&rgba.pixels[..3], &[10, 20, 30]);
        assert_eq!(rgba.alpha[0], 128);
        let bgra = decode_level(&plain(true), 0).unwrap();
        assert_eq!(&bgra.pixels[..3], &[30, 20, 10]);
        let smallest = decode_level(&plain(false), 1).unwrap();
        assert_eq!(smallest.pixels, vec![1, 2, 3]);
    }

    #[test]
    fn a_level_is_decoded_in_its_colour() {
        const RED: u16 = 0xf800;
        const GREEN: u16 = 0x07e0;
        let bytes = texture(8, 8, &[RED, GREEN, RED, GREEN]);
        let full = decode_level(&bytes, 0).unwrap();
        assert_eq!((full.width, full.height), (8, 8));
        assert_eq!(&full.pixels[..3], &[255, 0, 0]);
        let half = decode_level(&bytes, 1).unwrap();
        assert_eq!((half.width, half.height), (4, 4));
        assert_eq!(&half.pixels[..3], &[0, 255, 0]);
        let smallest = decode_level(&bytes, 3).unwrap();
        assert_eq!((smallest.width, smallest.height), (1, 1));
        assert_eq!(smallest.pixels, vec![0, 255, 0]);
    }

    #[test]
    fn a_level_is_found_from_the_header_alone() {
        let bytes = texture(8, 8, &[1, 2, 3, 4]);
        let level = locate_level(&bytes[..HEADER_SIZE], 1).unwrap();
        // 4 blocks of the first level come before it.
        assert_eq!(level.offset, HEADER_SIZE + 4 * 8);
        assert_eq!(level.length, 8);
        assert_eq!((level.width, level.height), (4, 4));
    }

    #[test]
    fn alpha_is_kept() {
        // BC3: endpoints 255 and 0, all pixels index 1: clear.
        let mut block = vec![255u8, 0];
        block.extend([
            0b0100_1001,
            0b1001_0010,
            0b0010_0100,
            0b0100_1001,
            0b1001_0010,
            0b0010_0100,
        ]);
        assert_eq!(bc3_alpha(&block), [0; 16]);
        // BC1 in its three-colour mode: index 3 is clear.
        let mut block = Vec::new();
        block.extend(0u16.to_le_bytes());
        block.extend(0xffffu16.to_le_bytes());
        block.extend(0b11u32.to_le_bytes());
        let opacity = bc1_alpha(&block);
        assert_eq!(opacity[0], 0);
        assert_eq!(opacity[1], 255);
    }

    #[test]
    fn a_missing_level_is_reported() {
        let bytes = texture(8, 8, &[0x001f]);
        assert_eq!(decode_level(&bytes, 1), Err(DdsError::NoSuchLevel(1)));
    }

    #[test]
    fn other_formats_are_refused() {
        let mut bytes = texture(4, 4, &[0]);
        bytes[84..88].copy_from_slice(b"ATI2");
        assert_eq!(
            decode_level(&bytes, 0),
            Err(DdsError::Unsupported("ATI2".to_owned()))
        );
        assert_eq!(decode_level(b"not a texture", 0), Err(DdsError::NotDds));
    }

    #[test]
    fn the_two_colour_mode_mixes_halfway() {
        // first <= second: the third colour is the average.
        let mut block = Vec::new();
        block.extend(0x0000u16.to_le_bytes());
        block.extend(0xffffu16.to_le_bytes());
        block.extend(0b10u32.to_le_bytes());
        let texels = bc1_block(&block);
        assert_eq!(texels[0], [127, 127, 127]);
        assert_eq!(texels[1], [0, 0, 0]);
    }

    proptest! {
        #[test]
        fn never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512), level in 0u32..40) {
            let _ = decode_level(&bytes, level);
            let _ = size(&bytes);
        }

        #[test]
        fn never_panics_on_a_valid_header(
            width in 0u32..64, height in 0u32..64, levels in 0u32..40,
            body in proptest::collection::vec(any::<u8>(), 0..256), level in 0u32..40,
        ) {
            let mut bytes = texture(4, 4, &[]);
            bytes[12..16].copy_from_slice(&height.to_le_bytes());
            bytes[16..20].copy_from_slice(&width.to_le_bytes());
            bytes[28..32].copy_from_slice(&levels.to_le_bytes());
            bytes.extend(body);
            let _ = decode_level(&bytes, level);
        }
    }
}
