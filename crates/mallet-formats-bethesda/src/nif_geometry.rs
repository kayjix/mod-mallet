//! The shapes of a mesh, for drawing it: points, texture coordinates,
//! triangles and the colour texture of every surface.
//!
//! Three kinds of shape are read:
//!
//! * `NiTriShape` with `NiTriShapeData`: meshes in the format of the first
//!   Skyrim, which mods still ship and the game still loads.
//! * `BSTriShape` (and its relatives `BSMeshLODTriShape`,
//!   `BSSubIndexTriShape`): the format of Skyrim Special Edition, with all
//!   of a point's data packed together as the shape's vertex description
//!   says.
//! * `BSDynamicTriShape`, as in faces: the points are kept apart as
//!   floats, the triangles and texture coordinates in the shape's skin
//!   partition.
//!
//! Nodes place their children; their transforms are applied, so the shapes
//! come out in the mesh's own space. Skinned shapes are taken as they are
//! stored, which for heads and most armour is the pose they were made in.
//! Everything else in a mesh (collision, animation, effects) is skipped.
//! Like everything that reads files of mods, nothing here trusts a size or
//! an index in the file (ADR 0013).

use std::collections::HashMap;

use crate::Bytes;
use crate::nif::{NifError, blocks};

/// One surface of a mesh, ready to draw.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    /// The points, three numbers each, in the mesh's space.
    pub positions: Vec<f32>,
    /// The texture coordinates, two numbers each, one pair per point;
    /// empty if the shape has none.
    pub uvs: Vec<f32>,
    /// The triangles, three point numbers each.
    pub indices: Vec<u32>,
    /// The colour texture, as the mesh names it, if it names one.
    pub texture: Option<String>,
    /// The material file (`.bgsm`, `.bgem`) the shape's shader names, as
    /// the mesh spells it. From Fallout 4 on, it often holds the textures
    /// the mesh does not name itself.
    pub material: Option<String>,
    /// Whether the shape has an alpha property: its texture's transparent
    /// parts, such as between strands of hair, are left out.
    pub alpha: bool,
}

/// A transform: rotation (row by row), translation, scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Transform {
    pub(crate) rotation: [[f32; 3]; 3],
    pub(crate) translation: [f32; 3],
    pub(crate) scale: f32,
}

fn dot(row: [f32; 3], column: [f32; 3]) -> f32 {
    row[0] * column[0] + row[1] * column[1] + row[2] * column[2]
}

impl Transform {
    pub(crate) const IDENTITY: Self = Self {
        rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0; 3],
        scale: 1.0,
    };

    fn apply(&self, point: [f32; 3]) -> [f32; 3] {
        let [first, second, third] = self.rotation;
        let [tx, ty, tz] = self.translation;
        [
            self.scale * dot(first, point) + tx,
            self.scale * dot(second, point) + ty,
            self.scale * dot(third, point) + tz,
        ]
    }

    /// `self` placed inside `parent`.
    pub(crate) fn within(&self, parent: &Self) -> Self {
        let [[xx, xy, xz], [yx, yy, yz], [zx, zy, zz]] = self.rotation;
        let columns = [[xx, yx, zx], [xy, yy, zy], [xz, yz, zz]];
        let rotation = parent
            .rotation
            .map(|row| columns.map(|column| dot(row, column)));
        Self {
            rotation,
            translation: parent.apply(self.translation),
            scale: parent.scale * self.scale,
        }
    }
}

/// Reads a rotation of nine numbers, row by row.
pub(crate) fn read_rotation(bytes: &mut Bytes<'_>) -> Option<[[f32; 3]; 3]> {
    let mut row = || Some([bytes.f32()?, bytes.f32()?, bytes.f32()?]);
    Some([row()?, row()?, row()?])
}

/// The parts of a block of the `NiAVObject` family the reader needs.
struct Placed {
    transform: Transform,
    /// Whether the block is hidden: the game does not draw it, nor
    /// anything below it.
    hidden: bool,
}

/// Reads name, extra data, controller, flags and transform, and the
/// collision reference: what every node and shape starts with.
fn read_av_object(bytes: &mut Bytes<'_>, bs_version: u32) -> Option<Placed> {
    bytes.u32()?; // name
    let extra = bytes.u32()?;
    for _ in 0..extra.min(4096) {
        bytes.u32()?;
    }
    bytes.u32()?; // controller
    let flags = if bs_version > 26 {
        bytes.u32()?
    } else {
        u32::from(bytes.u16()?)
    };
    let translation = [bytes.f32()?, bytes.f32()?, bytes.f32()?];
    let rotation = read_rotation(bytes)?;
    let scale = bytes.f32()?;
    if bs_version <= 34 {
        let properties = bytes.u32()?;
        for _ in 0..properties.min(4096) {
            bytes.u32()?;
        }
    }
    bytes.u32()?; // collision object
    let scale = if scale.is_finite() && scale != 0.0 {
        scale
    } else {
        1.0
    };
    Some(Placed {
        transform: Transform {
            rotation,
            translation,
            scale,
        },
        hidden: flags & 0x1 != 0,
    })
}

fn reference(value: u32) -> Option<usize> {
    let index = i32::from_le_bytes(value.to_le_bytes());
    usize::try_from(index).ok()
}

/// The place of a node, whether it is hidden, and its children.
fn node_children(block: &[u8], bs_version: u32) -> Option<(Placed, Vec<usize>)> {
    let mut bytes = Bytes::new(block);
    let placed = read_av_object(&mut bytes, bs_version)?;
    let count = bytes.u32()?;
    let mut children = Vec::new();
    for _ in 0..count.min(65_536) {
        if let Some(child) = reference(bytes.u32()?) {
            children.push(child);
        }
    }
    Some((placed, children))
}

