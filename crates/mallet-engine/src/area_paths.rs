//! Where characters walk around a point, and who made each part of it, for
//! the paths mode of the 3D views (ADR 0071). Only reads.
//!
//! The navigation meshes of the cells asked about are read with every
//! plugin's version, once per set of cells: the last few sets are kept, so
//! that a camera that moves back and forth, or choices that change, do not
//! read the plugins again.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, Mutex};

use mallet_analysis::paths_origin;
use mallet_analysis::walkways::{self, Hidden};
use mallet_core::ObjectRef;
use mallet_core::area_paths::{AreaNavmesh, AreaPaths, MakerKind, PathsMaker, PathsOverHidden};
use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{Beyond, NavmeshShape, NearHidden};
use mallet_core::facts::objects::PlacedObject;
use mallet_game_bethesda::Game;
use mallet_game_bethesda::area::AreaReader;
use mallet_game_bethesda::walkways::Around;

use crate::ScanOptions;
use crate::area::{library, reader, setup_key};
use crate::walkways::{models_of, placed_object, record_id};

/// Game units per cell side.
const CELL: f32 = 4096.0;

/// The farthest around the point that is read, in game units: two and a
/// half cells, so at most six cells across.
const MAX_RADIUS: f32 = 2.5 * CELL;

/// How many sets of cells are kept.
const KEPT: usize = 6;

/// The sets of cells read last, newest last: by setup, place and cells.
static KEPT_READS: Mutex<Vec<(String, Arc<NearHidden>)>> = Mutex::new(Vec::new());

/// The paths in the worldspace `place` within `radius` game units of
/// `centre` (at most two and a half cells), or, without a centre, in the
/// interior cell `place` (by its editor ID): who made each triangle, where
/// the paths the game uses run over any of `hidden`. `None` if the setup
/// cannot be read.
pub fn area_paths(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    radius: f32,
    hidden: &[ObjectRef],
) -> Option<AreaPaths> {
    let started = std::time::Instant::now();
    let game = options.game().ok()?;
    let setup = setup_key(options)?;
    let radius = radius.clamp(256.0, MAX_RADIUS);
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;

    let mut around = Around::default();
    let mut cells: Vec<[i32; 2]> = Vec::new();
    let interior = if let Some(centre) = centre {
        let cells_here = cells_around(centre, radius);
        cells = cells_here.iter().map(|&(x, y)| [x, y]).collect();
        around.worlds.insert(place.to_owned(), cells_here);
        None
    } else {
        let cell = reader.interior_cell(place)?;
        around.interiors.insert(cell.clone());
        Some(cell)
    };

    let key = format!("{setup}|{}|{cells:?}|{interior:?}", place.to_lowercase());
    let kept = KEPT_READS.lock().ok().and_then(|reads| {
        reads
            .iter()
            .find(|(known, _)| *known == key)
            .map(|(_, read)| Arc::clone(read))
    });
    let read = if let Some(read) = kept {
        read
    } else {
        let read = Arc::new(reader.navmeshes(&around));
        if let Ok(mut reads) = KEPT_READS.lock() {
            reads.retain(|(known, _)| *known != key);
            reads.push((key, Arc::clone(&read)));
            let over = reads.len().saturating_sub(KEPT);
            reads.drain(..over);
        }
        read
    };

    let (makers, navmeshes, taken_out) = paths_of(game, &read);

    let over_hidden = if hidden.is_empty() {
        Vec::new()
    } else {
        let asked = Asked {
            place,
            centre,
            radius,
            interior: interior.as_ref(),
        };
        paths_over_hidden(options, &mut reader, &asked, hidden, &read, &taken_out)
    };
    drop(reader);
    tracing::info!(
        place,
        cells = cells.len(),
        navmeshes = navmeshes.len(),
        over_hidden = over_hidden.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the paths of an area"
    );
    Some(AreaPaths {
        place: place.to_owned(),
        cells,
        makers,
        navmeshes,
        over_hidden,
    })
}

/// Where the objects asked about stand: as for [`area_paths`].
struct Asked<'a> {
    place: &'a str,
    centre: Option<[f32; 2]>,
    radius: f32,
    interior: Option<&'a RecordId>,
}

