//! Whether the surfaces of two placed models cross, and over how long a
//! stretch (ADR 0039).
//!
//! Every edge of each model's triangles is tested against the triangles of
//! the other model that lie near it; where an edge passes through a
//! triangle, the surfaces cross. Only the triangles inside the space both
//! models take up are tested, sorted into a grid so that each is compared
//! only with its neighbours.

/// A point or direction in the world, in game units.
pub(crate) type Point = [f64; 3];

/// Most triangles of one model that are tested in the shared space. Beyond
/// this the answer is "not known", and the boxes decide as before.
const MAX_TRIANGLES: usize = 60_000;

/// Most grid cells along each side of the shared space.
const GRID_CELLS: f64 = 24.0;

/// Below this, numbers count as zero in the intersection tests.
const EPSILON: f64 = 1e-9;

/// A model placed in the world: its points, and its triangles as numbers of
/// points.
pub(crate) struct Placed<'a> {
    /// The points, in the world.
    pub(crate) points: Vec<Point>,
    /// The triangles.
    pub(crate) triangles: &'a [[u32; 3]],
}

impl Placed<'_> {
    /// The corners of a triangle, if all its points exist.
    fn corners(&self, triangle: [u32; 3]) -> Option<[Point; 3]> {
        let point = |index: u32| self.points.get(usize::try_from(index).ok()?).copied();
        Some([
            point(triangle[0])?,
            point(triangle[1])?,
            point(triangle[2])?,
        ])
    }

    /// The box around all points.
    fn bounds(&self) -> Option<(Point, Point)> {
        let first = *self.points.first()?;
        Some(
            self.points
                .iter()
                .fold((first, first), |(min, max), point| {
                    (lower(min, *point), upper(max, *point))
                }),
        )
    }
}

/// How long the stretch is along which the surfaces of `a` and `b` cross:
/// the longest side of the box around all crossing points, in game units.
/// `Some(0.0)` where they do not cross, `None` where it could not be told
/// because the models are too detailed where they meet.
pub(crate) fn crossing(a: &Placed<'_>, b: &Placed<'_>) -> Option<f64> {
    let (Some((a_min, a_max)), Some((b_min, b_max))) = (a.bounds(), b.bounds()) else {
        return Some(0.0);
    };
    let min = upper(a_min, b_min);
    let max = lower(a_max, b_max);
    if min.iter().zip(&max).any(|(low, high)| low > high) {
        return Some(0.0);
    }
    let near = |placed: &Placed<'_>| -> Option<Vec<[Point; 3]>> {
        let found: Vec<[Point; 3]> = placed
            .triangles
            .iter()
            .filter_map(|&triangle| placed.corners(triangle))
            .filter(|corners| {
                let (low, high) = triangle_bounds(corners);
                boxes_meet((low, high), (min, max))
            })
            .collect();
        (found.len() <= MAX_TRIANGLES).then_some(found)
    };
    let (first, second) = (near(a)?, near(b)?);
    if first.is_empty() || second.is_empty() {
        return Some(0.0);
    }

    let grid = Grid::new(min, max, &second);
    let mut stamp = vec![usize::MAX; second.len()];
    let mut found: Option<(Point, Point)> = None;
    let mut add = |point: Point| {
        found = Some(match found {
            None => (point, point),
            Some((low, high)) => (lower(low, point), upper(high, point)),
        });
    };
    for (index, triangle) in first.iter().enumerate() {
        let (low, high) = triangle_bounds(triangle);
        for other in grid.near(low, high) {
            let Some(seen) = stamp.get_mut(other) else {
                continue;
            };
            if *seen == index {
                continue;
            }
            *seen = index;
            let Some(other) = second.get(other) else {
                continue;
            };
            for point in crossings(triangle, other) {
                add(point);
            }
        }
    }
    Some(found.map_or(0.0, |(low, high)| {
        high.iter()
            .zip(&low)
            .map(|(high, low)| high - low)
            .fold(0.0, f64::max)
    }))
}