/// A half-precision float.
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x3ff);
    match exponent {
        0 => sign * fraction * 2f32.powi(-24),
        31 => 0.0,
        _ => sign * (1.0 + fraction / 1024.0) * 2f32.powi(exponent - 15),
    }
}

/// What a `BSVertexDesc` says about the packed vertex data.
#[derive(Clone, Copy)]
struct VertexLayout {
    stride: usize,
    uv_offset: usize,
    has_position: bool,
    has_uv: bool,
    full_precision: bool,
}

impl VertexLayout {
    /// The layout `desc` describes, in a mesh of Bethesda version
    /// `bs_version`. Skyrim Special Edition keeps positions as full floats
    /// whatever the flag says; half precision came with later games.
    fn of(desc: u64, bs_version: u32) -> Self {
        let quads = |shift: u32| ((desc >> shift) & 0xF) as usize * 4;
        let attributes = desc >> 44;
        Self {
            stride: quads(0),
            uv_offset: quads(8),
            has_position: attributes & 0x1 != 0,
            has_uv: attributes & 0x2 != 0,
            full_precision: attributes & 0x400 != 0 || bs_version < 130,
        }
    }

    /// Positions and texture coordinates of `count` packed vertices.
    fn read(&self, data: &[u8], count: usize) -> (Vec<f32>, Vec<f32>) {
        let mut positions = Vec::with_capacity(count * 3);
        let mut uvs = Vec::with_capacity(count * 2);
        if self.stride == 0 {
            return (positions, uvs);
        }
        for vertex in data.chunks_exact(self.stride).take(count) {
            if self.has_position {
                let mut bytes = Bytes::new(vertex);
                for _ in 0..3 {
                    let value = if self.full_precision {
                        bytes.f32()
                    } else {
                        bytes.u16().map(half)
                    };
                    positions.push(value.unwrap_or(0.0));
                }
            }
            if self.has_uv {
                let mut bytes = Bytes::new(vertex.get(self.uv_offset..).unwrap_or_default());
                for _ in 0..2 {
                    uvs.push(bytes.u16().map_or(0.0, half));
                }
            }
        }
        (positions, uvs)
    }
}

/// A shape before its skin and shader are looked up.
struct RawShape {
    positions: Vec<f32>,
    uvs: Vec<f32>,
    indices: Vec<u32>,
    skin: Option<usize>,
    shader: Option<usize>,
    alpha: bool,
}

/// `BSTriShape` and its relatives; `dynamic` for `BSDynamicTriShape`.
fn read_bs_tri_shape(block: &[u8], bs_version: u32, dynamic: bool) -> Option<RawShape> {
    let mut bytes = Bytes::new(block);
    read_av_object(&mut bytes, bs_version)?;
    for _ in 0..4 {
        bytes.f32()?; // bounding sphere
    }
    let skin = reference(bytes.u32()?);
    let shader = reference(bytes.u32()?);
    let alpha = reference(bytes.u32()?).is_some();
    let desc = u64::from(bytes.u32()?) | (u64::from(bytes.u32()?) << 32);
    let triangles = if bs_version >= 130 {
        bytes.u32()? as usize
    } else {
        usize::from(bytes.u16()?)
    };
    let vertices = usize::from(bytes.u16()?);
    let data_size = bytes.u32()? as usize;
    let layout = VertexLayout::of(desc, bs_version);
    let (mut positions, mut uvs, mut indices) = (Vec::new(), Vec::new(), Vec::new());
    if data_size > 0 {
        let data = bytes.take(vertices.checked_mul(layout.stride)?)?;
        (positions, uvs) = layout.read(data, vertices);
        for _ in 0..triangles {
            for _ in 0..3 {
                indices.push(u32::from(bytes.u16()?));
            }
        }
    }
    if bs_version == 100 {
        // Particle data: half-precision points and normals, and triangles.
        if bytes.u32()? > 0 {
            bytes.skip(vertices * 12 + triangles * 6)?;
        }
    }
    if dynamic {
        let dynamic_size = bytes.u32()? as usize;
        let count = dynamic_size / 16;
        positions.clear();
        for _ in 0..count {
            positions.extend([bytes.f32()?, bytes.f32()?, bytes.f32()?]);
            bytes.f32()?;
        }
    }
    Some(RawShape {
        positions,
        uvs,
        indices,
        skin,
        shader,
        alpha,
    })
}

/// `NiTriShape`: its data and shader.
fn read_ni_tri_shape(
    block: &[u8],
    bs_version: u32,
) -> Option<(Transform, usize, Option<usize>, bool)> {
    let mut bytes = Bytes::new(block);
    let placed = read_av_object(&mut bytes, bs_version)?;
    if placed.hidden {
        // The game does not draw it.
        return None;
    }
    let data = reference(bytes.u32()?)?;
    bytes.u32()?; // skin instance
    let materials = bytes.u32()?;
    for _ in 0..materials.min(4096) {
        bytes.u32()?;
        bytes.u32()?;
    }
    bytes.u32()?; // active material
    bytes.u8()?; // dirty flag
    let (shader, alpha) = if bs_version > 34 {
        let shader = reference(bytes.u32()?);
        (shader, reference(bytes.u32()?).is_some())
    } else {
        (None, false)
    };
    Some((placed.transform, data, shader, alpha))
}

