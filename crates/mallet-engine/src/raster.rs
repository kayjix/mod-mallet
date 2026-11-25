//! A small software renderer for pictures of meshes in lists (ADR 0085):
//! textured triangles seen from a fixed three-quarter angle under a
//! neutral light, with a depth buffer and two-by-two supersampling.
//!
//! It runs in the engine, beside the reading, so a list never waits for
//! the window's graphics and the pictures can be kept in the cache.

/// A texture to sample: red, green and blue per pixel, and the opacity.
pub(crate) struct Texture {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgb: Vec<u8>,
    pub(crate) alpha: Vec<u8>,
}

impl Texture {
    /// The colour and opacity at the texture coordinates `u`, `v`,
    /// repeated beyond 0..1 as the game repeats them.
    fn sample(&self, u: f32, v: f32) -> ([f32; 3], u8) {
        let width = self.width.max(1);
        let height = self.height.max(1);
        let wrap = |value: f32, size: u32| -> usize {
            let fraction = value - value.floor();
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            // The fraction lies in 0..1, so the product in 0..size.
            let at = (fraction * size as f32) as u32;
            at.min(size - 1) as usize
        };
        let at = wrap(v, height) * width as usize + wrap(u, width);
        let colour = self
            .rgb
            .get(at * 3..at * 3 + 3)
            .and_then(|rgb| <[u8; 3]>::try_from(rgb).ok())
            .map_or(PLAIN, |rgb| rgb.map(|channel| f32::from(channel) / 255.0));
        (colour, self.alpha.get(at).copied().unwrap_or(255))
    }
}

/// One surface to draw.
pub(crate) struct Surface<'a> {
    /// The points, three numbers each.
    pub(crate) positions: &'a [f32],
    /// Two texture coordinates per point, or empty.
    pub(crate) uvs: &'a [f32],
    /// The triangles, three point numbers each.
    pub(crate) indices: &'a [u32],
    /// The colour texture, if it could be read.
    pub(crate) texture: Option<&'a Texture>,
    /// Whether the transparent parts of the texture are left out.
    pub(crate) alpha_test: bool,
}

/// An image with an opacity per pixel, rows top to bottom.
pub(crate) struct Picture {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgb: Vec<u8>,
    pub(crate) alpha: Vec<u8>,
}

/// How far around the camera stands, in radians: the 3D viewer's start.
const YAW: f32 = 0.6;
/// How far above it looks down, in radians.
const PITCH: f32 = 0.3;
/// Where the light stands around and above, in radians, as seen from the
/// camera: the 3D viewer's default light.
const LIGHT_AROUND: f32 = 35.0 * std::f32::consts::PI / 180.0;
const LIGHT_UP: f32 = 40.0 * std::f32::consts::PI / 180.0;
/// The light all around, and how much the direct light adds.
const AMBIENT: f32 = 0.42;
const DIRECT: f32 = 0.72;
/// The colour of a surface without a texture.
const PLAIN: [f32; 3] = [0.72, 0.72, 0.72];
/// Free space around the thing, as a share of the side.
const MARGIN: f32 = 0.06;
/// Pixels per side of the picture, before averaging.
const SUPERSAMPLE: u32 = 2;
/// The most triangles drawn; larger meshes are drawn in part.
const MOST_TRIANGLES: usize = 2_000_000;

