//! Reads the meshes of Oblivion (format versions 10.1.0.106, 10.2.0.0,
//! 20.0.0.4 and 20.0.0.5).
//!
//! Unlike the later games' meshes, these do not say how long each block
//! is. A block can only be found by reading every block before it, so this
//! reader knows the layout of the block types the game's own meshes use
//! most, including the animations of doors and gates (checked against all
//! of them, see [`layout`]). A block of a type it does not know, such as a
//! particle system, is measured by where the next block it knows starts.
//!
//! Textures are named by `NiSourceTexture` blocks, which hang from a
//! shape's `NiTexturingProperty`. Shapes are `NiTriShape` (triangles) and
//! `NiTriStrips` (strips of triangles).

use std::collections::HashSet;

use crate::nif::{BlockInfo, NifError};
use crate::nif_geometry::{Shape, Transform, finish, read_rotation};
use crate::{Bytes, decode_text};

/// The versions of the format Oblivion uses: 10.2.0.0 for some of its
/// older meshes, 20.0.0.4 and 20.0.0.5 for the rest.
const VERSIONS: [u32; 4] = [0x0A01_006A, 0x0A02_0000, 0x1400_0004, 0x1400_0005];

/// The first version whose header says in which order its bytes are.
const BYTE_ORDER_SINCE: u32 = 0x1400_0003;

/// The longest text inside a mesh that is read.
const MAX_TEXT: usize = 64 * 1024;

/// Most entries a list in a block is believed to have.
const MAX_LIST: usize = 1 << 20;

/// The version of the format a mesh is in, for the few fields that come
/// and go between Oblivion's versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Format {
    version: u32,
}

impl Format {
    /// The newest of Oblivion's versions.
    #[cfg(test)]
    const NEWEST: Self = Self {
        version: 0x1400_0005,
    };

    /// Whether each block is preceded by a number, zero in the game's
    /// meshes.
    const fn has_block_prefix(self) -> bool {
        self.version < 0x0A02_0000
    }

    /// Whether this reader knows how animation sequences, blends and
    /// morphs are laid out; they changed up to 10.2.0.0.
    const fn knows_animations(self) -> bool {
        self.version >= 0x0A02_0000
    }

    /// Whether geometry data starts with the number of its group.
    const fn has_geometry_group(self) -> bool {
        self.version >= 0x0A01_0072
    }

    /// Whether geometry data ends with a reference to additional data.
    const fn has_additional_data(self) -> bool {
        self.version >= 0x1400_0004
    }

    /// Whether a texture slot keeps two numbers for the `PlayStation` 2.
    const fn has_ps2_numbers(self) -> bool {
        self.version <= 0x0A04_0001
    }
}

/// Reads the first line, the version and, where there is one, the byte
/// order: the format and whether its bytes are little-endian.
fn versions(bytes: &mut Bytes<'_>, line_len: usize) -> Option<(Format, bool)> {
    let line = bytes.take(line_len + 1)?;
    if !line.starts_with(b"Gamebryo File Format, Version ") {
        return None;
    }
    let version = bytes.u32()?;
    let little_endian = version < BYTE_ORDER_SINCE || bytes.u8()? == 1;
    Some((Format { version }, little_endian))
}

/// Whether `data` starts like a mesh in one of Oblivion's versions. Only
/// the first line and the version numbers are looked at.
pub(crate) fn has_oblivion_version(data: &[u8]) -> bool {
    let Some(line_len) = data.iter().take(64).position(|&byte| byte == b'\n') else {
        return false;
    };
    matches!(
        versions(&mut Bytes::new(data), line_len),
        Some((format, true)) if VERSIONS.contains(&format.version)
    )
}

/// What the header says: the version, the types of the blocks, and where
/// they start.
struct Header {
    format: Format,
    kinds: Vec<String>,
    blocks_start: usize,
}

fn header(data: &[u8]) -> Result<Header, NifError> {
    let mut bytes = Bytes::new(data);
    let damaged = || NifError::Damaged;
    let line_len = data
        .iter()
        .take(64)
        .position(|&byte| byte == b'\n')
        .ok_or(NifError::NotAMesh)?;
    let (format, little_endian) = versions(&mut bytes, line_len).ok_or(NifError::NotAMesh)?;
    let user_version = bytes.u32().ok_or_else(damaged)?;
    if !VERSIONS.contains(&format.version) || !little_endian || !(10..=11).contains(&user_version) {
        return Err(NifError::UnsupportedVersion);
    }
    let block_count = bytes.u32().ok_or_else(damaged)?;
    bytes.u32().ok_or_else(damaged)?; // Bethesda version
    for _ in 0..3 {
        // Author, process script, export script.
        let len = usize::from(bytes.u8().ok_or_else(damaged)?);
        bytes.skip(len).ok_or_else(damaged)?;
    }
    let type_count = bytes.u16().ok_or_else(damaged)?;
    let mut names = Vec::new();
    for _ in 0..type_count {
        names.push(decode_text(text(&mut bytes).ok_or_else(damaged)?));
    }
    let mut kinds = Vec::new();
    for _ in 0..block_count {
        let index = usize::from(bytes.u16().ok_or_else(damaged)?);
        kinds.push(names.get(index).cloned().ok_or_else(damaged)?);
    }
    // Groups of blocks, which the game's meshes do not use.
    let groups = count32(&mut bytes).ok_or_else(damaged)?;
    skip_n(&mut bytes, groups, 4).ok_or_else(damaged)?;
    Ok(Header {
        format,
        kinds,
        blocks_start: data.len() - bytes.remaining(),
    })
}

/// A text: its length, then its bytes.
fn text<'a>(bytes: &mut Bytes<'a>) -> Option<&'a [u8]> {
    let len = usize::try_from(bytes.u32()?).ok()?;
    if len > MAX_TEXT {
        return None;
    }
    bytes.take(len)
}

/// A count of list entries, refused if it is beyond belief.
fn count32(bytes: &mut Bytes<'_>) -> Option<usize> {
    usize::try_from(bytes.u32()?)
        .ok()
        .filter(|&n| n <= MAX_LIST)
}

/// Skips `count` entries of `size` bytes each.
fn skip_n(bytes: &mut Bytes<'_>, count: usize, size: usize) -> Option<()> {
    bytes.skip(count.checked_mul(size)?)
}