/// `NiTriShapeData`: points, texture coordinates and triangles.
fn read_ni_tri_shape_data(block: &[u8]) -> Option<(Vec<f32>, Vec<f32>, Vec<u32>)> {
    let mut bytes = Bytes::new(block);
    bytes.u32()?; // group id
    let count = usize::from(bytes.u16()?);
    bytes.u8()?; // keep flags
    bytes.u8()?; // compress flags
    let mut positions = Vec::new();
    if bytes.u8()? != 0 {
        for _ in 0..count * 3 {
            positions.push(bytes.f32()?);
        }
    }
    let vector_flags = bytes.u16()?;
    bytes.u32()?; // material checksum, in every Skyrim mesh
    if bytes.u8()? != 0 {
        bytes.skip(count * 12)?; // normals
        if vector_flags & 0x1000 != 0 {
            bytes.skip(count * 24)?; // tangents and bitangents
        }
    }
    bytes.skip(16)?; // centre and radius
    if bytes.u8()? != 0 {
        bytes.skip(count * 16)?; // colours
    }
    let mut uvs = Vec::new();
    if vector_flags & 0x3F != 0 {
        for _ in 0..count * 2 {
            uvs.push(bytes.f32()?);
        }
    }
    bytes.u16()?; // consistency
    bytes.u32()?; // additional data
    let triangles = usize::from(bytes.u16()?);
    bytes.u32()?; // number of triangle points
    let mut indices = Vec::new();
    if bytes.u8()? != 0 {
        for _ in 0..triangles * 3 {
            indices.push(u32::from(bytes.u16()?));
        }
    }
    Some((positions, uvs, indices))
}

/// The transform of the data of a skin: from the space its points are
/// stored in to the space of the skeleton.
fn skin_transform(data: &[u8]) -> Option<Transform> {
    let mut bytes = Bytes::new(data);
    let rotation = read_rotation(&mut bytes)?;
    let translation = [bytes.f32()?, bytes.f32()?, bytes.f32()?];
    let scale = bytes.f32()?;
    Some(Transform {
        rotation,
        translation,
        scale: if scale == 0.0 { 1.0 } else { scale },
    })
}

/// The bones a skin instance names, as block numbers.
fn instance_bones(instance: &[u8]) -> Vec<Option<usize>> {
    let mut bytes = Bytes::new(instance);
    let mut bones = Vec::new();
    let head = (|| {
        bytes.u32()?;
        bytes.u32()?;
        bytes.u32()?;
        bytes.u32()
    })();
    for _ in 0..head.unwrap_or(0).min(1024) {
        match bytes.u32() {
            Some(value) => bones.push(reference(value)),
            None => break,
        }
    }
    bones
}

/// From a skin's data, every bone's transform: from the space the points
/// are stored in to that bone's.
fn bone_transforms(data: &[u8]) -> Vec<Transform> {
    let mut bytes = Bytes::new(data);
    let mut found = Vec::new();
    let head = (|| {
        bytes.skip(13 * 4)?; // the skin's own transform
        let count = bytes.u32()?;
        let weighted = bytes.u8()? != 0;
        Some((count, weighted))
    })();
    let Some((count, weighted)) = head else {
        return found;
    };
    for _ in 0..count.min(1024) {
        let Some(transform) = bytes.take(13 * 4).and_then(skin_transform) else {
            break;
        };
        found.push(transform);
        let rest = (|| {
            bytes.skip(16)?; // bounding sphere
            let points = usize::from(bytes.u16()?);
            if weighted {
                bytes.skip(points * 6)?;
            }
            Some(())
        })();
        if rest.is_none() {
            break;
        }
    }
    found
}

/// The first bone a skin instance names, as block number.
fn first_bone(instance: &[u8]) -> Option<usize> {
    let mut bytes = Bytes::new(instance);
    bytes.u32()?; // skin data
    bytes.u32()?; // skin partition
    bytes.u32()?; // skeleton root
    if bytes.u32()? == 0 {
        return None;
    }
    reference(bytes.u32()?)
}

/// From a skin's data, the transform of its first bone: from the space the
/// points are stored in to that bone's.
fn first_bone_transform(data: &[u8]) -> Option<Transform> {
    let mut bytes = Bytes::new(data);
    bytes.skip(13 * 4)?; // the skin's own transform
    if bytes.u32()? == 0 {
        return None;
    }
    bytes.u8()?; // whether weights follow
    skin_transform(bytes.take(13 * 4)?)
}

/// The skin data of a skin instance.
fn skin_data_of(instance: &[u8]) -> Option<usize> {
    reference(Bytes::new(instance).u32()?)
}

/// The skin partition of a skin instance.
fn skin_partition_of(instance: &[u8]) -> Option<usize> {
    let mut bytes = Bytes::new(instance);
    bytes.u32()?; // skin data
    reference(bytes.u32()?)
}

/// What a skin partition holds for drawing.
#[derive(Default)]
struct Partition {
    positions: Vec<f32>,
    uvs: Vec<f32>,
    indices: Vec<u32>,
    /// For every shared point: the bones of the skin instance it hangs on,
    /// by their number there, with their weights.
    influences: HashMap<u32, Vec<(usize, f32)>>,
}

/// From a skin partition: the shared points and texture coordinates, if
/// the partition keeps them, every partition's triangles in shared point
/// numbers, and the bones each point hangs on.
fn read_skin_partition(block: &[u8], bs_version: u32) -> Option<Partition> {
    let mut bytes = Bytes::new(block);
    let partitions = bytes.u32()?;
    let (mut positions, mut uvs) = (Vec::new(), Vec::new());
    if bs_version == 100 {
        let data_size = bytes.u32()? as usize;
        let vertex_size = bytes.u32()? as usize;
        let desc = u64::from(bytes.u32()?) | (u64::from(bytes.u32()?) << 32);
        let data = bytes.take(data_size)?;
        if let Some(count) = data_size.checked_div(vertex_size) {
            (positions, uvs) = VertexLayout::of(desc, bs_version).read(data, count);
        }
    }
    let mut indices = Vec::new();
    let mut influences: HashMap<u32, Vec<(usize, f32)>> = HashMap::new();
    for _ in 0..partitions.min(4096) {
        read_one_partition(&mut bytes, bs_version, &mut indices, &mut influences)?;
    }
    Some(Partition {
        positions,
        uvs,
        indices,
        influences,
    })
}

