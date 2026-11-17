//! Reads the meshes of Oblivion (format versions 20.0.0.4 and 20.0.0.5).
//!
//! Unlike the later games' meshes, these do not say how long each block
//! is. A block can only be found by reading every block before it, so this
//! reader knows the layout of the block types the game's own meshes use
//! (checked against all of them, see [`layout`]). Where it meets a block
//! type it does not know, it stops; what lies before is still used.
//!
//! Textures are named by `NiSourceTexture` blocks, which hang from a
//! shape's `NiTexturingProperty`. Shapes are `NiTriShape` (triangles) and
//! `NiTriStrips` (strips of triangles).

use std::collections::HashSet;

use crate::nif::{BlockInfo, NifError};
use crate::nif_geometry::{Shape, Transform, finish, read_rotation};
use crate::{Bytes, decode_text};

/// The versions of the format Oblivion uses.
const VERSIONS: [u32; 2] = [0x1400_0004, 0x1400_0005];

/// The longest text inside a mesh that is read.
const MAX_TEXT: usize = 64 * 1024;

/// Most entries a list in a block is believed to have.
const MAX_LIST: usize = 1 << 20;

/// Whether `data` starts like a mesh in one of Oblivion's versions. Only
/// the first line and the version numbers are looked at.
pub(crate) fn has_oblivion_version(data: &[u8]) -> bool {
    let Some(line_len) = data.iter().take(64).position(|&byte| byte == b'\n') else {
        return false;
    };
    let mut bytes = Bytes::new(data);
    let mut versions = || -> Option<(u32, u8)> {
        let line = bytes.take(line_len + 1)?;
        if !line.starts_with(b"Gamebryo File Format, Version ") {
            return None;
        }
        Some((bytes.u32()?, bytes.u8()?))
    };
    matches!(versions(), Some((version, 1)) if VERSIONS.contains(&version))
}