/// Name, extra data and controller: what every object starts with.
fn object(bytes: &mut Bytes<'_>) -> Option<()> {
    text(bytes)?;
    let extra = count32(bytes)?;
    skip_n(bytes, extra, 4)?;
    bytes.skip(4) // controller
}

/// What every node and shape starts with: the object, flags, place,
/// properties and collision. Returns the place and the properties.
fn av_object(bytes: &mut Bytes<'_>) -> Option<(Transform, Vec<usize>)> {
    object(bytes)?;
    bytes.u16()?; // flags
    let translation = [bytes.f32()?, bytes.f32()?, bytes.f32()?];
    let rotation = read_rotation(bytes)?;
    let scale = bytes.f32()?;
    let count = count32(bytes)?;
    let mut properties = Vec::new();
    for _ in 0..count {
        if let Some(property) = reference(bytes.u32()?) {
            properties.push(property);
        }
    }
    bytes.u32()?; // collision object
    let scale = if scale.is_finite() && scale != 0.0 {
        scale
    } else {
        1.0
    };
    Some((
        Transform {
            rotation,
            translation,
            scale,
        },
        properties,
    ))
}

/// A texture slot of a texturing property: the reference to its source,
/// as stored.
fn texture_slot(bytes: &mut Bytes<'_>, format: Format) -> Option<u32> {
    let source = bytes.u32()?;
    bytes.skip(4 + 4 + 4)?; // clamp, filter, texture coordinate set
    if format.has_ps2_numbers() {
        bytes.skip(2 + 2)?;
    }
    if bytes.u8()? != 0 {
        // Texture transform: translation, scale, rotation, method, centre.
        bytes.skip(8 + 8 + 4 + 4 + 8)?;
    }
    Some(source)
}

/// The texture in the first slot of a texturing property, the colour map,
/// as a reference as stored (`u32::MAX` for none).
fn texturing(bytes: &mut Bytes<'_>, format: Format) -> Option<u32> {
    object(bytes)?;
    bytes.u32()?; // apply mode
    let count = bytes.u32()?;
    let mut base = u32::MAX;
    // Base, dark, detail, gloss, glow and bump map.
    for slot in 0..6 {
        if bytes.u8()? != 0 {
            let source = texture_slot(bytes, format)?;
            if slot == 0 {
                base = source;
            }
            if slot == 5 {
                bytes.skip(4 + 4 + 16)?; // luma scale and offset, matrix
            }
        }
    }
    // Up to four decals, as many as the count allows.
    for decal in 0..4u32 {
        if count > 6 + decal && bytes.u8()? != 0 {
            texture_slot(bytes, format)?;
        }
    }
    let shader_textures = count32(bytes)?;
    for _ in 0..shader_textures {
        if bytes.u8()? != 0 {
            texture_slot(bytes, format)?;
            bytes.u32()?; // map
        }
    }
    Some(base)
}

/// The file a texture source names; empty if it names none.
fn source_file(bytes: &mut Bytes<'_>) -> Option<String> {
    object(bytes)?;
    let external = bytes.u8()? != 0;
    let name = text(bytes)?;
    bytes.skip(4)?; // pixel data, or nothing
    bytes.skip(12 + 1 + 1)?; // pixel layout, mipmaps, alpha, static, direct
    Some(if external {
        decode_text(name)
    } else {
        String::new()
    })
}

/// The points and first texture coordinates of geometry data, as stored.
/// Left as bytes, so that skipping a block costs nothing.
struct RawGeometry<'a> {
    positions: &'a [u8],
    uvs: &'a [u8],
}

/// Geometry data: points, texture coordinates. Leaves `bytes` at the
/// part that differs between triangles and strips.
fn geometry_data<'a>(bytes: &mut Bytes<'a>, format: Format) -> Option<RawGeometry<'a>> {
    if format.has_geometry_group() {
        bytes.u32()?;
    }
    let count = usize::from(bytes.u16()?);
    bytes.skip(2)?; // keep and compress flags
    let positions = if bytes.u8()? != 0 {
        bytes.take(count * 12)?
    } else {
        &[]
    };
    let flags = bytes.u16()?;
    if bytes.u8()? != 0 {
        skip_n(bytes, count, 12)?; // normals
        if flags & 0x1000 != 0 {
            skip_n(bytes, count, 24)?; // tangents and bitangents
        }
    }
    bytes.skip(16)?; // centre and radius
    if bytes.u8()? != 0 {
        skip_n(bytes, count, 16)?; // colours
    }
    let sets = usize::from(flags & 0x3F);
    let uvs = if sets > 0 {
        let first = bytes.take(count * 8)?;
        skip_n(bytes, count * (sets - 1), 8)?;
        first
    } else {
        &[]
    };
    bytes.u16()?; // consistency
    if format.has_additional_data() {
        bytes.u32()?;
    }
    Some(RawGeometry { positions, uvs })
}

/// Numbers stored one after another.
fn floats(raw: &[u8]) -> Vec<f32> {
    raw.as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect()
}

/// Corner numbers stored one after another.
fn corners(raw: &[u8]) -> impl Iterator<Item = u32> {
    raw.as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| u32::from(u16::from_le_bytes(*bytes)))
}

/// The corners of the triangles of `NiTriShapeData`, after its geometry
/// data, as stored.
fn triangle_corners<'a>(bytes: &mut Bytes<'a>) -> Option<&'a [u8]> {
    let count = usize::from(bytes.u16()?);
    bytes.u32()?; // number of triangle points
    let corners = if bytes.u8()? != 0 {
        bytes.take(count * 6)?
    } else {
        &[]
    };
    let groups = bytes.u16()?;
    for _ in 0..groups {
        let size = usize::from(bytes.u16()?);
        skip_n(bytes, size, 2)?;
    }
    Some(corners)
}

/// The lengths of the strips of `NiTriStripsData`, after its geometry
/// data, and their corners as stored.
fn strip_corners<'a>(bytes: &mut Bytes<'a>) -> Option<(Vec<usize>, &'a [u8])> {
    bytes.u16()?; // number of triangles
    let count = usize::from(bytes.u16()?);
    let mut lengths = Vec::new();
    for _ in 0..count {
        lengths.push(usize::from(bytes.u16()?));
    }
    let corners = if bytes.u8()? != 0 {
        bytes.take(lengths.iter().sum::<usize>() * 2)?
    } else {
        &[]
    };
    Some((lengths, corners))
}