/// Reads one partition: adds its triangles and the bones of its points.
fn read_one_partition(
    bytes: &mut Bytes<'_>,
    bs_version: u32,
    indices: &mut Vec<u32>,
    influences: &mut HashMap<u32, Vec<(usize, f32)>>,
) -> Option<()> {
    let vertices = usize::from(bytes.u16()?);
    let triangles = usize::from(bytes.u16()?);
    let bones = usize::from(bytes.u16()?);
    let strips = usize::from(bytes.u16()?);
    let weights = usize::from(bytes.u16()?);
    let mut bone_list = Vec::new();
    for _ in 0..bones {
        bone_list.push(usize::from(bytes.u16()?));
    }
    let mut map = Vec::new();
    if bytes.u8()? != 0 {
        for _ in 0..vertices {
            map.push(u32::from(bytes.u16()?));
        }
    }
    let mut weight_values = Vec::new();
    if bytes.u8()? != 0 {
        for _ in 0..vertices * weights {
            weight_values.push(bytes.f32()?);
        }
    }
    let mut strip_lengths = Vec::new();
    for _ in 0..strips {
        strip_lengths.push(usize::from(bytes.u16()?));
    }
    let has_faces = bytes.u8()? != 0;
    // With one vertex buffer for all partitions (Skyrim Special
    // Edition), triangles already number the shared points; before,
    // they numbered the partition's own, which the map translates.
    let shared = bs_version == 100;
    let global = |local: u16| -> u32 {
        if shared {
            return u32::from(local);
        }
        map.get(usize::from(local))
            .copied()
            .unwrap_or_else(|| u32::from(local))
    };
    if has_faces && strips == 0 {
        for _ in 0..triangles * 3 {
            indices.push(global(bytes.u16()?));
        }
    } else if has_faces {
        for length in &strip_lengths {
            let mut strip = Vec::new();
            for _ in 0..*length {
                strip.push(global(bytes.u16()?));
            }
            for (index, window) in strip.windows(3).enumerate() {
                let &[a, b, c] = window else {
                    continue;
                };
                if a != b && b != c && a != c {
                    indices.extend(if index % 2 == 0 { [a, b, c] } else { [a, c, b] });
                }
            }
        }
    }
    if bytes.u8()? != 0 {
        let bone_indices = bytes.take(vertices * weights)?;
        for (local, (slots, values)) in bone_indices
            .chunks(weights.max(1))
            .zip(weight_values.chunks(weights.max(1)))
            .enumerate()
        {
            let point = map
                .get(local)
                .copied()
                .unwrap_or_else(|| u32::try_from(local).unwrap_or(u32::MAX));
            let list = influences.entry(point).or_default();
            for (slot, weight) in slots.iter().zip(values) {
                if *weight > 0.0
                    && let Some(bone) = bone_list.get(usize::from(*slot))
                {
                    list.push((*bone, *weight));
                }
            }
        }
    }
    if bs_version > 34 {
        bytes.u8()?; // level of detail
        bytes.u8()?; // global vertex buffer
    }
    if bs_version == 100 {
        bytes.skip(8)?; // vertex description
        bytes.skip(triangles * 6)?; // copy of the triangles
    }
    Some(())
}

/// The texture set a shader names, as block number.
fn texture_set_of(shader: &[u8], bs_version: u32) -> Option<usize> {
    let mut bytes = Bytes::new(shader);
    if bs_version >= 83 {
        bytes.u32()?; // shader type
    }
    bytes.u32()?; // name
    let extra = bytes.u32()?;
    for _ in 0..extra.min(4096) {
        bytes.u32()?;
    }
    bytes.u32()?; // controller
    bytes.u32()?; // flags 1
    bytes.u32()?; // flags 2
    bytes.skip(16)?; // texture offset and scale
    reference(bytes.u32()?)
}

/// The colour texture a shader of the type `kind` names: through its
/// texture set (found with `block`), or itself for an effect shader.
fn shader_texture<'a>(
    kind: &str,
    shader: &[u8],
    bs_version: u32,
    block: &dyn Fn(usize) -> Option<(&'a str, &'a [u8])>,
) -> Option<String> {
    match kind {
        "BSLightingShaderProperty" => {
            let (_, set) = block(texture_set_of(shader, bs_version)?)?;
            first_texture(set)
        }
        "BSEffectShaderProperty" => effect_texture(shader),
        _ => None,
    }
}

/// The material file a shader of the type `kind` names, if its name is
/// one: shaders of Fallout 4 are named after their material.
fn material_name(kind: &str, shader: &[u8], bs_version: u32, strings: &[String]) -> Option<String> {
    let mut bytes = Bytes::new(shader);
    match kind {
        "BSLightingShaderProperty" if bs_version >= 83 => bytes.u32()?, // shader type
        "BSLightingShaderProperty" | "BSEffectShaderProperty" => 0,
        _ => return None,
    };
    let name = strings.get(usize::try_from(bytes.u32()?).ok()?)?;
    let is_material = std::path::Path::new(name)
        .extension()
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("bgsm") || extension.eq_ignore_ascii_case("bgem")
        });
    is_material.then(|| name.clone())
}

