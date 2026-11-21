//! A net of triangles through given points, seen from above: as even as
//! it can be (Delaunay), and with chosen edges kept (ADR 0058).
//!
//! The repair of paths lays new triangles over the ground where a hidden
//! object stood: through the corners of the paths around it, whose edges
//! must be kept so that the new triangles join them, and through points on
//! the land. Points are added one by one; each lands in a triangle, which
//! is split, and edges are turned until no triangle's circle holds a
//! neighbour's far corner. The edges to keep are then turned into place.

use std::collections::{HashMap, HashSet};

use crate::contact::Point;

/// Below this, numbers count as zero.
const EPSILON: f64 = 1e-9;

/// How close to an edge, in game units, a point counts as lying on it.
const ON_EDGE: f64 = 0.5;

/// At most this many edges are turned per point or kept edge.
const MAX_TURNS: usize = 100_000;

/// A net of triangles over points.
pub(crate) struct Tin {
    /// The points, then three far corners of a triangle around them all.
    pub(crate) points: Vec<Point>,
    /// How many of `points` were given.
    given: usize,
    /// Anticlockwise seen from above.
    triangles: Vec<[usize; 3]>,
    /// Which triangle has each directed edge.
    edges: HashMap<(usize, usize), usize>,
    /// The edges kept, both directions.
    kept: HashSet<(usize, usize)>,
}

impl Tin {
    /// The even net through `points`. `None` for fewer than three.
    pub(crate) fn new(points: Vec<Point>) -> Option<Self> {
        if points.len() < 3 {
            return None;
        }
        let first = points.first()?;
        let (mut min, mut max) = ([first[0], first[1]], [first[0], first[1]]);
        for point in &points {
            min = [min[0].min(point[0]), min[1].min(point[1])];
            max = [max[0].max(point[0]), max[1].max(point[1])];
        }
        let span = (max[0] - min[0]).max(max[1] - min[1]).max(1.0);
        let centre = [f64::midpoint(min[0], max[0]), f64::midpoint(min[1], max[1])];
        let given = points.len();
        let mut tin = Self {
            points,
            given,
            triangles: Vec::new(),
            edges: HashMap::new(),
            kept: HashSet::new(),
        };
        tin.points.extend([
            [centre[0] - 20.0 * span, centre[1] - 10.0 * span, 0.0],
            [centre[0] + 20.0 * span, centre[1] - 10.0 * span, 0.0],
            [centre[0], centre[1] + 20.0 * span, 0.0],
        ]);
        tin.add([given, given + 1, given + 2]);
        for point in 0..given {
            tin.insert(point);
        }
        Some(tin)
    }

    /// The triangles that do not touch the far corners.
    pub(crate) fn triangles(&self) -> Vec<[usize; 3]> {
        self.triangles
            .iter()
            .copied()
            .filter(|triangle| triangle.iter().all(|corner| *corner < self.given))
            .collect()
    }

    /// Turns edges until the net has an edge from `a` to `b` and keeps
    /// it; where a point lies on the way, the edges to and from it. Whether
    /// it worked.
    pub(crate) fn keep(&mut self, a: usize, b: usize) -> bool {
        if a == b || a >= self.given || b >= self.given {
            return false;
        }
        let (Some(pa), Some(pb)) = (self.point(a), self.point(b)) else {
            return false;
        };
        // A point on the way splits the edge.
        let between = (0..self.given).find(|&other| {
            other != a
                && other != b
                && self
                    .point(other)
                    .is_some_and(|point| lies_between(pa, pb, point))
        });
        if let Some(middle) = between {
            let first = self.keep(a, middle);
            let second = self.keep(middle, b);
            return first && second;
        }
        for _ in 0..MAX_TURNS {
            if self.edges.contains_key(&(a, b)) || self.edges.contains_key(&(b, a)) {
                self.kept.insert((a, b));
                self.kept.insert((b, a));
                return true;
            }
            let crossing = self
                .triangles
                .iter()
                .enumerate()
                .find_map(|(index, triangle)| {
                    (0..3).find_map(|edge| {
                        let u = *triangle.get(edge)?;
                        let v = *triangle.get((edge + 1) % 3)?;
                        if [u, v].iter().any(|corner| *corner == a || *corner == b)
                            || self.kept.contains(&(u, v))
                        {
                            return None;
                        }
                        let (pu, pv) = (self.point(u)?, self.point(v)?);
                        let crosses = orient(pa, pb, pu) * orient(pa, pb, pv) < 0.0
                            && orient(pu, pv, pa) * orient(pu, pv, pb) < 0.0;
                        (crosses && self.can_turn(index, u, v)).then_some((u, v))
                    })
                });
            let Some((u, v)) = crossing else {
                return false;
            };
            self.turn(u, v);
        }
        false
    }

    /// Makes the net even again after edges were kept.
    pub(crate) fn even_out(&mut self) {
        let mut all: Vec<(usize, usize)> = self.edges.keys().copied().collect();
        all.sort_unstable();
        all.reverse();
        self.legalize(all);
    }

    fn point(&self, index: usize) -> Option<Point> {
        self.points.get(index).copied()
    }