/// Strips turned into triangles, every second one turned around.
fn strips(lengths: &[usize], raw: &[u8]) -> Vec<u32> {
    let mut all = corners(raw);
    let mut indices = Vec::new();
    for &length in lengths {
        let strip: Vec<u32> = all.by_ref().take(length).collect();
        for (at, corners) in strip.windows(3).enumerate() {
            let [a, b, c] = corners else { continue };
            if a == b || b == c || a == c {
                continue;
            }
            if at % 2 == 0 {
                indices.extend([*a, *b, *c]);
            } else {
                indices.extend([*a, *c, *b]);
            }
        }
    }
    indices
}

/// Skips one block of the type `kind`. `None` if the type is not known or
/// the block does not fit the data.
#[allow(clippy::too_many_lines)]
fn skip_block(kind: &str, format: Format, bytes: &mut Bytes<'_>) -> Option<()> {
    if !format.knows_animations()
        && matches!(
            kind,
            "NiControllerSequence"
                | "NiGeomMorpherController"
                | "NiBlendTransformInterpolator"
                | "NiBlendPoint3Interpolator"
                | "NiBlendFloatInterpolator"
                | "NiBlendBoolInterpolator"
        )
    {
        return None;
    }
    match kind {
        "NiNode" | "RootCollisionNode" | "AvoidNode" | "NiBSAnimationNode" | "NiBSParticleNode"
        | "NiBillboardNode" => {
            av_object(bytes)?;
            let children = count32(bytes)?;
            skip_n(bytes, children, 4)?;
            let effects = count32(bytes)?;
            skip_n(bytes, effects, 4)?;
            if kind == "NiBillboardNode" {
                bytes.u16()?;
            }
        }
        "NiTriShape" | "NiTriStrips" => {
            av_object(bytes)?;
            bytes.skip(4 + 4)?; // data, skin
            if bytes.u8()? != 0 {
                text(bytes)?;
                bytes.u32()?;
            }
        }
        "NiTriShapeData" => {
            geometry_data(bytes, format)?;
            triangle_corners(bytes)?;
        }
        "NiTriStripsData" => {
            geometry_data(bytes, format)?;
            strip_corners(bytes)?;
        }
        "BSXFlags" => {
            text(bytes)?;
            bytes.u32()?;
        }
        "NiStringExtraData" => {
            text(bytes)?;
            text(bytes)?;
        }
        "NiBinaryExtraData" => {
            text(bytes)?;
            let size = count32(bytes)?;
            bytes.skip(size)?;
        }
        "NiTextKeyExtraData" => {
            text(bytes)?;
            let keys = count32(bytes)?;
            for _ in 0..keys {
                bytes.f32()?;
                text(bytes)?;
            }
        }
        "BSFurnitureMarker" => {
            text(bytes)?;
            let positions = count32(bytes)?;
            skip_n(bytes, positions, 16)?;
        }
        "NiMaterialProperty" => {
            object(bytes)?;
            bytes.skip(56)?;
        }
        "NiTexturingProperty" => {
            texturing(bytes, format)?;
        }
        "NiSourceTexture" => {
            source_file(bytes)?;
        }
        "NiVertexColorProperty" => {
            object(bytes)?;
            bytes.skip(2 + 8)?;
        }
        "NiAlphaProperty" => {
            object(bytes)?;
            bytes.skip(3)?;
        }
        "NiStencilProperty" => {
            object(bytes)?;
            bytes.skip(29)?;
        }
        "NiZBufferProperty" => {
            object(bytes)?;
            bytes.skip(6)?;
        }
        "NiSpecularProperty" | "NiWireframeProperty" | "NiDitherProperty" | "NiShadeProperty" => {
            object(bytes)?;
            bytes.skip(2)?;
        }
        "bhkCollisionObject" | "bhkSPCollisionObject" => {
            bytes.skip(10)?;
        }
        "bhkRigidBody" | "bhkRigidBodyT" => {
            // Shape, filter and world object info; entity info; body info.
            bytes.skip(28 + 4 + 196)?;
            let constraints = count32(bytes)?;
            skip_n(bytes, constraints, 4)?;
            bytes.u32()?; // body flags
        }
        "bhkMoppBvTreeShape" => {
            bytes.skip(4 + 12 + 4)?;
            let size = count32(bytes)?;
            bytes.skip(12 + 4)?;
            bytes.skip(size)?;
        }
        "bhkNiTriStripsShape" => {
            bytes.skip(4 + 4 + 20 + 4 + 16)?;
            let data = count32(bytes)?;
            skip_n(bytes, data, 4)?;
            let layers = count32(bytes)?;
            skip_n(bytes, layers, 4)?;
        }
        "bhkConvexVerticesShape" => {
            bytes.skip(4 + 4 + 12 + 12)?;
            let vertices = count32(bytes)?;
            skip_n(bytes, vertices, 16)?;
            let normals = count32(bytes)?;
            skip_n(bytes, normals, 16)?;
        }
        "bhkBoxShape" => bytes.skip(32)?,
        "bhkCapsuleShape" => bytes.skip(48)?,
        "bhkSphereShape" => bytes.skip(8)?,
        "bhkConvexTransformShape" | "bhkTransformShape" => bytes.skip(84)?,
        "bhkListShape" => {
            let shapes = count32(bytes)?;
            skip_n(bytes, shapes, 4)?;
            bytes.skip(4 + 12 + 12)?;
            let unknown = count32(bytes)?;
            skip_n(bytes, unknown, 4)?;
        }
        "bhkLimitedHingeConstraint" | "bhkRagdollConstraint" | "bhkHingeConstraint" => {
            let entities = count32(bytes)?;
            skip_n(bytes, entities, 4)?;
            bytes.u32()?; // priority
            // Pivots and axes (four numbers each), then limits.
            bytes.skip(match kind {
                "bhkLimitedHingeConstraint" => 7 * 16 + 3 * 4,
                "bhkRagdollConstraint" => 6 * 16 + 6 * 4,
                _ => 5 * 16,
            })?;
        }
        "NiSkinInstance" => {
            bytes.skip(12)?;
            let bones = count32(bytes)?;
            skip_n(bytes, bones, 4)?;
        }
        "NiSkinData" => {
            bytes.skip(52)?;
            let bones = count32(bytes)?;
            let weighted = bytes.u8()? != 0;
            for _ in 0..bones {
                bytes.skip(52 + 16)?;
                let vertices = usize::from(bytes.u16()?);
                if weighted {
                    skip_n(bytes, vertices, 6)?;
                }
            }
        }
        "NiSkinPartition" => {
            let partitions = count32(bytes)?;
            for _ in 0..partitions {
                skin_partition(bytes)?;
            }
        }
        "bhkPackedNiTriStripsShape" => {
            let parts = usize::from(bytes.u16()?);
            skip_n(bytes, parts, 12)?; // filter, vertex count, material
            // User data, unused, radius, unused, scale, both again, data.
            bytes.skip(4 + 4 + 4 + 4 + 16 + 4 + 16 + 4)?;
        }
        "hkPackedNiTriStripsData" => {
            let triangles = count32(bytes)?;
            skip_n(bytes, triangles, 6 + 2 + 12)?; // corners, welding, normal
            let vertices = count32(bytes)?;
            skip_n(bytes, vertices, 12)?;
        }
        "bhkSimpleShapePhantom" => bytes.skip(28 + 8 + 64)?,
        "bhkMultiSphereShape" => {
            bytes.skip(4 + 4 + 12)?;
            let spheres = count32(bytes)?;
            skip_n(bytes, spheres, 16)?;
        }
        "bhkPrismaticConstraint" | "bhkStiffSpringConstraint" => {
            let entities = count32(bytes)?;
            skip_n(bytes, entities, 4)?;
            bytes.u32()?; // priority
            bytes.skip(if kind == "bhkPrismaticConstraint" {
                8 * 16 + 3 * 4
            } else {
                2 * 16 + 4
            })?;
        }
        "BSBound" => {
            text(bytes)?;
            bytes.skip(12 + 12)?; // centre, half size
        }
        "NiFogProperty" => {
            object(bytes)?;
            bytes.skip(2 + 4 + 12)?;
        }
        "NiAmbientLight" | "NiDirectionalLight" | "NiPointLight" | "NiSpotLight" => {
            av_object(bytes)?;
            bytes.u8()?; // switched on
            let nodes = count32(bytes)?;
            skip_n(bytes, nodes, 4)?;
            bytes.skip(4 + 3 * 12)?; // dimmer, colours
            match kind {
                "NiPointLight" => bytes.skip(12)?,
                "NiSpotLight" => bytes.skip(12 + 8)?,
                _ => {}
            }
        }
        _ => skip_animation_block(kind, bytes)?,
    }
    Some(())
}