/// What the header says: the types of the blocks, and where they start.
struct Header {
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
    let line = bytes.take(line_len + 1).ok_or(NifError::NotAMesh)?;
    if !line.starts_with(b"Gamebryo File Format, Version ") {
        return Err(NifError::NotAMesh);
    }
    let version = bytes.u32().ok_or_else(damaged)?;
    let little_endian = bytes.u8().ok_or_else(damaged)? == 1;
    let user_version = bytes.u32().ok_or_else(damaged)?;
    if !VERSIONS.contains(&version) || !little_endian || !(10..=11).contains(&user_version) {
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
fn texture_slot(bytes: &mut Bytes<'_>) -> Option<u32> {
    let source = bytes.u32()?;
    bytes.skip(4 + 4 + 4)?; // clamp, filter, texture coordinate set
    if bytes.u8()? != 0 {
        bytes.skip(36)?; // texture transform
    }
    Some(source)
}

/// The texture in the first slot of a texturing property, the colour map,
/// as a reference as stored (`u32::MAX` for none).
fn texturing(bytes: &mut Bytes<'_>) -> Option<u32> {
    object(bytes)?;
    bytes.u32()?; // apply mode
    let count = bytes.u32()?;
    let mut base = u32::MAX;
    // Base, dark, detail, gloss, glow and bump map.
    for slot in 0..6 {
        if bytes.u8()? != 0 {
            let source = texture_slot(bytes)?;
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
            texture_slot(bytes)?;
        }
    }
    let shader_textures = count32(bytes)?;
    for _ in 0..shader_textures {
        if bytes.u8()? != 0 {
            texture_slot(bytes)?;
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

/// Geometry data: points, texture coordinates. Leaves `bytes` at the
/// part that differs between triangles and strips.
fn geometry_data(bytes: &mut Bytes<'_>) -> Option<(Vec<f32>, Vec<f32>)> {
    bytes.u32()?; // group
    let count = usize::from(bytes.u16()?);
    bytes.skip(2)?; // keep and compress flags
    let mut positions = Vec::new();
    if bytes.u8()? != 0 {
        for _ in 0..count * 3 {
            positions.push(bytes.f32()?);
        }
    }
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
    let mut uvs = Vec::new();
    for set in 0..(flags & 0x3F) {
        for _ in 0..count * 2 {
            let value = bytes.f32()?;
            if set == 0 {
                uvs.push(value);
            }
        }
    }
    bytes.u16()?; // consistency
    bytes.u32()?; // additional data
    Some((positions, uvs))
}

/// The triangles of `NiTriShapeData`, after its geometry data.
fn triangles(bytes: &mut Bytes<'_>) -> Option<Vec<u32>> {
    let count = usize::from(bytes.u16()?);
    bytes.u32()?; // number of triangle points
    let mut indices = Vec::new();
    if bytes.u8()? != 0 {
        for _ in 0..count * 3 {
            indices.push(u32::from(bytes.u16()?));
        }
    }
    let groups = bytes.u16()?;
    for _ in 0..groups {
        let size = usize::from(bytes.u16()?);
        skip_n(bytes, size, 2)?;
    }
    Some(indices)
}

/// The triangles of `NiTriStripsData`, after its geometry data: each strip
/// turned into triangles, every second one turned around.
fn strips(bytes: &mut Bytes<'_>) -> Option<Vec<u32>> {
    bytes.u16()?; // number of triangles
    let count = usize::from(bytes.u16()?);
    let mut lengths = Vec::new();
    for _ in 0..count {
        lengths.push(usize::from(bytes.u16()?));
    }
    let mut indices = Vec::new();
    if bytes.u8()? != 0 {
        for length in lengths {
            let mut strip = Vec::new();
            for _ in 0..length {
                strip.push(u32::from(bytes.u16()?));
            }
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
    }
    Some(indices)
}

/// Skips one block of the type `kind`. `None` if the type is not known or
/// the block does not fit the data.
#[allow(clippy::too_many_lines)]
fn skip_block(kind: &str, bytes: &mut Bytes<'_>) -> Option<()> {
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
            geometry_data(bytes)?;
            triangles(bytes)?;
        }
        "NiTriStripsData" => {
            geometry_data(bytes)?;
            strips(bytes)?;
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
            texturing(bytes)?;
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
        _ => return None,
    }
    Some(())
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

/// Where the blocks of an Oblivion mesh lie, as far as their types are
/// known, and whether every block was found: then the mesh ends right after
/// them, with the list of its roots.
pub(crate) fn layout(data: &[u8]) -> Result<(Vec<BlockInfo>, bool), NifError> {
    let header = header(data)?;
    let mut bytes = Bytes::new(data);
    bytes.skip(header.blocks_start).ok_or(NifError::Damaged)?;
    let mut blocks = Vec::new();
    for kind in header.kinds {
        let start = data.len() - bytes.remaining();
        if skip_block(&kind, &mut bytes).is_none() {
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
                    texture = texturing(&mut Bytes::new(bytes))
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
                let Some((positions, uvs)) = geometry_data(&mut data) else {
                    continue;
                };
                let indices = match data_kind {
                    "NiTriShapeData" => triangles(&mut data),
                    "NiTriStripsData" => strips(&mut data),
                    _ => None,
                };
                let Some(indices) = indices else { continue };
                if let Some(shape) = finish(&positions, uvs, &indices, here) {
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
        let mut out = 0u32.to_le_bytes().to_vec(); // group
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
        out.extend((-1i32).to_le_bytes()); // additional data
        out
    }

    fn triangle_data() -> Vec<u8> {
        let mut out = geometry(3);
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
        let mut out = Vec::new();
        object(&mut out);
        out.extend(2u32.to_le_bytes()); // apply mode
        out.extend(7u32.to_le_bytes()); // texture count
        out.push(1); // base texture
        out.extend(source.to_le_bytes());
        out.extend([0u8; 12]);
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
        let mut out = b"Gamebryo File Format, Version 20.0.0.4\n".to_vec();
        out.extend(0x1400_0004u32.to_le_bytes());
        out.push(1);
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
    fn blocks_before_an_unknown_one_are_still_used() {
        let data = mesh(&[
            ("NiNode", node(&[1])),
            ("NiTriShape", shape(2, &[])),
            ("NiTriShapeData", triangle_data()),
            ("NiSomethingNew", vec![1, 2, 3]),
        ]);
        let (blocks, complete) = layout(&data).unwrap();
        assert!(!complete);
        assert_eq!(blocks.len(), 3);
        assert_eq!(shapes(&data).unwrap().len(), 1);
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
