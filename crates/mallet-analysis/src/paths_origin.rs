//! Who made each triangle of the paths the game uses (ADR 0071): of every
//! plugin's version of a navigation mesh, in load order, the first from
//! which on every version has the triangle in the same place. A triangle
//! is the same when its three corners are; a mod that moves a corner made
//! the triangle anew.

use std::collections::HashSet;

use mallet_core::facts::navmeshes::{Beyond, NavmeshShape};

/// Who made the triangles of the last of several versions of one mesh.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Origins {
    /// For each triangle of the last version, the version that made it, by
    /// its place among the versions.
    pub made_by: Vec<usize>,
    /// The triangles of the last version that lead nowhere, though the
    /// version before had them leading on: taken out of the paths, as a
    /// repair does (ADR 0052, 0058).
    pub taken_out: Vec<u32>,
    /// How many triangles of the version `reference` the last version no
    /// longer has in the same place.
    pub gone: usize,
}

/// A triangle by its three corners, in a fixed order.
type Key = [[i32; 3]; 3];

/// The triangles of `shape` by their corners; `None` for one whose corners
/// are not all there.
fn keys(shape: &NavmeshShape) -> Vec<Option<Key>> {
    shape
        .triangles
        .iter()
        .map(|corners| {
            let at = |vertex: u32| shape.vertices.get(usize::try_from(vertex).ok()?).copied();
            let mut key = [at(corners[0])?, at(corners[1])?, at(corners[2])?];
            key.sort_unstable();
            Some(key)
        })
        .collect()
}

/// Whether a triangle leads nowhere: no edge steps on.
fn leads_nowhere(edges: Option<&[Beyond; 3]>) -> bool {
    edges.is_some_and(|edges| edges.iter().all(|beyond| *beyond == Beyond::Nothing))
}

/// Who made the triangles of the last of `versions`, every version of one
/// mesh in load order. `reference`, if given, is the version the triangles
/// gone are counted against, such as the game's own last one. Empty for no
/// versions.
pub fn origins(versions: &[&NavmeshShape], reference: Option<usize>) -> Origins {
    let Some((last, earlier)) = versions.split_last() else {
        return Origins::default();
    };
    let sets: Vec<HashSet<Key>> = earlier
        .iter()
        .map(|shape| keys(shape).into_iter().flatten().collect())
        .collect();
    let own = keys(last);
    let made_by = own
        .iter()
        .map(|key| {
            let Some(key) = key else {
                return earlier.len();
            };
            let mut maker = earlier.len();
            while maker > 0 && sets.get(maker - 1).is_some_and(|set| set.contains(key)) {
                maker -= 1;
            }
            maker
        })
        .collect();

    // Taken out: the version before led on from the same triangle.
    let mut taken_out = Vec::new();
    if let Some(before) = earlier.last() {
        let before_keys = keys(before);
        let led_on: HashSet<Key> = before_keys
            .iter()
            .zip(&before.edges)
            .filter(|(_, edges)| !leads_nowhere(Some(edges)))
            .filter_map(|(key, _)| *key)
            .collect();
        for (number, key) in own.iter().enumerate() {
            let Some(key) = key else {
                continue;
            };
            if leads_nowhere(last.edges.get(number))
                && led_on.contains(key)
                && let Ok(number) = u32::try_from(number)
            {
                taken_out.push(number);
            }
        }
    }

    let gone = reference
        .filter(|index| *index < earlier.len())
        .and_then(|index| versions.get(index))
        .map_or(0, |reference| {
            let now: HashSet<Key> = own.iter().flatten().copied().collect();
            keys(reference)
                .into_iter()
                .flatten()
                .collect::<HashSet<Key>>()
                .iter()
                .filter(|key| !now.contains(*key))
                .count()
        });
    Origins {
        made_by,
        taken_out,
        gone,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mallet_core::facts::RecordId;
    use mallet_core::facts::objects::Place;

    fn shape(plugin: &str, vertices: Vec<[i32; 3]>, triangles: Vec<[u32; 3]>) -> NavmeshShape {
        let edges = triangles
            .iter()
            .map(|_| [Beyond::Triangle(0), Beyond::Nothing, Beyond::Nothing])
            .collect();
        NavmeshShape {
            id: RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id: 1,
            },
            plugin: plugin.to_owned(),
            used: false,
            place: Place::Interior {
                cell: RecordId {
                    plugin: "skyrim.esm".to_owned(),
                    local_id: 2,
                },
            },
            place_name: None,
            vertices,
            triangles,
            edges,
            doors: Vec::new(),
        }
    }

    fn square() -> Vec<[i32; 3]> {
        vec![
            [0, 0, 0],
            [100, 0, 0],
            [100, 100, 0],
            [0, 100, 0],
            [200, 0, 0],
        ]
    }

    #[test]
    fn a_triangle_is_made_by_the_first_version_that_keeps_it() {
        let game = shape("Skyrim.esm", square(), vec![[0, 1, 2], [0, 2, 3]]);
        // The mod keeps one triangle, numbered differently, and adds one.
        let modded = shape("Mod.esp", square(), vec![[2, 3, 0], [1, 4, 2]]);
        let origins = origins(&[&game, &modded], Some(0));
        assert_eq!(origins.made_by, vec![0, 1]);
        assert_eq!(origins.gone, 1);
        assert_eq!(origins.taken_out, Vec::<u32>::new());
    }

    #[test]
    fn a_version_in_between_that_dropped_a_triangle_makes_it_again() {
        let game = shape("Skyrim.esm", square(), vec![[0, 1, 2]]);
        let other = shape("Other.esp", square(), vec![[0, 2, 3]]);
        let last = shape("Last.esp", square(), vec![[0, 1, 2]]);
        assert_eq!(origins(&[&game, &other, &last], None).made_by, vec![2]);
    }

    #[test]
    fn a_moved_corner_makes_the_triangle_anew() {
        let game = shape("Skyrim.esm", square(), vec![[0, 1, 2]]);
        let mut moved = square();
        moved[2] = [100, 100, 40];
        let modded = shape("Mod.esp", moved, vec![[0, 1, 2]]);
        assert_eq!(origins(&[&game, &modded], Some(0)).made_by, vec![1]);
    }

    #[test]
    fn triangles_that_lead_nowhere_any_more_are_taken_out() {
        let game = shape("Skyrim.esm", square(), vec![[0, 1, 2], [0, 2, 3]]);
        let mut repaired = shape(
            "Mod Mallet - Paths.esp",
            square(),
            vec![[0, 1, 2], [0, 2, 3]],
        );
        repaired.edges[1] = [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing];
        let origins = origins(&[&game, &repaired], Some(0));
        assert_eq!(origins.taken_out, vec![1]);
        assert_eq!(origins.made_by, vec![0, 0]);
    }

    #[test]
    fn broken_corners_and_no_versions_do_not_panic() {
        let broken = shape("Mod.esp", vec![[0, 0, 0]], vec![[0, 7, 9]]);
        assert_eq!(origins(&[&broken], Some(3)).made_by, vec![0]);
        assert_eq!(origins(&[], None), Origins::default());
    }
}