/// The triangles of one model sorted into the cells of a grid over the
/// shared space.
struct Grid {
    min: Point,
    cell: Point,
    cells: [usize; 3],
    members: Vec<Vec<usize>>,
}

impl Grid {
    fn new(min: Point, max: Point, triangles: &[[Point; 3]]) -> Self {
        let side = |low: f64, high: f64| (high - low).max(1.0);
        let sides = [
            side(min[0], max[0]),
            side(min[1], max[1]),
            side(min[2], max[2]),
        ];
        let cell = sides.map(|side| (side / GRID_CELLS).max(1.0));
        let cells = [
            cell_count(sides[0], cell[0]),
            cell_count(sides[1], cell[1]),
            cell_count(sides[2], cell[2]),
        ];
        let mut grid = Self {
            min,
            cell,
            cells,
            members: vec![Vec::new(); cells[0] * cells[1] * cells[2]],
        };
        for (index, triangle) in triangles.iter().enumerate() {
            let (low, high) = triangle_bounds(triangle);
            for slot in grid.slots(low, high) {
                if let Some(members) = grid.members.get_mut(slot) {
                    members.push(index);
                }
            }
        }
        grid
    }

    /// The cell numbers along each axis that a box reaches into.
    fn range(&self, low: Point, high: Point) -> [(usize, usize); 3] {
        let axis = |low: f64, high: f64, min: f64, cell: f64, cells: usize| {
            let at = |value: f64| cell_index(((value - min) / cell).floor(), cells);
            (at(low), at(high))
        };
        [
            axis(low[0], high[0], self.min[0], self.cell[0], self.cells[0]),
            axis(low[1], high[1], self.min[1], self.cell[1], self.cells[1]),
            axis(low[2], high[2], self.min[2], self.cell[2], self.cells[2]),
        ]
    }

    fn slots(&self, low: Point, high: Point) -> Vec<usize> {
        let [(x0, x1), (y0, y1), (z0, z1)] = self.range(low, high);
        let mut slots = Vec::new();
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    slots.push((x * self.cells[1] + y) * self.cells[2] + z);
                }
            }
        }
        slots
    }

    /// The triangles in the cells a box reaches into; a triangle may come
    /// more than once.
    fn near(&self, low: Point, high: Point) -> impl Iterator<Item = usize> + '_ {
        self.slots(low, high)
            .into_iter()
            .filter_map(|slot| self.members.get(slot))
            .flatten()
            .copied()
    }
}

/// How many cells of `cell` units cover `side` units, at least one and at
/// most the grid's size.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn cell_count(side: f64, cell: f64) -> usize {
    // Bounded to 1..=GRID_CELLS before converting.
    (side / cell).ceil().clamp(1.0, GRID_CELLS) as usize
}

/// A cell number from a count of steps, kept inside the grid.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn cell_index(steps: f64, cells: usize) -> usize {
    let last = cells.saturating_sub(1);
    if steps.is_nan() || steps <= 0.0 {
        0
    } else {
        // Clamped to the grid before converting.
        (steps.min(1e9) as usize).min(last)
    }
}

/// The points where an edge of one triangle passes through the other.
fn crossings(a: &[Point; 3], b: &[Point; 3]) -> Vec<Point> {
    let mut points = Vec::new();
    for (triangle, other) in [(a, b), (b, a)] {
        for (start, end) in [(0, 1), (1, 2), (2, 0)] {
            if let (Some(&from), Some(&to)) = (triangle.get(start), triangle.get(end))
                && let Some(point) = edge_through(from, to, other)
            {
                points.push(point);
            }
        }
    }
    points
}

/// Where the edge from `from` to `to` passes through `triangle`, if it does
/// (Möller and Trumbore's test).
fn edge_through(from: Point, to: Point, triangle: &[Point; 3]) -> Option<Point> {
    let [p0, p1, p2] = *triangle;
    let direction = minus(to, from);
    let edge1 = minus(p1, p0);
    let edge2 = minus(p2, p0);
    let normal_part = cross(direction, edge2);
    let determinant = dot(edge1, normal_part);
    if determinant.abs() < EPSILON {
        return None;
    }
    let inverse = 1.0 / determinant;
    let offset = minus(from, p0);
    let u = dot(offset, normal_part) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(offset, edge1);
    let v = dot(direction, q) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(edge2, q) * inverse;
    (0.0..=1.0).contains(&t).then(|| {
        [
            from[0] + direction[0] * t,
            from[1] + direction[1] * t,
            from[2] + direction[2] * t,
        ]
    })
}