/// The texture an effect shader (glass, glow, fire) names itself, instead
/// of through a texture set.
fn effect_texture(shader: &[u8]) -> Option<String> {
    let mut bytes = Bytes::new(shader);
    bytes.u32()?; // name
    let extra = bytes.u32()?;
    for _ in 0..extra.min(4096) {
        bytes.u32()?;
    }
    bytes.u32()?; // controller
    bytes.u32()?; // flags 1
    bytes.u32()?; // flags 2
    bytes.skip(16)?; // texture offset and scale
    let length = bytes.u32()? as usize;
    let text = crate::decode_text(bytes.take(length)?);
    (!text.is_empty()).then_some(text)
}

/// The colour texture of a texture set: its first slot.
fn first_texture(set: &[u8]) -> Option<String> {
    let mut bytes = Bytes::new(set);
    let slots = bytes.u32()?;
    if slots == 0 {
        return None;
    }
    let length = bytes.u32()? as usize;
    let text = crate::decode_text(bytes.take(length)?);
    (!text.is_empty()).then_some(text)
}

/// Where every node of a mesh lies, and every other block hanging from a
/// node with the place of that node. Walked from the root, which is the
/// first block. Blocks below a hidden node keep their place, since bones
/// may be among them, but are not counted as hanging there to be drawn.
#[allow(clippy::type_complexity)]
fn place<'a>(
    block: &dyn Fn(usize) -> Option<(&'a str, &'a [u8])>,
    bs_version: u32,
) -> (HashMap<usize, Transform>, Vec<(usize, Transform)>) {
    let mut world: HashMap<usize, Transform> = HashMap::new();
    let mut hanging = Vec::new();
    let mut pending = vec![(0usize, Transform::IDENTITY, 0u32, false)];
    while let Some((index, parent, depth, hidden)) = pending.pop() {
        if depth > 64 || world.contains_key(&index) {
            continue;
        }
        let Some((kind, bytes)) = block(index) else {
            continue;
        };
        if kind.ends_with("Node") {
            if let Some((local, children)) = node_children(bytes, bs_version) {
                let here = if index == 0 {
                    Transform::IDENTITY
                } else {
                    local.transform.within(&parent)
                };
                world.insert(index, here);
                for child in children {
                    pending.push((child, here, depth + 1, hidden || local.hidden));
                }
            }
        } else {
            world.insert(index, parent);
            if !hidden {
                hanging.push((index, parent));
            }
        }
    }
    (world, hanging)
}

/// The shapes of the mesh in `data`.
///
/// # Errors
///
/// If the data is not a mesh of a supported version. Shapes that cannot be
/// read are left out.
pub fn shapes(data: &[u8]) -> Result<Vec<Shape>, NifError> {
    if crate::nif_oblivion::has_oblivion_version(data) {
        return crate::nif_oblivion::shapes(data);
    }
    let (bs_version, found, strings) = blocks(data)?;
    let block = |index: usize| -> Option<(&str, &[u8])> {
        let info = found.get(index)?;
        Some((
            info.kind.as_str(),
            data.get(info.start..info.start + info.size)?,
        ))
    };
    let texture_of = |shader: Option<usize>| -> Option<String> {
        let (kind, shader) = block(shader?)?;
        shader_texture(kind, shader, bs_version, &block)
    };
    let material_of = |shader: Option<usize>| -> Option<String> {
        let (kind, shader) = block(shader?)?;
        material_name(kind, shader, bs_version, &strings)
    };

    // In Skyrim, surfaces with an effect shader are glow, flames and mist:
    // light, not a body; drawn plainly they would hide the thing. From
    // Fallout 4 on, glass and other real surfaces use it too, and are kept.
    let is_effect = |shader: Option<usize>| -> bool {
        bs_version < 130
            && shader
                .and_then(block)
                .is_some_and(|(kind, _)| kind == "BSEffectShaderProperty")
    };

    let (world, shape_blocks) = place(&block, bs_version);

    let mut shapes = Vec::new();
    for (index, parent) in shape_blocks {
        let Some((kind, bytes)) = block(index) else {
            continue;
        };
        match kind {
            "BSTriShape" | "BSMeshLODTriShape" | "BSSubIndexTriShape" | "BSDynamicTriShape" => {
                let dynamic = kind == "BSDynamicTriShape";
                let placed = read_av_object(&mut Bytes::new(bytes), bs_version);
                if placed.as_ref().is_some_and(|placed| placed.hidden) {
                    continue;
                }
                let Some(mut raw) = read_bs_tri_shape(bytes, bs_version, dynamic) else {
                    continue;
                };
                if is_effect(raw.shader) {
                    continue;
                }
                let local = placed.map_or(Transform::IDENTITY, |placed| placed.transform);
                let skinned = apply_skin(&mut raw, &block, &world, bs_version);
                // Points of a skin that has no weights are stored for the
                // skeleton: they are put where the first bone holds them.
                let bound = raw.skin.and_then(block).and_then(|(_, skin)| {
                    let bone = *world.get(&first_bone(skin)?)?;
                    let (_, data) = block(skin_data_of(skin)?)?;
                    Some(first_bone_transform(data)?.within(&bone))
                });
                let transform = if skinned {
                    Transform::IDENTITY
                } else {
                    bound.unwrap_or_else(|| local.within(&parent))
                };
                if let Some(shape) = finish(&raw.positions, raw.uvs, &raw.indices, transform) {
                    shapes.push(Shape {
                        texture: texture_of(raw.shader),
                        material: material_of(raw.shader),
                        alpha: raw.alpha,
                        ..shape
                    });
                }
            }
            "NiTriShape" => {
                let Some((local, data_index, shader, alpha)) = read_ni_tri_shape(bytes, bs_version)
                else {
                    continue;
                };
                if is_effect(shader) {
                    continue;
                }
                let Some((positions, uvs, indices)) = block(data_index)
                    .filter(|(kind, _)| *kind == "NiTriShapeData")
                    .and_then(|(_, data)| read_ni_tri_shape_data(data))
                else {
                    continue;
                };
                if let Some(shape) = finish(&positions, uvs, &indices, local.within(&parent)) {
                    shapes.push(Shape {
                        texture: texture_of(shader),
                        material: material_of(shader),
                        alpha,
                        ..shape
                    });
                }
            }
            _ => {}
        }
    }
    Ok(shapes)
}