/// Skips one block that names the animations of a mesh and what they move.
/// `None` if the type is not known or the block does not fit.
fn skip_sequence_block(kind: &str, bytes: &mut Bytes<'_>) -> Option<()> {
    match kind {
        "NiControllerManager" => {
            time_controller(bytes)?;
            bytes.u8()?; // cumulative
            let sequences = count32(bytes)?;
            skip_n(bytes, sequences, 4)?;
            bytes.u32()?; // object palette
        }
        "NiControllerSequence" => {
            text(bytes)?;
            let controlled = count32(bytes)?;
            bytes.u32()?; // array grow by
            // Interpolator, controller, priority, string palette and five
            // offsets into it.
            skip_n(bytes, controlled, 4 + 4 + 1 + 4 + 5 * 4)?;
            // Weight, text keys, cycle, frequency, start, stop, manager.
            bytes.skip(7 * 4)?;
            text(bytes)?; // accumulation root
            bytes.u32()?; // string palette
        }
        "NiDefaultAVObjectPalette" => {
            bytes.u32()?; // scene
            let objects = count32(bytes)?;
            for _ in 0..objects {
                text(bytes)?;
                bytes.u32()?;
            }
        }
        "NiStringPalette" => {
            text(bytes)?;
            bytes.u32()?; // length again
        }
        _ => return None,
    }
    Some(())
}

/// Skips one block that animates a mesh: its controllers, interpolators
/// and keys. `None` if the type is not known or the block does not fit.
fn skip_animation_block(kind: &str, bytes: &mut Bytes<'_>) -> Option<()> {
    match kind {
        "NiTransformInterpolator" => bytes.skip(12 + 16 + 4 + 4)?,
        "NiPoint3Interpolator" => bytes.skip(12 + 4)?,
        "NiFloatInterpolator" => bytes.skip(4 + 4)?,
        "NiBoolInterpolator" => bytes.skip(1 + 4)?,
        "NiTransformData" | "NiKeyframeData" => {
            let rotations = count32(bytes)?;
            if rotations > 0 {
                match bytes.u32()? {
                    4 => {
                        // Separate keys for each axis.
                        for _ in 0..3 {
                            key_group(bytes, 4)?;
                        }
                    }
                    1 | 2 | 5 => skip_n(bytes, rotations, 4 + 16)?,
                    3 => skip_n(bytes, rotations, 4 + 16 + 12)?,
                    _ => return None,
                }
            }
            key_group(bytes, 12)?; // translations
            key_group(bytes, 4)?; // scales
        }
        "NiFloatData" => key_group(bytes, 4)?,
        "NiPosData" => key_group(bytes, 12)?,
        "NiBoolData" => key_group(bytes, 1)?,
        "NiTransformController"
        | "NiKeyframeController"
        | "NiAlphaController"
        | "NiVisController"
        | "bhkBlendController" => {
            time_controller(bytes)?;
            bytes.u32()?; // interpolator, or keys
        }
        "NiMaterialColorController" => {
            time_controller(bytes)?;
            bytes.skip(4 + 2)?; // interpolator, which colour
        }
        "NiTextureTransformController" => {
            time_controller(bytes)?;
            bytes.skip(4 + 1 + 4 + 4)?; // interpolator, slot, operation
        }
        "NiFlipController" => {
            time_controller(bytes)?;
            bytes.skip(4 + 4)?; // interpolator, slot
            let sources = count32(bytes)?;
            skip_n(bytes, sources, 4)?;
        }
        "NiGeomMorpherController" => {
            time_controller(bytes)?;
            bytes.skip(2 + 4 + 1)?; // flags, data, always update
            let interpolators = count32(bytes)?;
            skip_n(bytes, interpolators, 4)?;
            let unknown = count32(bytes)?;
            skip_n(bytes, unknown, 4)?;
        }
        "NiMorphData" => {
            let morphs = count32(bytes)?;
            let vertices = count32(bytes)?;
            bytes.u8()?; // relative targets
            for _ in 0..morphs {
                text(bytes)?;
                skip_n(bytes, vertices, 12)?;
            }
        }
        "NiMultiTargetTransformController" => {
            time_controller(bytes)?;
            let targets = usize::from(bytes.u16()?);
            skip_n(bytes, targets, 4)?;
        }
        "NiBlendTransformInterpolator" => blend_interpolator(bytes)?,
        "NiBlendPoint3Interpolator" => {
            blend_interpolator(bytes)?;
            bytes.skip(12)?;
        }
        "NiBlendFloatInterpolator" => {
            blend_interpolator(bytes)?;
            bytes.skip(4)?;
        }
        "NiBlendBoolInterpolator" => {
            blend_interpolator(bytes)?;
            bytes.u8()?;
        }
        "NiBSBoneLODController" | "NiBoneLODController" => {
            time_controller(bytes)?;
            bytes.u32()?; // level shown
            let levels = count32(bytes)?;
            bytes.u32()?; // node groups, the same number
            for _ in 0..levels {
                let nodes = count32(bytes)?;
                skip_n(bytes, nodes, 4)?;
            }
        }
        _ => skip_sequence_block(kind, bytes)?,
    }
    Some(())
}