/// The triangles of the paths the game uses in `near` that run over those
/// of `hidden` that stand where `asked` says, without those `taken_out`.
fn paths_over_hidden(
    options: &ScanOptions,
    reader: &mut AreaReader,
    asked: &Asked<'_>,
    hidden: &[ObjectRef],
    near: &NearHidden,
    taken_out: &BTreeMap<String, BTreeSet<u32>>,
) -> Vec<PathsOverHidden> {
    let wanted: HashSet<RecordId> = hidden.iter().map(record_id).collect();
    let found: Vec<_> = match asked.centre {
        Some(centre) => reader.objects(asked.place, centre, asked.radius),
        None => reader.interior(asked.place),
    }
    .into_iter()
    .filter(|object| wanted.contains(&object.id))
    .collect();
    let Some(library) = library(options).filter(|_| !found.is_empty()) else {
        return Vec::new();
    };
    let (models, meshes) = models_of(reader, &library, &found);
    let placed: Vec<(PlacedObject, Option<&String>)> = found
        .iter()
        .zip(&models)
        .filter_map(|(object, model)| {
            Some((placed_object(object, asked.interior)?, model.as_ref()))
        })
        .collect();
    let shown: Vec<Hidden<'_>> = placed
        .iter()
        .map(|(object, model)| Hidden {
            object,
            mesh: model.and_then(|model| meshes.get(model)),
            bounds: None,
        })
        .collect();
    let mut over_hidden = Vec::new();
    for carried in walkways::carried(&shown, near) {
        let (Some(object), Some(shape)) = (
            shown.get(carried.hidden),
            near.navmeshes.get(carried.navmesh),
        ) else {
            continue;
        };
        if !shape.used {
            continue;
        }
        let navmesh = shape.id.to_string();
        let out = taken_out.get(&navmesh);
        let triangles: Vec<u32> = carried
            .triangles
            .into_iter()
            .filter(|triangle| out.is_none_or(|out| !out.contains(triangle)))
            .collect();
        // The object as the window named it.
        let Some(named) = hidden
            .iter()
            .find(|named| record_id(named) == object.object.id)
        else {
            continue;
        };
        if !triangles.is_empty() {
            over_hidden.push(PathsOverHidden {
                object: named.clone(),
                navmesh,
                triangles,
            });
        }
    }
    over_hidden
}

/// The cells within `radius` game units of `centre` in both directions.
#[allow(clippy::cast_possible_truncation)]
fn cells_around(centre: [f32; 2], radius: f32) -> BTreeSet<(i32, i32)> {
    // Worldspaces lie within a few hundred cells of their origin.
    let cell = |value: f32| (value / CELL).floor().clamp(-32_768.0, 32_767.0) as i32;
    let mut cells = BTreeSet::new();
    for x in cell(centre[0] - radius)..=cell(centre[0] + radius) {
        for y in cell(centre[1] - radius)..=cell(centre[1] + radius) {
            cells.insert((x, y));
        }
    }
    cells
}