    fn add(&mut self, triangle: [usize; 3]) -> usize {
        let index = self.triangles.len();
        self.triangles.push(triangle);
        for edge in 0..3 {
            if let (Some(&a), Some(&b)) = (triangle.get(edge), triangle.get((edge + 1) % 3)) {
                self.edges.insert((a, b), index);
            }
        }
        index
    }

    fn set(&mut self, index: usize, triangle: [usize; 3]) {
        if let Some(old) = self.triangles.get(index).copied() {
            for edge in 0..3 {
                if let (Some(&a), Some(&b)) = (old.get(edge), old.get((edge + 1) % 3))
                    && self.edges.get(&(a, b)) == Some(&index)
                {
                    self.edges.remove(&(a, b));
                }
            }
        }
        if let Some(slot) = self.triangles.get_mut(index) {
            *slot = triangle;
        }
        for edge in 0..3 {
            if let (Some(&a), Some(&b)) = (triangle.get(edge), triangle.get((edge + 1) % 3)) {
                self.edges.insert((a, b), index);
            }
        }
    }

    /// The corner of a triangle opposite its edge from `a` to `b`.
    fn opposite(&self, index: usize, a: usize, b: usize) -> Option<usize> {
        self.triangles
            .get(index)?
            .iter()
            .copied()
            .find(|corner| *corner != a && *corner != b)
    }

    /// Adds point `point` to the net.
    fn insert(&mut self, point: usize) {
        let Some(at) = self.point(point) else {
            return;
        };
        let found = self
            .triangles
            .iter()
            .enumerate()
            .find_map(|(index, triangle)| {
                let [a, b, c] = *triangle;
                let (pa, pb, pc) = (self.point(a)?, self.point(b)?, self.point(c)?);
                let inside = orient(pa, pb, at) >= -EPSILON
                    && orient(pb, pc, at) >= -EPSILON
                    && orient(pc, pa, at) >= -EPSILON;
                inside.then_some((index, *triangle))
            });
        let Some((index, [a, b, c])) = found else {
            return;
        };
        // On an edge: split both triangles beside it.
        let on_edge = [(a, b, c), (b, c, a), (c, a, b)]
            .into_iter()
            .find(|(u, v, _)| match (self.point(*u), self.point(*v)) {
                (Some(pu), Some(pv)) => distance_to_line(pu, pv, at) < ON_EDGE,
                _ => false,
            });
        let mut check = Vec::new();
        if let Some((u, v, w)) = on_edge {
            let other = self.edges.get(&(v, u)).copied();
            self.set(index, [u, point, w]);
            self.add([point, v, w]);
            check.extend([(u, w), (w, u), (v, w), (w, v)]);
            if let Some(other) = other
                && let Some(x) = self.opposite(other, v, u)
            {
                self.set(other, [v, point, x]);
                self.add([point, u, x]);
                check.extend([(v, x), (x, v), (u, x), (x, u)]);
            }
        } else {
            self.set(index, [a, b, point]);
            self.add([b, c, point]);
            self.add([c, a, point]);
            check.extend([(a, b), (b, c), (c, a)]);
        }
        self.legalize(check);
    }

    /// Turns the edges in `stack`, and those around turned ones, until no
    /// triangle's circle holds a neighbour's far corner.
    fn legalize(&mut self, mut stack: Vec<(usize, usize)>) {
        let mut turns = 0;
        while let Some((a, b)) = stack.pop() {
            if turns > MAX_TURNS {
                break;
            }
            if self.kept.contains(&(a, b)) {
                continue;
            }
            let (Some(&first), Some(&second)) = (self.edges.get(&(a, b)), self.edges.get(&(b, a)))
            else {
                continue;
            };
            let (Some(c), Some(d)) = (self.opposite(first, a, b), self.opposite(second, b, a))
            else {
                continue;
            };
            let (Some(pa), Some(pb), Some(pc), Some(pd)) =
                (self.point(a), self.point(b), self.point(c), self.point(d))
            else {
                continue;
            };
            if !in_circle(pa, pb, pc, pd) || !self.can_turn(first, a, b) {
                continue;
            }
            self.turn(a, b);
            turns += 1;
            stack.extend([(a, d), (d, b), (b, c), (c, a)]);
        }
    }

    /// Whether the edge from `a` to `b` of triangle `index` can be turned:
    /// the two triangles beside it make a convex four-sided shape.
    fn can_turn(&self, index: usize, a: usize, b: usize) -> bool {
        let Some(&other) = self.edges.get(&(b, a)) else {
            return false;
        };
        let (Some(c), Some(d)) = (self.opposite(index, a, b), self.opposite(other, b, a)) else {
            return false;
        };
        match (self.point(a), self.point(b), self.point(c), self.point(d)) {
            (Some(pa), Some(pb), Some(pc), Some(pd)) => {
                orient(pa, pd, pc) > EPSILON && orient(pd, pb, pc) > EPSILON
            }
            _ => false,
        }
    }