/// Skips what every animation controller starts with: the next
/// controller, flags, frequency, phase, start, stop and target.
fn time_controller(bytes: &mut Bytes<'_>) -> Option<()> {
    bytes.skip(4 + 2 + 4 * 4 + 4)
}

/// Skips what every interpolator that blends others starts with.
fn blend_interpolator(bytes: &mut Bytes<'_>) -> Option<()> {
    let flags = bytes.u8()?;
    let items = usize::from(bytes.u8()?);
    bytes.f32()?; // weight threshold
    if flags & 1 == 0 {
        // Counts, priorities, time, weights; then interpolator, weight,
        // normalised weight, priority and ease spinner for each.
        bytes.skip(4 + 4 * 4)?;
        skip_n(bytes, items, 4 + 4 + 4 + 1 + 4)?;
    }
    Some(())
}

/// Skips a group of animation keys whose values take `size` bytes each.
fn key_group(bytes: &mut Bytes<'_>, size: usize) -> Option<()> {
    let keys = count32(bytes)?;
    if keys == 0 {
        return Some(());
    }
    let each = match bytes.u32()? {
        // Linear and constant: time and value.
        1 | 5 => 4 + size,
        // Quadratic: time, value and two tangents.
        2 => 4 + 3 * size,
        // Tension, bias, continuity.
        3 => 4 + size + 12,
        _ => return None,
    };
    skip_n(bytes, keys, each)
}

/// Skips one partition of a skin.
fn skin_partition(bytes: &mut Bytes<'_>) -> Option<()> {
    let vertices = usize::from(bytes.u16()?);
    let triangles = usize::from(bytes.u16()?);
    let bones = usize::from(bytes.u16()?);
    let strips = usize::from(bytes.u16()?);
    let weights = usize::from(bytes.u16()?);
    skip_n(bytes, bones, 2)?;
    if bytes.u8()? != 0 {
        skip_n(bytes, vertices, 2)?;
    }
    if bytes.u8()? != 0 {
        skip_n(bytes, vertices.checked_mul(weights)?, 4)?;
    }
    let mut lengths = Vec::new();
    for _ in 0..strips {
        lengths.push(usize::from(bytes.u16()?));
    }
    if bytes.u8()? != 0 {
        if strips == 0 {
            skip_n(bytes, triangles, 6)?;
        } else {
            skip_n(bytes, lengths.iter().sum(), 2)?;
        }
    }
    if bytes.u8()? != 0 {
        skip_n(bytes, vertices.checked_mul(weights)?, 1)?;
    }
    Some(())
}

/// Where the blocks of an Oblivion mesh lie, and whether every block was
/// found: then the mesh ends right after them, with the list of its roots.
///
/// Blocks of types this reader does not know are measured by where the
/// next block of a known type starts: the place from which every block
/// after it can be read and the mesh ends exactly after its roots. Of a run
/// of such blocks, the first takes up the space and the others none. Where
/// no such place is found, the blocks up to the first that cannot be read
/// are returned.
pub(crate) fn layout(data: &[u8]) -> Result<(Vec<BlockInfo>, bool), NifError> {
    let header = header(data)?;
    let mut search = Search {
        data,
        format: header.format,
        kinds: &header.kinds,
        known: header
            .kinds
            .iter()
            .map(|kind| is_known(kind, header.format))
            .collect(),
        tries: MAX_TRIES,
        failed: HashSet::new(),
    };
    if let Some(spans) = search.spans_from(0, header.blocks_start) {
        let blocks = header
            .kinds
            .into_iter()
            .zip(spans)
            .map(|(kind, (start, size))| BlockInfo { kind, start, size })
            .collect();
        return Ok((blocks, true));
    }
    let mut bytes = Bytes::new(data);
    bytes.skip(header.blocks_start).ok_or(NifError::Damaged)?;
    let mut blocks = Vec::new();
    for kind in header.kinds {
        if header.format.has_block_prefix() && bytes.u32().is_none() {
            return Ok((blocks, false));
        }
        let start = data.len() - bytes.remaining();
        if skip_block(&kind, header.format, &mut bytes).is_none() {
            return Ok((blocks, false));
        }
        let size = data.len() - bytes.remaining() - start;
        blocks.push(BlockInfo { kind, start, size });
    }
    // The footer: the roots.
    let complete = bytes
        .u32()
        .and_then(|roots| skip_n(&mut bytes, usize::try_from(roots).ok()?, 4))
        .is_some()
        && bytes.remaining() == 0;
    Ok((blocks, complete))
}

/// How many places one mesh may try for blocks of unknown types, all runs
/// together, before the search gives up.
const MAX_TRIES: usize = 300_000;