/// Takes what a shape keeps in its skin partition (points, texture
/// coordinates, triangles, if the shape has none itself) and moves each
/// point where the bones it hangs on hold it, by their weights, as the game
/// puts it in its resting pose. Whether the points were moved.
#[allow(clippy::type_complexity)]
fn apply_skin<'a>(
    raw: &mut RawShape,
    block: &dyn Fn(usize) -> Option<(&'a str, &'a [u8])>,
    world: &HashMap<usize, Transform>,
    bs_version: u32,
) -> bool {
    let Some((_, skin)) = raw.skin.and_then(block) else {
        return false;
    };
    let Some(partition) = skin_partition_of(skin)
        .and_then(block)
        .and_then(|(_, partition)| read_skin_partition(partition, bs_version))
    else {
        return false;
    };
    if raw.indices.is_empty() {
        if raw.positions.is_empty() {
            raw.positions = partition.positions;
        }
        if raw.uvs.is_empty() {
            raw.uvs = partition.uvs;
        }
        raw.indices = partition.indices;
    }
    if partition.influences.is_empty() {
        return false;
    }
    let bones = bone_places(skin, block, world);
    raw.positions = skin_points(&raw.positions, &partition.influences, &bones);
    true
}

/// Where each bone of a skin instance takes the points stored for it: its
/// place in the mesh after the skin data's transform into its space.
#[allow(clippy::type_complexity)]
fn bone_places<'a>(
    instance: &[u8],
    block: &dyn Fn(usize) -> Option<(&'a str, &'a [u8])>,
    world: &HashMap<usize, Transform>,
) -> Vec<Option<Transform>> {
    let transforms = skin_data_of(instance)
        .and_then(block)
        .map_or_else(Vec::new, |(_, data)| bone_transforms(data));
    instance_bones(instance)
        .iter()
        .zip(&transforms)
        .map(|(node, to_bone)| {
            let placed = world.get(&(*node)?)?;
            Some(to_bone.within(placed))
        })
        .collect()
}

/// Points moved by the bones they hang on: each the weighted sum of where
/// its bones put it. A point without bones stays where it is.
fn skin_points(
    positions: &[f32],
    influences: &HashMap<u32, Vec<(usize, f32)>>,
    bones: &[Option<Transform>],
) -> Vec<f32> {
    let (points, _) = positions.as_chunks::<3>();
    points
        .iter()
        .enumerate()
        .flat_map(|(index, &point)| {
            let mut sum = [0.0f32; 3];
            let mut total = 0.0;
            let list = u32::try_from(index)
                .ok()
                .and_then(|index| influences.get(&index));
            for (bone, weight) in list.into_iter().flatten() {
                if let Some(Some(transform)) = bones.get(*bone) {
                    let [x, y, z] = transform.apply(point);
                    sum = [
                        sum[0] + weight * x,
                        sum[1] + weight * y,
                        sum[2] + weight * z,
                    ];
                    total += weight;
                }
            }
            if total > 0.0 {
                sum.map(|value| value / total)
            } else {
                point
            }
        })
        .collect()
}