/// Whether two boxes, each given by its lowest and highest corner, meet.
fn boxes_meet((low, high): (Point, Point), (min, max): (Point, Point)) -> bool {
    low.iter()
        .zip(&max)
        .chain(min.iter().zip(&high))
        .all(|(below, above)| below <= above)
}

fn triangle_bounds(triangle: &[Point; 3]) -> (Point, Point) {
    let [a, b, c] = *triangle;
    (lower(lower(a, b), c), upper(upper(a, b), c))
}

fn lower(a: Point, b: Point) -> Point {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])]
}

fn upper(a: Point, b: Point) -> Point {
    [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])]
}

fn minus(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A cube of side `side` with its lowest corner at `at`, as twelve
    /// triangles.
    fn cube(at: Point, side: f64) -> (Vec<Point>, Vec<[u32; 3]>) {
        let mut points = Vec::new();
        for i in 0..8u8 {
            points.push([
                at[0] + f64::from(i & 1) * side,
                at[1] + f64::from((i >> 1) & 1) * side,
                at[2] + f64::from((i >> 2) & 1) * side,
            ]);
        }
        let faces = [
            [0, 1, 3, 2],
            [4, 6, 7, 5],
            [0, 4, 5, 1],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 5, 7, 3],
        ];
        let triangles = faces
            .iter()
            .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
            .collect();
        (points, triangles)
    }

    fn placed(shape: &(Vec<Point>, Vec<[u32; 3]>)) -> Placed<'_> {
        Placed {
            points: shape.0.clone(),
            triangles: &shape.1,
        }
    }

    #[test]
    fn cubes_that_cut_into_each_other_cross_along_their_overlap() {
        let a = cube([0.0, 0.0, 0.0], 100.0);
        let b = cube([80.0, 10.0, 10.0], 50.0);
        let stretch = crossing(&placed(&a), &placed(&b)).unwrap();
        // The cut runs over b's 50 units of height and width.
        assert!((stretch - 50.0).abs() < 1e-6, "{stretch}");
    }

    #[test]
    fn cubes_apart_do_not_cross() {
        let a = cube([0.0, 0.0, 0.0], 100.0);
        let b = cube([150.0, 0.0, 0.0], 50.0);
        assert_eq!(crossing(&placed(&a), &placed(&b)), Some(0.0));
    }

    #[test]
    fn a_cube_inside_another_does_not_cross_its_surface() {
        // What stands under an arch or inside a hall does not touch it.
        let a = cube([0.0, 0.0, 0.0], 300.0);
        let b = cube([100.0, 100.0, 100.0], 50.0);
        assert_eq!(crossing(&placed(&a), &placed(&b)), Some(0.0));
    }

    #[test]
    fn missing_points_are_skipped() {
        let points = vec![[0.0; 3]];
        let triangles = [[0, 5, 9]];
        let broken = Placed {
            points,
            triangles: &triangles,
        };
        let a = cube([0.0, 0.0, 0.0], 100.0);
        assert_eq!(crossing(&broken, &placed(&a)), Some(0.0));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_shapes(
            points in proptest::collection::vec(prop::array::uniform3(-1e6f64..1e6), 0..20),
            triangles in proptest::collection::vec(prop::array::uniform3(0u32..25), 0..20),
            shift in prop::array::uniform3(-200f64..200.0),
        ) {
            let a = Placed { points: points.clone(), triangles: &triangles };
            let moved: Vec<Point> = points.iter().map(|p| [p[0] + shift[0], p[1] + shift[1], p[2] + shift[2]]).collect();
            let b = Placed { points: moved, triangles: &triangles };
            let _ = crossing(&a, &b);
        }
    }
}