/// The search for the places of blocks of unknown types; see [`layout`].
struct Search<'a> {
    data: &'a [u8],
    format: Format,
    kinds: &'a [String],
    /// Whether the layout of each block's type is known.
    known: Vec<bool>,
    /// How many places may still be tried.
    tries: usize,
    /// Pairs of a block and a place from which the mesh cannot be read to
    /// its end.
    failed: HashSet<(usize, usize)>,
}

impl Search<'_> {
    /// The start and size of each block from the block `first` on, that
    /// block starting at `start`, such that the mesh ends right after them
    /// with its roots. `None` if there is no such layout or the tries run
    /// out.
    fn spans_from(&mut self, first: usize, start: usize) -> Option<Vec<(usize, usize)>> {
        let data = self.data;
        let mut bytes = Bytes::new(data.get(start..)?);
        let position = |bytes: &Bytes<'_>| data.len() - bytes.remaining();
        let mut spans = Vec::new();
        for (index, kind) in self.kinds.iter().enumerate().skip(first) {
            self.tries = self.tries.checked_sub(1)?;
            if self.format.has_block_prefix() {
                bytes.u32()?;
            }
            let from = position(&bytes);
            if skip_block(kind, self.format, &mut bytes).is_some() {
                spans.push((from, position(&bytes) - from));
                continue;
            }
            if self.known.get(index).copied().unwrap_or(true) {
                // A known block that does not fit: the place is wrong.
                return None;
            }
            let next = self
                .known
                .iter()
                .enumerate()
                .skip(index)
                .find(|(_, known)| **known)
                .map_or(self.kinds.len(), |(next, _)| next);
            for candidate in from + 1..=data.len() {
                self.tries = self.tries.checked_sub(1)?;
                if self.failed.contains(&(next, candidate)) {
                    continue;
                }
                if let Some(found) = self.spans_from(next, candidate) {
                    spans.push((from, candidate - from));
                    spans.extend((index + 1..next).map(|_| (candidate, 0)));
                    spans.extend(found);
                    return Some(spans);
                }
                self.failed.insert((next, candidate));
            }
            return None;
        }
        let roots = count32(&mut bytes)?;
        skip_n(&mut bytes, roots, 4)?;
        (bytes.remaining() == 0).then_some(spans)
    }
}

/// Whether this reader knows the layout of blocks of the type `kind` in
/// `format`. Every known type can be read from zeros, as a block without
/// name, entries or keys.
fn is_known(kind: &str, format: Format) -> bool {
    const ZEROS: [u8; 512] = [0; 512];
    skip_block(kind, format, &mut Bytes::new(&ZEROS)).is_some()
}

/// A reference to another block: its number, or `None` for "no block".
fn reference(value: u32) -> Option<usize> {
    (value != u32::MAX)
        .then(|| usize::try_from(value).ok())
        .flatten()
}

