//! Small pieces of geometry that several analyses of walkways and
//! navigation meshes share.

use mallet_core::facts::navmeshes::NavmeshShape;

use crate::contact::Point;

/// The vertex numbered `index` of a navigation mesh, in game units; `None`
/// if the mesh has no such vertex.
pub(crate) fn mesh_vertex(shape: &NavmeshShape, index: u32) -> Option<Point> {
    let [x, y, z] = *shape.vertices.get(usize::try_from(index).ok()?)?;
    Some([f64::from(x), f64::from(y), f64::from(z)])
}

/// The corners of a triangle of a navigation mesh, given by the numbers of
/// its vertices; `None` if the mesh lacks one of them.
pub(crate) fn mesh_corners(shape: &NavmeshShape, triangle: [u32; 3]) -> Option<[Point; 3]> {
    let [a, b, c] = triangle;
    Some([
        mesh_vertex(shape, a)?,
        mesh_vertex(shape, b)?,
        mesh_vertex(shape, c)?,
    ])
}

/// The middle of a triangle: the mean of its corners.
pub(crate) fn triangle_middle([a, b, c]: &[Point; 3]) -> Point {
    [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ]
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::RecordId;
    use mallet_core::facts::objects::Place;

    use super::*;

    fn shape(vertices: Vec<[i32; 3]>) -> NavmeshShape {
        NavmeshShape {
            id: RecordId::new("skyrim.esm", 0x210),
            plugin: "Skyrim.esm".to_owned(),
            used: true,
            place: Place::Interior {
                cell: RecordId::new("skyrim.esm", 0x100),
            },
            place_name: None,
            vertices,
            triangles: Vec::new(),
            edges: Vec::new(),
            doors: Vec::new(),
        }
    }

    #[test]
    fn corners_are_read_by_number_and_missing_ones_are_none() {
        let shape = shape(vec![[0, 0, 0], [30, 0, 3], [0, 30, 6]]);
        let corners = mesh_corners(&shape, [0, 1, 2]).unwrap();
        assert_eq!(corners[1], [30.0, 0.0, 3.0]);
        assert_eq!(triangle_middle(&corners), [10.0, 10.0, 3.0]);
        assert_eq!(mesh_corners(&shape, [0, 1, 3]), None);
        assert_eq!(mesh_vertex(&shape, u32::MAX), None);
    }
}