/// The surfaces seen from the fixed angle, as a square picture of `side`
/// pixels. `None` if nothing of them can be seen: no triangles, or all of
/// them left out.
pub(crate) fn draw(surfaces: &[Surface<'_>], side: u32) -> Option<Picture> {
    let side = side.clamp(8, 1024);
    let camera = Camera::new();

    // The frame: the points the triangles use, as the camera sees them.
    let mut low = [f32::INFINITY; 2];
    let mut high = [f32::NEG_INFINITY; 2];
    for surface in surfaces {
        for &index in surface.indices {
            if let Some(point) = point(surface.positions, index) {
                let [x, y, _] = camera.view(point);
                low = [low[0].min(x), low[1].min(y)];
                high = [high[0].max(x), high[1].max(y)];
            }
        }
    }
    let extent = (high[0] - low[0]).max(high[1] - low[1]);
    if !extent.is_finite() || extent <= f32::EPSILON {
        return None;
    }
    let full = side * SUPERSAMPLE;
    #[allow(clippy::cast_precision_loss)]
    let full_f = full as f32;
    let scale = full_f * (1.0 - 2.0 * MARGIN) / extent;
    let centre = [
        f32::midpoint(low[0], high[0]),
        f32::midpoint(low[1], high[1]),
    ];
    let to_screen = |[x, y, depth]: [f32; 3]| {
        [
            full_f / 2.0 + (x - centre[0]) * scale,
            // Rows go down, the camera's up goes up.
            full_f / 2.0 - (y - centre[1]) * scale,
            depth,
        ]
    };

    let mut target = Target::new(full);
    let mut drawn = 0usize;
    for surface in surfaces {
        let normals = normals_of(surface.positions, surface.indices);
        for triangle in surface.indices.as_chunks::<3>().0 {
            if drawn >= MOST_TRIANGLES {
                break;
            }
            drawn += 1;
            let corners = triangle.map(|index| {
                let position = point(surface.positions, index)?;
                let normal = point(&normals, index).unwrap_or([0.0, 0.0, 1.0]);
                let uv = surface
                    .uvs
                    .get(index as usize * 2..index as usize * 2 + 2)
                    .and_then(|uv| <[f32; 2]>::try_from(uv).ok())
                    .unwrap_or([0.0, 0.0]);
                Some(Corner {
                    screen: to_screen(camera.view(position)),
                    normal,
                    uv,
                })
            });
            if let [Some(a), Some(b), Some(c)] = corners {
                target.triangle(&[a, b, c], surface, &camera.light);
            }
        }
    }
    target.finish(side)
}

/// Where the camera looks from, and the light.
struct Camera {
    right: [f32; 3],
    up: [f32; 3],
    /// From the camera into the picture.
    forward: [f32; 3],
    /// Towards the light, in the mesh's space.
    light: [f32; 3],
}

impl Camera {
    fn new() -> Self {
        // Skyrim's up is z. The camera stands where the 3D viewer starts.
        let towards_eye = [
            PITCH.cos() * YAW.cos(),
            PITCH.cos() * YAW.sin(),
            PITCH.sin(),
        ];
        let forward = towards_eye.map(|value| -value);
        let right = normalized(cross(forward, [0.0, 0.0, 1.0]));
        let up = cross(right, forward);
        let around = LIGHT_AROUND + YAW;
        let light = [
            LIGHT_UP.cos() * around.cos(),
            LIGHT_UP.cos() * around.sin(),
            LIGHT_UP.sin(),
        ];
        Self {
            right,
            up,
            forward,
            light,
        }
    }

    /// A point as the camera sees it: across, up, and how far in.
    fn view(&self, point: [f32; 3]) -> [f32; 3] {
        [
            dot(point, self.right),
            dot(point, self.up),
            dot(point, self.forward),
        ]
    }
}

/// One corner of a triangle on the screen.
#[derive(Clone, Copy)]
struct Corner {
    /// Across and down in pixels, and how far in.
    screen: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}

/// The pixels being drawn, with their depth.
struct Target {
    side: u32,
    colour: Vec<[f32; 3]>,
    depth: Vec<f32>,
}

impl Target {
    fn new(side: u32) -> Self {
        let count = (side * side) as usize;
        Self {
            side,
            colour: vec![[0.0; 3]; count],
            depth: vec![f32::INFINITY; count],
        }
    }

    /// Fills the pixels whose middle the triangle covers, nearest first.
    fn triangle(&mut self, corners: &[Corner; 3], surface: &Surface<'_>, light: &[f32; 3]) {
        let [a, b, c] = corners.map(|corner| corner.screen);
        let area = edge(a, b, c);
        if area.abs() <= f32::EPSILON || !area.is_finite() {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let last = (self.side - 1) as f32;
        let clamp = |value: f32| value.clamp(0.0, last);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        // Clamped to the picture, so whole numbers within it.
        let (left, right, top, bottom) = (
            clamp(a[0].min(b[0]).min(c[0]).floor()) as u32,
            clamp(a[0].max(b[0]).max(c[0]).ceil()) as u32,
            clamp(a[1].min(b[1]).min(c[1]).floor()) as u32,
            clamp(a[1].max(b[1]).max(c[1]).ceil()) as u32,
        );
        for row in top..=bottom {
            for column in left..=right {
                #[allow(clippy::cast_precision_loss)]
                let pixel = [column as f32 + 0.5, row as f32 + 0.5, 0.0];
                let weights = [edge(b, c, pixel), edge(c, a, pixel), edge(a, b, pixel)]
                    .map(|weight| weight / area);
                if weights.iter().any(|&weight| weight < 0.0) {
                    continue;
                }
                let blend = |values: [f32; 3]| {
                    values[0] * weights[0] + values[1] * weights[1] + values[2] * weights[2]
                };
                let depth = blend([a[2], b[2], c[2]]);
                let at = (row * self.side + column) as usize;
                if self.depth.get(at).is_none_or(|&known| depth >= known) {
                    continue;
                }
                let uv = [
                    blend(corners.map(|corner| corner.uv[0])),
                    blend(corners.map(|corner| corner.uv[1])),
                ];
                let (base, opacity) = surface
                    .texture
                    .map_or((PLAIN, 255), |texture| texture.sample(uv[0], uv[1]));
                if surface.alpha_test && opacity < 128 {
                    continue;
                }
                let normal = [
                    blend(corners.map(|corner| corner.normal[0])),
                    blend(corners.map(|corner| corner.normal[1])),
                    blend(corners.map(|corner| corner.normal[2])),
                ];
                // Normals of meshes do not always face out; light both sides
                // alike, as the 3D viewer does.
                let direct = dot(normalized(normal), *light).abs();
                let shade = AMBIENT + DIRECT * direct;
                if let (Some(slot), Some(known)) = (self.colour.get_mut(at), self.depth.get_mut(at))
                {
                    *slot = base.map(|channel| (channel * shade).clamp(0.0, 1.0));
                    *known = depth;
                }
            }
        }
    }

    /// The picture at `side` pixels, each the average of its samples;
    /// `None` if nothing was drawn.
    fn finish(&self, side: u32) -> Option<Picture> {
        let mut rgb = Vec::with_capacity((side * side * 3) as usize);
        let mut alpha = Vec::with_capacity((side * side) as usize);
        let mut any = false;
        for row in 0..side {
            for column in 0..side {
                let mut sum = [0.0f32; 3];
                let mut covered = 0u32;
                for dy in 0..SUPERSAMPLE {
                    for dx in 0..SUPERSAMPLE {
                        let at = ((row * SUPERSAMPLE + dy) * self.side + column * SUPERSAMPLE + dx)
                            as usize;
                        if self.depth.get(at).is_some_and(|depth| depth.is_finite())
                            && let Some(colour) = self.colour.get(at)
                        {
                            covered += 1;
                            for (total, channel) in sum.iter_mut().zip(colour) {
                                *total += channel;
                            }
                        }
                    }
                }
                any |= covered > 0;
                #[allow(clippy::cast_precision_loss)]
                let share = covered.max(1) as f32;
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                // Colours lie in 0..1, so bytes.
                rgb.extend(sum.map(|total| (total / share * 255.0).round() as u8));
                #[allow(clippy::cast_possible_truncation)]
                // At most SUPERSAMPLE² samples of 255 each over SUPERSAMPLE².
                alpha.push((covered * 255 / (SUPERSAMPLE * SUPERSAMPLE)) as u8);
            }
        }
        any.then_some(Picture {
            width: side,
            height: side,
            rgb,
            alpha,
        })
    }
}

/// Twice the signed area of the triangle `a`, `b`, `c` on the screen.
fn edge(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Point `index` of `positions`, three numbers each.
fn point(positions: &[f32], index: u32) -> Option<[f32; 3]> {
    let at = index as usize * 3;
    positions
        .get(at..at + 3)
        .and_then(|point| <[f32; 3]>::try_from(point).ok())
        .filter(|point| point.iter().all(|value| value.is_finite()))
}

/// Smooth normals, one per point, from the triangles around it.
fn normals_of(positions: &[f32], indices: &[u32]) -> Vec<f32> {
    let mut normals = vec![0.0f32; positions.len()];
    for triangle in indices.as_chunks::<3>().0 {
        let [Some(a), Some(b), Some(c)] = triangle.map(|index| point(positions, index)) else {
            continue;
        };
        let normal = cross(
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        for index in triangle {
            let at = *index as usize * 3;
            if let Some(slot) = normals.get_mut(at..at + 3) {
                for (total, part) in slot.iter_mut().zip(normal) {
                    *total += part;
                }
            }
        }
    }
    normals
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt();
    if length > f32::EPSILON && length.is_finite() {
        vector.map(|value| value / length)
    } else {
        [0.0, 0.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A cube of side 2 around the origin, twelve triangles.
    fn cube() -> (Vec<f32>, Vec<u32>) {
        let positions = vec![
            -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, 1.0,
            1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0,
        ];
        let indices = vec![
            0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 1, 2, 6, 1, 6,
            5, 0, 3, 7, 0, 7, 4,
        ];
        (positions, indices)
    }

    fn opaque(picture: &Picture) -> usize {
        picture.alpha.iter().filter(|&&alpha| alpha > 0).count()
    }

    #[test]
    fn a_cube_fills_the_middle_and_leaves_the_corners_clear() {
        let (positions, indices) = cube();
        let surface = Surface {
            positions: &positions,
            uvs: &[],
            indices: &indices,
            texture: None,
            alpha_test: false,
        };
        let picture = draw(&[surface], 32).unwrap();
        assert_eq!((picture.width, picture.height), (32, 32));
        let middle = (16 * 32 + 16) as usize;
        assert_eq!(picture.alpha[middle], 255);
        assert_eq!(picture.alpha[0], 0, "the top left corner is background");
        // Lit surfaces are neither black nor brighter than white.
        assert!(picture.rgb[middle * 3] > 60);
        assert!(opaque(&picture) > 32 * 32 / 3);
    }

    #[test]
    fn the_texture_colours_the_surface() {
        let (positions, indices) = cube();
        let uvs: Vec<f32> = (0..8).flat_map(|_| [0.5, 0.5]).collect();
        let red = Texture {
            width: 1,
            height: 1,
            rgb: vec![255, 0, 0],
            alpha: vec![255],
        };
        let surface = Surface {
            positions: &positions,
            uvs: &uvs,
            indices: &indices,
            texture: Some(&red),
            alpha_test: false,
        };
        let picture = draw(&[surface], 16).unwrap();
        let middle = (8 * 16 + 8) as usize * 3;
        assert!(picture.rgb[middle] > 100);
        assert_eq!(picture.rgb[middle + 1], 0);
        assert_eq!(picture.rgb[middle + 2], 0);
    }

    #[test]
    fn transparent_parts_are_left_out_where_the_shape_asks() {
        let (positions, indices) = cube();
        let clear = Texture {
            width: 1,
            height: 1,
            rgb: vec![0, 0, 0],
            alpha: vec![0],
        };
        let surface = |alpha_test| Surface {
            positions: &positions,
            uvs: &[],
            indices: &indices,
            texture: Some(&clear),
            alpha_test,
        };
        assert!(draw(&[surface(true)], 16).is_none());
        assert!(draw(&[surface(false)], 16).is_some());
    }

    #[test]
    fn nothing_to_see_gives_no_picture() {
        assert!(draw(&[], 16).is_none());
        let flat = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let surface = Surface {
            positions: &flat,
            uvs: &[],
            indices: &[0, 1, 2],
            texture: None,
            alpha_test: false,
        };
        assert!(draw(&[surface], 16).is_none());
    }

    proptest! {
        #[test]
        fn never_panics_on_any_numbers(
            positions in proptest::collection::vec(any::<f32>(), 0..60),
            uvs in proptest::collection::vec(any::<f32>(), 0..40),
            indices in proptest::collection::vec(0u32..30, 0..60),
        ) {
            let texture = Texture { width: 2, height: 1, rgb: vec![1, 2, 3], alpha: vec![] };
            let surface = Surface {
                positions: &positions,
                uvs: &uvs,
                indices: &indices,
                texture: Some(&texture),
                alpha_test: true,
            };
            let _ = draw(&[surface], 16);
        }
    }
}
