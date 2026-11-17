//! Reads which texture a material file (`.bgsm`, `.bgem`) gives a surface.
//!
//! From Fallout 4 on, a mesh often does not name its textures itself. Its
//! shader names a material file instead, which holds the textures and how
//! the surface is drawn. Two kinds exist: `BGSM` for ordinary surfaces,
//! `BGEM` for effects such as glass and glow. Both start with the same
//! settings, followed by the texture paths as length-prefixed texts:
//!
//! ```text
//! "BGSM" or "BGEM", version
//! tiling flags, texture offset and scale, alpha
//! blending: one byte and two numbers; alpha test: two bytes
//! ten switches (depth, reflections, decal, two-sided, refraction ...)
//! refraction power
//! version < 10: environment mapping switch and mask scale
//! version >= 10: depth bias switch
//! grey-scale-to-palette switch
//! version >= 6: which channels are written
//! the textures: the colour map first
//! ```
//!
//! The layout was checked against the 6,899 material files Fallout 4
//! ships, all of version 2. A material file is untrusted input like any
//! other (ADR 0013).

use crate::{Bytes, decode_text};

/// The longest texture path that is read. Real ones are below 260.
const MAX_PATH: usize = 4096;

/// Why a material file could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MaterialError {
    /// The file does not start like a material file.
    #[error("the file is not a material file")]
    NotAMaterial,
    /// The data ends too early, or a size in it is wrong.
    #[error("the material file is damaged or cut off")]
    Damaged,
}

/// The texture a material gives the surface's colour: the colour map of a
/// `.bgsm`, the base texture of a `.bgem`. `None` if the slot is empty.
pub fn main_texture(data: &[u8]) -> Result<Option<String>, MaterialError> {
    let mut bytes = Bytes::new(data);
    let signature = bytes.signature().ok_or(MaterialError::NotAMaterial)?;
    if &signature != b"BGSM" && &signature != b"BGEM" {
        return Err(MaterialError::NotAMaterial);
    }
    let damaged = || MaterialError::Damaged;
    let version = bytes.u32().ok_or_else(damaged)?;
    // Tiling flags, texture offset and scale, alpha.
    bytes.skip(4 + 16 + 4).ok_or_else(damaged)?;
    // Blending: on or off, source and target factors; alpha test reference
    // and switch.
    bytes.skip(1 + 4 + 4 + 1 + 1).ok_or_else(damaged)?;
    // Depth write and test, reflections, wet reflections, decal,
    // two-sided, decal without fading, non-occluder, refraction and its
    // falloff.
    bytes.skip(10).ok_or_else(damaged)?;
    bytes.skip(4).ok_or_else(damaged)?; // refraction power
    if version < 10 {
        bytes.skip(1 + 4).ok_or_else(damaged)?; // environment mapping
    } else {
        bytes.skip(1).ok_or_else(damaged)?; // depth bias
    }
    bytes.skip(1).ok_or_else(damaged)?; // grey scale to palette
    if version >= 6 {
        bytes.skip(1).ok_or_else(damaged)?; // channels written
    }

    let length = usize::try_from(bytes.u32().ok_or_else(damaged)?).map_err(|_| damaged())?;
    if length > MAX_PATH {
        return Err(MaterialError::Damaged);
    }
    let text = bytes.take(length).ok_or_else(damaged)?;
    let text = text.split(|&byte| byte == 0).next().unwrap_or_default();
    let path = decode_text(text);
    Ok((!path.is_empty()).then_some(path))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A material of `kind` and `version` whose first texture is `texture`.
    fn material(kind: [u8; 4], version: u32, texture: &str) -> Vec<u8> {
        let mut data = kind.to_vec();
        data.extend(version.to_le_bytes());
        data.extend([0u8; 24]);
        data.extend([0u8; 11]);
        data.extend([1u8; 10]);
        data.extend([0u8; 4]);
        data.extend(if version < 10 {
            vec![0u8; 5]
        } else {
            vec![0u8; 1]
        });
        data.push(0);
        if version >= 6 {
            data.push(0);
        }
        data.extend(u32::try_from(texture.len() + 1).unwrap().to_le_bytes());
        data.extend(texture.as_bytes());
        data.push(0);
        data.extend(5u32.to_le_bytes());
        data.extend(b"next\0");
        data
    }

    #[test]
    fn the_colour_map_of_a_fallout_4_material_is_read() {
        let data = material(*b"BGSM", 2, "Props/NukaColaFull_d.dds");
        // The texture starts where it does in the game's own files.
        assert_eq!(data.get(63..67), Some(&25u32.to_le_bytes()[..]));
        assert_eq!(
            main_texture(&data),
            Ok(Some("Props/NukaColaFull_d.dds".to_owned()))
        );
    }

    #[test]
    fn effect_materials_and_newer_versions_are_read() {
        let data = material(*b"BGEM", 2, "Effects/Glow_d.dds");
        assert_eq!(
            main_texture(&data),
            Ok(Some("Effects/Glow_d.dds".to_owned()))
        );
        let data = material(*b"BGSM", 20, "Newer/Wall_d.dds");
        assert_eq!(main_texture(&data), Ok(Some("Newer/Wall_d.dds".to_owned())));
    }

    #[test]
    fn an_empty_slot_is_no_texture() {
        assert_eq!(main_texture(&material(*b"BGSM", 2, "")), Ok(None));
    }

    #[test]
    fn other_files_and_cut_off_materials_are_errors() {
        assert_eq!(main_texture(b"DDS "), Err(MaterialError::NotAMaterial));
        assert_eq!(main_texture(b"BG"), Err(MaterialError::NotAMaterial));
        let data = material(*b"BGSM", 2, "Props/Cut_d.dds");
        assert_eq!(main_texture(&data[..70]), Err(MaterialError::Damaged));
        let mut huge = data.clone();
        huge.splice(63..67, 1_000_000u32.to_le_bytes());
        assert_eq!(main_texture(&huge), Err(MaterialError::Damaged));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = main_texture(&data);
        }

        #[test]
        fn never_panics_on_a_damaged_material(
            version in 0u32..30,
            cut in 0usize..120,
            texture in "[a-z/_.]{0,40}",
        ) {
            let data = material(*b"BGSM", version, &texture);
            let _ = main_texture(data.get(..cut.min(data.len())).unwrap_or_default());
        }
    }
}