/// Who made the versions of the meshes in `near`, the meshes the game
/// uses with who made each triangle, and the triangles each of them takes
/// out, by mesh.
fn paths_of(
    game: Game,
    near: &NearHidden,
) -> (
    Vec<PathsMaker>,
    Vec<AreaNavmesh>,
    BTreeMap<String, BTreeSet<u32>>,
) {
    let own = mallet_game_bethesda::paths_plugin::file_name();
    let mut makers: Vec<PathsMaker> = Vec::new();
    let mut maker_of = |plugin: &str| -> u16 {
        if let Some(found) = makers
            .iter()
            .position(|maker| maker.plugin.eq_ignore_ascii_case(plugin))
        {
            return u16::try_from(found).unwrap_or(u16::MAX);
        }
        let kind = if plugin.eq_ignore_ascii_case(&own) {
            MakerKind::Repair
        } else if game.is_official(plugin) {
            MakerKind::Game
        } else {
            MakerKind::Mod
        };
        makers.push(PathsMaker {
            plugin: plugin.to_owned(),
            kind,
        });
        u16::try_from(makers.len() - 1).unwrap_or(u16::MAX)
    };

    // Every version of each mesh, in load order.
    let mut versions: BTreeMap<&RecordId, Vec<&NavmeshShape>> = BTreeMap::new();
    for shape in &near.navmeshes {
        versions.entry(&shape.id).or_default().push(shape);
    }
    let mut navmeshes = Vec::new();
    let mut taken_out = BTreeMap::new();
    for (id, versions) in versions {
        let Some(last) = versions.last() else {
            continue;
        };
        if !last.used {
            continue;
        }
        let reference = versions
            .iter()
            .rposition(|shape| game.is_official(&shape.plugin));
        let origins = paths_origin::origins(&versions, reference);
        let version_makers: Vec<u16> = versions
            .iter()
            .map(|shape| maker_of(&shape.plugin))
            .collect();
        let made_by = origins
            .made_by
            .iter()
            .map(|version| version_makers.get(*version).copied().unwrap_or(u16::MAX))
            .collect();
        let middle = |corners: &[u32]| -> Option<[i32; 3]> {
            let mut sum = [0i64; 3];
            for vertex in corners {
                let at = last.vertices.get(usize::try_from(*vertex).ok()?)?;
                for (total, value) in sum.iter_mut().zip(at) {
                    *total += i64::from(*value);
                }
            }
            let count = i64::try_from(corners.len()).ok()?.max(1);
            Some(sum.map(|total| i32::try_from(total / count).unwrap_or_default()))
        };
        let doors = last
            .doors
            .iter()
            .filter_map(|(triangle, _)| {
                let corners = last.triangles.get(usize::try_from(*triangle).ok()?)?;
                middle(corners)
            })
            .collect();
        let mut links = Vec::new();
        for (corners, edges) in last.triangles.iter().zip(&last.edges) {
            for (edge, beyond) in edges.iter().enumerate() {
                if matches!(beyond, Beyond::Link { .. })
                    && let (Some(from), Some(to)) = (corners.get(edge), corners.get((edge + 1) % 3))
                    && let Some(at) = middle(&[*from, *to])
                {
                    links.push(at);
                }
            }
        }
        let navmesh = id.to_string();
        taken_out.insert(
            navmesh.clone(),
            origins.taken_out.iter().copied().collect::<BTreeSet<u32>>(),
        );
        navmeshes.push(AreaNavmesh {
            navmesh,
            cell: last.place_name.clone(),
            versions: version_makers,
            vertices: last.vertices.iter().flatten().copied().collect(),
            triangles: last.triangles.iter().flatten().copied().collect(),
            made_by,
            taken_out: origins.taken_out,
            gone: u32::try_from(origins.gone).unwrap_or(u32::MAX),
            doors,
            links,
        });
    }
    (makers, navmeshes, taken_out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mallet_core::facts::objects::Place;

    fn shape(plugin: &str, used: bool, triangles: Vec<[u32; 3]>) -> NavmeshShape {
        let id = RecordId {
            plugin: "skyrim.esm".to_owned(),
            local_id: 0x10,
        };
        NavmeshShape {
            id: id.clone(),
            plugin: plugin.to_owned(),
            used,
            place: Place::Interior { cell: id },
            place_name: Some("TestCell".to_owned()),
            vertices: vec![[0, 0, 0], [100, 0, 0], [100, 100, 0], [0, 100, 0]],
            edges: triangles
                .iter()
                .map(|_| {
                    [
                        Beyond::Link {
                            navmesh: RecordId {
                                plugin: "skyrim.esm".to_owned(),
                                local_id: 0x11,
                            },
                            triangle: 0,
                        },
                        Beyond::Triangle(1),
                        Beyond::Nothing,
                    ]
                })
                .collect(),
            triangles,
            doors: vec![(
                0,
                RecordId {
                    plugin: "skyrim.esm".to_owned(),
                    local_id: 0x20,
                },
            )],
        }
    }

    #[test]
    fn the_paths_say_who_made_each_triangle() {
        let near = NearHidden {
            navmeshes: vec![
                shape("Skyrim.esm", false, vec![[0, 1, 2]]),
                shape("Bridges.esp", false, vec![[0, 1, 2], [0, 2, 3]]),
                shape("Mod Mallet - Paths.esp", true, vec![[0, 1, 2], [0, 2, 3]]),
            ],
            ground: Vec::new(),
        };
        let (makers, navmeshes, _) = paths_of(Game::SkyrimSe, &near);
        let kinds: Vec<MakerKind> = makers.iter().map(|maker| maker.kind).collect();
        assert_eq!(
            kinds,
            vec![MakerKind::Game, MakerKind::Mod, MakerKind::Repair]
        );
        let [mesh] = navmeshes.as_slice() else {
            panic!("one mesh");
        };
        assert_eq!(mesh.navmesh, "skyrim.esm:000010");
        assert_eq!(mesh.versions, vec![0, 1, 2]);
        assert_eq!(mesh.made_by, vec![0, 1]);
        assert_eq!(mesh.doors, vec![[66, 33, 0]]);
        assert_eq!(mesh.links.len(), 2);
        assert_eq!(mesh.vertices.len(), 12);
    }

    #[test]
    fn cells_around_a_point_reach_as_far_as_asked() {
        let cells = cells_around([100.0, -100.0], 4096.0);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(-1, -2)) && cells.contains(&(1, 0)));
    }
}