/// A shape with its transform applied, if its numbers fit together.
pub(crate) fn finish(
    positions: &[f32],
    uvs: Vec<f32>,
    indices: &[u32],
    transform: Transform,
) -> Option<Shape> {
    let (points, _) = positions.as_chunks::<3>();
    let count = points.len();
    if count == 0 || indices.is_empty() {
        return None;
    }
    let (triangles, _) = indices.as_chunks::<3>();
    let indices: Vec<u32> = triangles
        .iter()
        .filter(|triangle| triangle.iter().all(|&index| (index as usize) < count))
        .flatten()
        .copied()
        .collect();
    if indices.is_empty() {
        return None;
    }
    let positions = points
        .iter()
        .flat_map(|&point| transform.apply(point))
        .collect();
    let uvs = if uvs.len() == count * 2 {
        uvs
    } else {
        Vec::new()
    };
    Some(Shape {
        positions,
        uvs,
        indices,
        texture: None,
        material: None,
        alpha: false,
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A mesh file of Bethesda version `bs_version` with the given blocks.
    fn mesh(bs_version: u32, blocks: &[(&str, Vec<u8>)]) -> Vec<u8> {
        mesh_with_strings(bs_version, blocks, &[])
    }

    /// A mesh file whose blocks can name `strings` by number.
    fn mesh_with_strings(bs_version: u32, blocks: &[(&str, Vec<u8>)], strings: &[&str]) -> Vec<u8> {
        let mut out = b"Gamebryo File Format, Version 20.2.0.7\n".to_vec();
        out.extend(0x1402_0007u32.to_le_bytes());
        out.push(1);
        out.extend(12u32.to_le_bytes());
        out.extend(u32::try_from(blocks.len()).unwrap().to_le_bytes());
        out.extend(bs_version.to_le_bytes());
        // Empty author and scripts, and in Fallout 4 the longest path.
        out.extend(vec![0u8; if bs_version >= 103 { 4 } else { 3 }]);
        let mut types: Vec<&str> = Vec::new();
        for (kind, _) in blocks {
            if !types.contains(kind) {
                types.push(kind);
            }
        }
        out.extend(u16::try_from(types.len()).unwrap().to_le_bytes());
        for kind in &types {
            out.extend(u32::try_from(kind.len()).unwrap().to_le_bytes());
            out.extend(kind.as_bytes());
        }
        for (kind, _) in blocks {
            let index = types.iter().position(|known| known == kind).unwrap();
            out.extend(u16::try_from(index).unwrap().to_le_bytes());
        }
        for (_, data) in blocks {
            out.extend(u32::try_from(data.len()).unwrap().to_le_bytes());
        }
        out.extend(u32::try_from(strings.len()).unwrap().to_le_bytes());
        out.extend(0u32.to_le_bytes()); // longest string, not checked
        for text in strings {
            out.extend(u32::try_from(text.len()).unwrap().to_le_bytes());
            out.extend(text.as_bytes());
        }
        out.extend(0u32.to_le_bytes()); // no groups
        for (_, data) in blocks {
            out.extend(data);
        }
        out
    }

    /// What every node and shape starts with: placed at `translation`.
    fn av_object(translation: [f32; 3]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(0u32.to_le_bytes()); // name
        out.extend(0u32.to_le_bytes()); // extra data
        out.extend((-1i32).to_le_bytes()); // controller
        out.extend(14u32.to_le_bytes()); // flags
        for value in translation {
            out.extend(value.to_le_bytes());
        }
        for value in [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0] {
            out.extend(value.to_le_bytes());
        }
        out.extend(1.0f32.to_le_bytes());
        out.extend((-1i32).to_le_bytes()); // collision
        out
    }

    fn node(children: &[i32]) -> Vec<u8> {
        let mut out = av_object([0.0, 0.0, 0.0]);
        out.extend(u32::try_from(children.len()).unwrap().to_le_bytes());
        for child in children {
            out.extend(child.to_le_bytes());
        }
        out.extend(0u32.to_le_bytes()); // effects
        out
    }

    /// A one-triangle `BSTriShape` of Skyrim Special Edition, moved up by 10.
    fn tri_shape() -> Vec<u8> {
        let mut out = av_object([0.0, 0.0, 10.0]);
        out.extend([0u8; 16]); // bounding sphere
        out.extend((-1i32).to_le_bytes()); // skin
        out.extend((-1i32).to_le_bytes()); // shader
        out.extend((-1i32).to_le_bytes()); // alpha
        // 24-byte vertices: position at 0 (16 bytes), texture coordinates at
        // 16; attributes: vertex and texture coordinates.
        let desc: u64 = 6 | (4 << 8) | (0x3 << 44);
        out.extend(desc.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(3u16.to_le_bytes());
        out.extend((3u32 * 24 + 6).to_le_bytes());
        for (x, y) in [(0.0f32, 0.0f32), (1.0, 0.0), (0.0, 1.0)] {
            for value in [x, y, 0.0, 0.0] {
                out.extend(value.to_le_bytes());
            }
            // u and v as half floats: 0.5 is 0x3800.
            out.extend(0x3800u16.to_le_bytes());
            out.extend(0x3800u16.to_le_bytes());
            out.extend([0u8; 4]);
        }
        for index in [0u16, 1, 2] {
            out.extend(index.to_le_bytes());
        }
        out.extend(0u32.to_le_bytes()); // particle data
        out
    }

    #[test]
    fn a_special_edition_shape_is_read_and_placed() {
        let data = mesh(
            100,
            &[("BSFadeNode", node(&[1])), ("BSTriShape", tri_shape())],
        );
        let found = shapes(&data).unwrap();
        assert_eq!(found.len(), 1);
        let shape = &found[0];
        assert_eq!(shape.indices, [0, 1, 2]);
        assert_eq!(
            shape.positions,
            [0.0, 0.0, 10.0, 1.0, 0.0, 10.0, 0.0, 1.0, 10.0]
        );
        assert_eq!(shape.uvs, [0.5; 6]);
        assert_eq!(shape.texture, None);
    }

    /// A one-triangle `BSTriShape` of Fallout 4 with the shader in block
    /// `shader`: like Skyrim Special Edition's, but with a 32-bit triangle
    /// count, half-precision positions and no particle data.
    fn fallout_4_tri_shape(shader: i32) -> Vec<u8> {
        let mut out = av_object([0.0, 0.0, 10.0]);
        out.extend([0u8; 16]); // bounding sphere
        out.extend((-1i32).to_le_bytes()); // skin
        out.extend(shader.to_le_bytes());
        out.extend((-1i32).to_le_bytes()); // alpha
        // 12-byte vertices: half-precision position at 0 (8 bytes),
        // texture coordinates at 8.
        let desc: u64 = 3 | (2 << 8) | (0x3 << 44);
        out.extend(desc.to_le_bytes());
        out.extend(1u32.to_le_bytes());
        out.extend(3u16.to_le_bytes());
        out.extend((3u32 * 12 + 6).to_le_bytes());
        // 1.0 is 0x3C00 as a half float.
        for (x, y) in [(0u16, 0u16), (0x3C00, 0), (0, 0x3C00)] {
            for value in [x, y, 0, 0, 0x3800, 0x3800] {
                out.extend(value.to_le_bytes());
            }
        }
        for index in [0u16, 1, 2] {
            out.extend(index.to_le_bytes());
        }
        out
    }

    /// An effect shader that names its texture itself.
    fn effect_shader(texture: &str) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(0u32.to_le_bytes()); // name
        out.extend(0u32.to_le_bytes()); // extra data
        out.extend((-1i32).to_le_bytes()); // controller
        out.extend([0u8; 8]); // flags
        for value in [0.0f32, 0.0, 1.0, 1.0] {
            out.extend(value.to_le_bytes());
        }
        out.extend(u32::try_from(texture.len()).unwrap().to_le_bytes());
        out.extend(texture.as_bytes());
        out
    }

    #[test]
    fn a_fallout_4_shape_is_read_with_its_effect_texture() {
        let data = mesh(
            130,
            &[
                ("BSFadeNode", node(&[1])),
                ("BSTriShape", fallout_4_tri_shape(2)),
                (
                    "BSEffectShaderProperty",
                    effect_shader(r"texturesPropsGlass_d.dds"),
                ),
            ],
        );
        let found = shapes(&data).unwrap();
        assert_eq!(found.len(), 1);
        let shape = &found[0];
        assert_eq!(shape.indices, [0, 1, 2]);
        assert_eq!(
            shape.positions,
            [0.0, 0.0, 10.0, 1.0, 0.0, 10.0, 0.0, 1.0, 10.0]
        );
        assert_eq!(shape.texture.as_deref(), Some(r"texturesPropsGlass_d.dds"));
    }

    #[test]
    fn a_fallout_4_shape_names_its_material() {
        let mut shader = Vec::new();
        shader.extend(1u32.to_le_bytes()); // shader type
        shader.extend(0u32.to_le_bytes()); // name: the first string
        shader.extend(0u32.to_le_bytes()); // extra data
        shader.extend((-1i32).to_le_bytes()); // controller
        shader.extend([0u8; 8]); // flags
        shader.extend([0u8; 16]); // texture offset and scale
        shader.extend((-1i32).to_le_bytes()); // no texture set
        let data = mesh_with_strings(
            130,
            &[
                ("BSFadeNode", node(&[1])),
                ("BSTriShape", fallout_4_tri_shape(2)),
                ("BSLightingShaderProperty", shader),
            ],
            &[r"MaterialsPropsBottle.BGSM"],
        );
        let found = shapes(&data).unwrap();
        assert_eq!(found[0].texture, None);
        assert_eq!(
            found[0].material.as_deref(),
            Some(r"MaterialsPropsBottle.BGSM")
        );
    }

    #[test]
    fn meshes_newer_than_fallout_4_are_not_read() {
        let data = mesh(155, &[("BSFadeNode", node(&[]))]);
        assert_eq!(shapes(&data), Err(NifError::UnsupportedVersion));
    }

    #[test]
    fn a_shape_with_points_out_of_range_keeps_only_its_good_triangles() {
        let mut shape = tri_shape();
        // The last index points past the three points.
        let at = shape.len() - 4 - 2;
        shape[at..at + 2].copy_from_slice(&7u16.to_le_bytes());
        let data = mesh(100, &[("BSFadeNode", node(&[1])), ("BSTriShape", shape)]);
        assert_eq!(shapes(&data).unwrap(), Vec::new());
    }

    /// Sets the hidden flag of a node or shape built by `av_object`.
    fn hide(mut block: Vec<u8>) -> Vec<u8> {
        block[12..16].copy_from_slice(&15u32.to_le_bytes());
        block
    }

    #[test]
    fn hidden_shapes_and_shapes_below_hidden_nodes_are_not_drawn() {
        let hidden_shape = mesh(
            100,
            &[
                ("BSFadeNode", node(&[1])),
                ("BSTriShape", hide(tri_shape())),
            ],
        );
        assert_eq!(shapes(&hidden_shape).unwrap(), Vec::new());
        let below_hidden_node = mesh(
            100,
            &[
                ("BSFadeNode", node(&[1, 2])),
                ("NiNode", hide(node(&[3]))),
                ("BSTriShape", tri_shape()),
                ("BSTriShape", tri_shape()),
            ],
        );
        assert_eq!(shapes(&below_hidden_node).unwrap().len(), 1);
    }

    #[test]
    fn shapes_with_an_effect_shader_are_not_drawn() {
        let mut glow = tri_shape();
        // The shader reference follows the object (72 bytes), the bounding
        // sphere (16) and the skin reference (4).
        glow[92..96].copy_from_slice(&2i32.to_le_bytes());
        let data = mesh(
            100,
            &[
                ("BSFadeNode", node(&[1])),
                ("BSTriShape", glow),
                ("BSEffectShaderProperty", vec![0; 8]),
            ],
        );
        assert_eq!(shapes(&data).unwrap(), Vec::new());
    }

    #[test]
    fn other_files_are_not_meshes() {
        assert_eq!(shapes(b"not a mesh"), Err(NifError::NotAMesh));
    }

    #[test]
    fn half_floats_are_read() {
        assert!((half(0x3C00) - 1.0).abs() < f32::EPSILON);
        assert!((half(0xC000) + 2.0).abs() < f32::EPSILON);
        assert!(half(0).abs() < f32::EPSILON);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = shapes(&data);
        }

        #[test]
        fn never_panics_on_a_damaged_mesh(
            position in 0usize..400, value in any::<u8>(), cut in 0usize..400,
        ) {
            let mut data = mesh(100, &[("BSFadeNode", node(&[1])), ("BSTriShape", tri_shape())]);
            if let Some(byte) = data.get_mut(position) {
                *byte = value;
            }
            data.truncate(data.len().saturating_sub(cut));
            let _ = shapes(&data);
        }
    }
}