    /// Turns the edge from `a` to `b`: the two triangles beside it share
    /// the other diagonal instead.
    fn turn(&mut self, a: usize, b: usize) {
        let (Some(&first), Some(&second)) = (self.edges.get(&(a, b)), self.edges.get(&(b, a)))
        else {
            return;
        };
        let (Some(c), Some(d)) = (self.opposite(first, a, b), self.opposite(second, b, a)) else {
            return;
        };
        self.set(first, [a, d, c]);
        self.set(second, [d, b, c]);
    }
}

/// Twice the area of the triangle seen from above; positive when its
/// corners run anticlockwise.
pub(crate) fn orient(a: Point, b: Point, c: Point) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Whether `point` lies inside the circle through the anticlockwise
/// corners `a`, `b`, `c`, seen from above.
fn in_circle(a: Point, b: Point, c: Point, point: Point) -> bool {
    let row = |corner: Point| {
        let (east, north) = (corner[0] - point[0], corner[1] - point[1]);
        [east, north, east * east + north * north]
    };
    let (first, second, third) = (row(a), row(b), row(c));
    let det = first[0] * (second[1] * third[2] - second[2] * third[1])
        - first[1] * (second[0] * third[2] - second[2] * third[0])
        + first[2] * (second[0] * third[1] - second[1] * third[0]);
    det > EPSILON
}

/// How far `point` lies from the line through `a` and `b`, seen from
/// above.
fn distance_to_line(a: Point, b: Point, point: Point) -> f64 {
    let length = (b[0] - a[0]).hypot(b[1] - a[1]);
    if length < EPSILON {
        return (point[0] - a[0]).hypot(point[1] - a[1]);
    }
    orient(a, b, point).abs() / length
}

/// Whether `point` lies on the segment from `a` to `b`, between its ends.
fn lies_between(a: Point, b: Point, point: Point) -> bool {
    if distance_to_line(a, b, point) >= ON_EDGE {
        return false;
    }
    let along = (point[0] - a[0]) * (b[0] - a[0]) + (point[1] - a[1]) * (b[1] - a[1]);
    let length = (b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2);
    along > EPSILON && along < length - EPSILON
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn square_with_middle() -> Vec<Point> {
        vec![
            [0.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
            [50.0, 50.0, 10.0],
        ]
    }

    fn area(tin: &Tin) -> f64 {
        tin.triangles()
            .iter()
            .map(|[a, b, c]| orient(tin.points[*a], tin.points[*b], tin.points[*c]) / 2.0)
            .sum()
    }

    #[test]
    fn points_are_joined_into_anticlockwise_triangles_covering_them() {
        let tin = Tin::new(square_with_middle()).unwrap();
        let triangles = tin.triangles();
        assert_eq!(triangles.len(), 4);
        for [a, b, c] in &triangles {
            assert!(orient(tin.points[*a], tin.points[*b], tin.points[*c]) > 0.0);
        }
        assert!((area(&tin) - 10_000.0).abs() < 1e-6);
    }

    #[test]
    fn a_kept_edge_is_turned_into_place_and_stays() {
        // Four points on a long thin diamond: the even net joins the
        // near pair; keeping the far pair turns the edge.
        let points = vec![
            [0.0, 0.0, 0.0],
            [100.0, -10.0, 0.0],
            [200.0, 0.0, 0.0],
            [100.0, 10.0, 0.0],
        ];
        let mut tin = Tin::new(points).unwrap();
        assert!(tin.edges.contains_key(&(1, 3)) || tin.edges.contains_key(&(3, 1)));
        assert!(tin.keep(0, 2));
        tin.even_out();
        assert!(tin.edges.contains_key(&(0, 2)) || tin.edges.contains_key(&(2, 0)));
        assert_eq!(tin.triangles().len(), 2);
    }

    #[test]
    fn a_point_on_a_kept_edge_splits_it() {
        let points = vec![
            [0.0, 0.0, 0.0],
            [50.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [50.0, 40.0, 0.0],
            [50.0, -40.0, 0.0],
        ];
        let mut tin = Tin::new(points).unwrap();
        assert!(tin.keep(0, 2));
        assert!(tin.kept.contains(&(0, 1)) && tin.kept.contains(&(1, 2)));
    }

    #[test]
    fn too_few_points_make_no_net() {
        assert!(Tin::new(vec![[0.0; 3], [1.0, 0.0, 0.0]]).is_none());
    }

    proptest! {
        #[test]
        fn the_net_covers_the_hull_of_any_points(
            points in prop::collection::vec((-500i32..500, -500i32..500), 3..40),
            keep in prop::collection::vec((0usize..40, 0usize..40), 0..6),
        ) {
            let mut seen = HashSet::new();
            let points: Vec<Point> = points
                .into_iter()
                .filter(|point| seen.insert(*point))
                .map(|(x, y)| [f64::from(x), f64::from(y), 0.0])
                .collect();
            if let Some(mut tin) = Tin::new(points) {
                for (a, b) in keep {
                    let _ = tin.keep(a, b);
                }
                tin.even_out();
                for [a, b, c] in tin.triangles() {
                    prop_assert!(orient(tin.points[a], tin.points[b], tin.points[c]) > -1e-6);
                }
            }
        }
    }
}