/// The texture paths an Oblivion mesh names, in its own spelling, without
/// duplicates. Blocks after the first one of an unknown type are not read.
pub(crate) fn texture_paths(data: &[u8]) -> Result<Vec<String>, NifError> {
    let (blocks, _) = layout(data)?;
    let mut paths: Vec<String> = Vec::new();
    for block in blocks
        .iter()
        .filter(|block| block.kind == "NiSourceTexture")
    {
        let Some(slice) = data.get(block.start..block.start + block.size) else {
            continue;
        };
        if let Some(path) = source_file(&mut Bytes::new(slice))
            && !path.is_empty()
            && !paths.iter().any(|known| known.eq_ignore_ascii_case(&path))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// The shapes of an Oblivion mesh, placed by the nodes above them. Shapes
/// in blocks that could not be reached are left out.
pub(crate) fn shapes(data: &[u8]) -> Result<Vec<Shape>, NifError> {
    let (blocks, _) = layout(data)?;
    let format = header(data)?.format;
    let block = |index: usize| -> Option<(&str, &[u8])> {
        let info = blocks.get(index)?;
        Some((
            info.kind.as_str(),
            data.get(info.start..info.start + info.size)?,
        ))
    };
    let texture_of = |properties: &[usize]| -> (Option<String>, bool) {
        let mut texture = None;
        let mut alpha = false;
        for &property in properties {
            match block(property) {
                Some(("NiTexturingProperty", bytes)) => {
                    texture = texturing(&mut Bytes::new(bytes), format)
                        .and_then(reference)
                        .and_then(block)
                        .filter(|(kind, _)| *kind == "NiSourceTexture")
                        .and_then(|(_, source)| source_file(&mut Bytes::new(source)))
                        .filter(|name| !name.is_empty());
                }
                Some(("NiAlphaProperty", _)) => alpha = true,
                _ => {}
            }
        }
        (texture, alpha)
    };

    let mut shapes = Vec::new();
    let mut seen: HashSet<usize> = HashSet::new();
    let mut pending = vec![(0usize, Transform::IDENTITY, 0u32)];
    while let Some((index, parent, depth)) = pending.pop() {
        if depth > 64 || !seen.insert(index) {
            continue;
        }
        let Some((kind, bytes)) = block(index) else {
            continue;
        };
        let mut reader = Bytes::new(bytes);
        let Some((local, properties)) = av_object(&mut reader) else {
            continue;
        };
        let here = if index == 0 {
            Transform::IDENTITY
        } else {
            local.within(&parent)
        };
        match kind {
            "NiTriShape" | "NiTriStrips" => {
                let Some((data_kind, data)) = reader.u32().and_then(reference).and_then(block)
                else {
                    continue;
                };
                let mut data = Bytes::new(data);
                let Some(geometry) = geometry_data(&mut data, format) else {
                    continue;
                };
                let indices = match data_kind {
                    "NiTriShapeData" => {
                        triangle_corners(&mut data).map(|raw| corners(raw).collect())
                    }
                    "NiTriStripsData" => {
                        strip_corners(&mut data).map(|(lengths, raw)| strips(&lengths, raw))
                    }
                    _ => None,
                };
                let Some(indices) = indices else { continue };
                let positions = floats(geometry.positions);
                if let Some(shape) = finish(&positions, floats(geometry.uvs), &indices, here) {
                    let (texture, alpha) = texture_of(&properties);
                    shapes.push(Shape {
                        texture,
                        alpha,
                        ..shape
                    });
                }
            }
            _ if kind.ends_with("Node") => {
                let Some(children) = count32(&mut reader) else {
                    continue;
                };
                for _ in 0..children {
                    if let Some(child) = reader.u32().and_then(reference) {
                        pending.push((child, here, depth + 1));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(shapes)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn text_of(out: &mut Vec<u8>, value: &str) {
        out.extend(u32::try_from(value.len()).unwrap().to_le_bytes());
        out.extend(value.as_bytes());
    }

    fn refs(out: &mut Vec<u8>, values: &[i32]) {
        out.extend(u32::try_from(values.len()).unwrap().to_le_bytes());
        for value in values {
            out.extend(value.to_le_bytes());
        }
    }

    fn floats(out: &mut Vec<u8>, values: &[f32]) {
        for value in values {
            out.extend(value.to_le_bytes());
        }
    }

    fn object(out: &mut Vec<u8>) {
        text_of(out, "");
        refs(out, &[]);
        out.extend((-1i32).to_le_bytes()); // controller
    }

    fn av_object(translation: [f32; 3], properties: &[i32]) -> Vec<u8> {
        let mut out = Vec::new();
        object(&mut out);
        out.extend(0u16.to_le_bytes()); // flags
        floats(&mut out, &translation);
        floats(
            &mut out,
            &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0],
        );
        refs(&mut out, properties);
        out.extend((-1i32).to_le_bytes()); // collision
        out
    }

    fn node(children: &[i32]) -> Vec<u8> {
        let mut out = av_object([0.0; 3], &[]);
        refs(&mut out, children);
        refs(&mut out, &[]);
        out
    }

    fn shape(data: i32, properties: &[i32]) -> Vec<u8> {
        let mut out = av_object([0.0, 0.0, 10.0], properties);
        out.extend(data.to_le_bytes());
        out.extend((-1i32).to_le_bytes()); // skin
        out.push(0); // no shader
        out
    }

    /// Geometry data of `count` points along x, with one set of texture
    /// coordinates.
    fn geometry(count: u16) -> Vec<u8> {
        geometry_in(count, Format::NEWEST)
    }

    fn geometry_in(count: u16, format: Format) -> Vec<u8> {
        let mut out = Vec::new();
        if format.has_geometry_group() {
            out.extend(0u32.to_le_bytes());
        }
        out.extend(count.to_le_bytes());
        out.extend([0, 0, 1]);
        for index in 0..count {
            floats(&mut out, &[f32::from(index), f32::from(index % 2), 0.0]);
        }
        out.extend(1u16.to_le_bytes()); // one set of texture coordinates
        out.push(0); // no normals
        floats(&mut out, &[0.0; 4]);
        out.push(0); // no colours
        for _ in 0..count {
            floats(&mut out, &[0.5, 0.5]);
        }
        out.extend(0u16.to_le_bytes()); // consistency
        if format.has_additional_data() {
            out.extend((-1i32).to_le_bytes());
        }
        out
    }

    fn triangle_data() -> Vec<u8> {
        triangle_data_in(Format::NEWEST)
    }

    fn triangle_data_in(format: Format) -> Vec<u8> {
        let mut out = geometry_in(3, format);
        out.extend(1u16.to_le_bytes());
        out.extend(3u32.to_le_bytes());
        out.push(1);
        for index in [0u16, 1, 2] {
            out.extend(index.to_le_bytes());
        }
        out.extend(0u16.to_le_bytes()); // match groups
        out
    }

    fn strip_data() -> Vec<u8> {
        let mut out = geometry(4);
        out.extend(2u16.to_le_bytes()); // triangles
        out.extend(1u16.to_le_bytes()); // one strip
        out.extend(4u16.to_le_bytes());
        out.push(1);
        for index in [0u16, 1, 2, 3] {
            out.extend(index.to_le_bytes());
        }
        out
    }

    fn texturing(source: i32) -> Vec<u8> {
        texturing_in(source, Format::NEWEST)
    }

    fn texturing_in(source: i32, format: Format) -> Vec<u8> {
        let mut out = Vec::new();
        object(&mut out);
        out.extend(2u32.to_le_bytes()); // apply mode
        out.extend(7u32.to_le_bytes()); // texture count
        out.push(1); // base texture
        out.extend(source.to_le_bytes());
        out.extend([0u8; 12]);
        if format.has_ps2_numbers() {
            out.extend([0u8; 4]);
        }
        out.push(0); // no transform
        out.extend([0u8; 5]); // dark, detail, gloss, glow, bump
        out.push(0); // first decal
        out.extend(0u32.to_le_bytes()); // shader textures
        out
    }

    fn source(path: &str) -> Vec<u8> {
        let mut out = Vec::new();
        object(&mut out);
        out.push(1);
        text_of(&mut out, path);
        out.extend((-1i32).to_le_bytes());
        out.extend([0u8; 14]);
        out
    }

    /// An Oblivion mesh with the given blocks and its root at block 0.
    fn mesh(blocks: &[(&str, Vec<u8>)]) -> Vec<u8> {
        mesh_in(
            Format {
                version: 0x1400_0004,
            },
            blocks,
        )
    }

    /// A mesh in `format` with the given blocks and its root at block 0.
    fn mesh_in(format: Format, blocks: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let name = match format.version {
            0x0A01_006A => "10.1.0.106",
            0x0A02_0000 => "10.2.0.0",
            _ => "20.0.0.4",
        };
        let mut out = format!("Gamebryo File Format, Version {name}\n").into_bytes();
        out.extend(format.version.to_le_bytes());
        if format.version >= BYTE_ORDER_SINCE {
            out.push(1);
        }
        out.extend(11u32.to_le_bytes());
        out.extend(u32::try_from(blocks.len()).unwrap().to_le_bytes());
        out.extend(11u32.to_le_bytes());
        out.extend([1, 0, 1, 0, 1, 0]);
        let mut types: Vec<&str> = Vec::new();
        for (kind, _) in blocks {
            if !types.contains(kind) {
                types.push(kind);
            }
        }
        out.extend(u16::try_from(types.len()).unwrap().to_le_bytes());
        for kind in &types {
            text_of(&mut out, kind);
        }
        for (kind, _) in blocks {
            let index = types.iter().position(|known| known == kind).unwrap();
            out.extend(u16::try_from(index).unwrap().to_le_bytes());
        }
        out.extend(0u32.to_le_bytes()); // groups
        for (_, data) in blocks {
            if format.has_block_prefix() {
                out.extend(0u32.to_le_bytes());
            }
            out.extend(data);
        }
        refs(&mut out, &[0]);
        out
    }

    fn table() -> Vec<u8> {
        mesh(&[
            ("NiNode", node(&[1])),
            ("NiTriShape", shape(2, &[3])),
            ("NiTriShapeData", triangle_data()),
            ("NiTexturingProperty", texturing(4)),
            ("NiSourceTexture", source(r"textures\furniture\table.dds")),
        ])
    }

    #[test]
    fn the_older_formats_oblivion_loads_are_read_as_well() {
        for version in [0x0A01_006A, 0x0A02_0000] {
            let format = Format { version };
            let data = mesh_in(
                format,
                &[
                    ("NiNode", node(&[1])),
                    ("NiTriShape", shape(2, &[3])),
                    ("NiTriShapeData", triangle_data_in(format)),
                    ("NiTexturingProperty", texturing_in(4, format)),
                    ("NiSourceTexture", source(r"textures\rocks\rock.dds")),
                ],
            );
            assert!(has_oblivion_version(&data), "{version:x}");
            let (blocks, complete) = layout(&data).unwrap();
            assert!(complete, "{version:x}");
            assert_eq!(blocks.len(), 5);
            let found = shapes(&data).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].indices, [0, 1, 2]);
            assert_eq!(
                found[0].texture.as_deref(),
                Some(r"textures\rocks\rock.dds")
            );
        }
    }

    #[test]
    fn every_block_of_a_mesh_is_found() {
        let (blocks, complete) = layout(&table()).unwrap();
        assert!(complete);
        assert_eq!(blocks.len(), 5);
        assert!(has_oblivion_version(&table()));
    }

    #[test]
    fn a_shape_is_placed_and_textured() {
        let found = shapes(&table()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].indices, [0, 1, 2]);
        assert_eq!(
            found[0].positions,
            [0.0, 0.0, 10.0, 1.0, 1.0, 10.0, 2.0, 0.0, 10.0]
        );
        assert_eq!(found[0].uvs, [0.5; 6]);
        assert_eq!(
            found[0].texture.as_deref(),
            Some(r"textures\furniture\table.dds")
        );
        assert_eq!(
            texture_paths(&table()).unwrap(),
            [r"textures\furniture\table.dds"]
        );
    }

    #[test]
    fn strips_become_triangles() {
        let data = mesh(&[
            ("NiNode", node(&[1])),
            ("NiTriStrips", shape(2, &[])),
            ("NiTriStripsData", strip_data()),
        ]);
        let found = shapes(&data).unwrap();
        assert_eq!(found[0].indices, [0, 1, 2, 1, 3, 2]);
        assert_eq!(found[0].texture, None);
    }

    #[test]
    fn a_block_of_an_unknown_type_is_measured_by_the_next_one() {
        let data = mesh(&[
            ("NiNode", node(&[2])),
            ("NiSomethingNew", vec![7, 7, 7, 7, 7]),
            ("NiTriShape", shape(3, &[])),
            ("NiTriShapeData", triangle_data()),
            ("NiSomethingElse", vec![1, 2, 3]),
        ]);
        let (blocks, complete) = layout(&data).unwrap();
        assert!(complete);
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[1].size, 5);
        assert_eq!(blocks[2].start, blocks[1].start + 5);
        assert_eq!(blocks[4].size, 3);
        assert_eq!(shapes(&data).unwrap().len(), 1);
    }

    #[test]
    fn of_a_run_of_unknown_blocks_the_first_takes_the_space() {
        let data = mesh(&[
            ("NiNode", node(&[3])),
            ("NiSomethingNew", vec![7, 7, 7]),
            ("NiSomethingElse", vec![8, 8]),
            ("NiTriShape", shape(4, &[])),
            ("NiTriShapeData", triangle_data()),
        ]);
        let (blocks, complete) = layout(&data).unwrap();
        assert!(complete);
        assert_eq!(blocks[1].size, 5);
        assert_eq!(blocks[2].size, 0);
        assert_eq!(shapes(&data).unwrap().len(), 1);
    }

    #[test]
    fn blocks_before_an_unknown_one_are_used_where_no_place_fits() {
        let mut data = mesh(&[
            ("NiNode", node(&[1])),
            ("NiTriShape", shape(2, &[])),
            ("NiTriShapeData", triangle_data()),
            ("NiSomethingNew", vec![1, 2, 3]),
            ("NiNode", node(&[])),
        ]);
        // The mesh ends too early for the last node.
        data.truncate(data.len() - 12);
        let (blocks, complete) = layout(&data).unwrap();
        assert!(!complete);
        assert_eq!(blocks.len(), 3);
        assert_eq!(shapes(&data).unwrap().len(), 1);
    }

    #[test]
    fn known_types_are_told_from_unknown_ones() {
        for kind in [
            "NiNode",
            "NiTriStripsData",
            "NiTexturingProperty",
            "NiControllerSequence",
            "NiTransformData",
            "bhkPackedNiTriStripsShape",
        ] {
            assert!(is_known(kind, Format::NEWEST), "{kind}");
        }
        assert!(!is_known("NiSomethingNew", Format::NEWEST));
        // Animations are laid out otherwise before 10.2.0.0.
        let old = Format {
            version: 0x0A01_006A,
        };
        assert!(!is_known("NiControllerSequence", old));
        assert!(is_known("NiNode", old));
    }

    #[test]
    fn other_versions_are_not_oblivions() {
        let mut data = table();
        let after_line = data.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        data[after_line..after_line + 4].copy_from_slice(&0x1402_0007u32.to_le_bytes());
        assert!(!has_oblivion_version(&data));
        assert!(matches!(layout(&data), Err(NifError::UnsupportedVersion)));
        assert!(!has_oblivion_version(b"not a mesh"));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = shapes(&data);
            let _ = texture_paths(&data);
        }

        #[test]
        fn never_panics_on_a_damaged_mesh(cut in 0usize..1200, at in 0usize..1200, value in any::<u8>()) {
            let mut data = table();
            if let Some(byte) = data.get_mut(at) {
                *byte = value;
            }
            data.truncate(cut);
            let _ = shapes(&data);
            let _ = texture_paths(&data);
        }
    }
}
