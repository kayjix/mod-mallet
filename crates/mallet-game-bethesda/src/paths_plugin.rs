//! The plugin that carries the repaired paths (ADR 0058):
//! `Mod Mallet - Paths.esp`.
//!
//! Each repaired navigation mesh goes into the temporary children of its
//! cell, inside a copy of the version the game uses of that cell and, outside,
//! of its worldspace, as the plugin of moves copies them (ADR 0060): every
//! reference rewritten for the plugin's masters, a field it does not know
//! refused rather than copied blindly. Its masters are the plugins whose
//! versions it copies and every plugin those refer to, in load order, so
//! the mod manager sorts it after all of them. Nothing here writes a file.

use std::collections::BTreeSet;
use std::path::PathBuf;

use mallet_core::facts::RecordId;
use mallet_core::facts::objects::Place;
use mallet_core::moves::MoveRefusal;
use mallet_core::walkway_repair::{PATHS_NAME, PathRefusal};
use mallet_formats_bethesda::navmesh_write::NavmeshData;
use mallet_formats_bethesda::plugin::{GroupLabel, resolve_form_id};
use mallet_formats_bethesda::plugin_write::{Children, PluginBuilder, WriteError, WrittenRecord};

use crate::game::Game;
use crate::moves::{self, Found, RecordVersion};
use crate::navmesh_records::{NavmeshRecord, form_id_in};
use crate::walkways::CHILDREN;

/// The plugin of paths, and what could not go into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathsPlugin {
    /// The plugin's bytes; `None` if it would repair nothing.
    pub bytes: Option<Vec<u8>>,
    /// Its masters, in load order.
    pub masters: Vec<String>,
    /// Whether it is flagged as light.
    pub light: bool,
    /// The meshes it repairs.
    pub repaired: Vec<RecordId>,
    /// The meshes it leaves out, and why.
    pub refused: Vec<(RecordId, PathRefusal)>,
}

/// The file name of the plugin of paths.
pub fn file_name() -> String {
    format!("{PATHS_NAME}.esp")
}

/// Builds the plugin that carries `records` into the setup whose active
/// plugins are `plugins` (name and file, in load order, without the plugin
/// of paths itself). `names` gives the text of a localized plugin's name
/// by plugin and number, where it can.
///
/// # Errors
///
/// If the plugin cannot be written, for example with more masters than a
/// plugin can name.
pub fn build(
    game: Game,
    plugins: &[(String, PathBuf)],
    records: &[NavmeshRecord],
    names: &dyn Fn(&str, u32) -> Option<String>,
) -> Result<PathsPlugin, WriteError> {
    let Some(format) = moves::plugin_format(game) else {
        return Ok(nothing(
            records
                .iter()
                .map(|record| (record.navmesh.clone(), PathRefusal::UnsupportedGame))
                .collect(),
        ));
    };
    let found = find(plugins, records);
    let name_of = |plugin: usize| plugins.get(plugin).map_or("", |(name, _)| name.as_str());
    let mut refused = Vec::new();
    let (planned, masters) = plan(&found, plugins, records, &mut refused);
    let own = file_name();

    let mut builder = PluginBuilder::new(masters.clone())
        .author("Mod Mallet")
        .description(
            "Repairs the paths characters walk on over objects Mod Mallet hides. \
             Made with Mod Mallet; remove it to bring the old paths back.",
        );
    let mut repaired = Vec::new();
    for (record, cell, world) in planned {
        let copy = |version: &RecordVersion| {
            let source = found
                .sources
                .get(&version.plugin)
                .cloned()
                .unwrap_or_default();
            moves::copied(version, &source, name_of(version.plugin), &masters, names)
        };
        let (cell, world) = match (copy(cell), world.map(copy).transpose()) {
            (Ok(cell), Ok(world)) => (cell, world),
            (Err(reason), _) | (_, Err(reason)) => {
                refused.push((record.navmesh.clone(), refusal(reason)));
                continue;
            }
        };
        let (Some(form_id), Some(fields)) = (
            form_id_in(&record.navmesh, &masters, &own),
            record.fields_for(&masters, &own),
        ) else {
            refused.push((record.navmesh.clone(), PathRefusal::NotFound));
            continue;
        };
        let navmesh = WrittenRecord {
            signature: *b"NAVM",
            form_id,
            flags: record.flags,
            form_version: record.form_version,
            subrecords: fields,
        };
        match &world {
            Some(world) => {
                builder.add_to_exterior(world, &cell, Children::Temporary, vec![navmesh]);
            }
            None => builder.add_to_interior(&cell, Children::Temporary, vec![navmesh]),
        }
        repaired.push(record.navmesh.clone());
    }
    if repaired.is_empty() {
        return Ok(nothing(refused));
    }
    let light = builder.fits_light(&format);
    Ok(PathsPlugin {
        bytes: Some(builder.write(&format)?),
        masters,
        light,
        repaired,
        refused,
    })
}

/// What one record copies: the mesh, and the versions the game uses of
/// its cell and, outside, its worldspace.
type Planned<'a> = (
    &'a NavmeshRecord,
    &'a RecordVersion,
    Option<&'a RecordVersion>,
);

/// What each record copies, and the masters all of them need: every
/// plugin whose versions are copied and those they refer to, in load
/// order. Records that cannot be copied go into `refused`.
fn plan<'a>(
    found: &'a Found,
    plugins: &[(String, PathBuf)],
    records: &'a [NavmeshRecord],
    refused: &mut Vec<(RecordId, PathRefusal)>,
) -> (Vec<Planned<'a>>, Vec<String>) {
    let name_of = |plugin: usize| plugins.get(plugin).map_or("", |(name, _)| name.as_str());
    let position = |name: &str| {
        plugins
            .iter()
            .position(|(known, _)| known.eq_ignore_ascii_case(name))
            .unwrap_or(usize::MAX)
    };
    let mut planned = Vec::new();
    let mut needed: Vec<String> = Vec::new();
    for record in records {
        let Some((cell, world)) = versions_of(found, record) else {
            refused.push((record.navmesh.clone(), PathRefusal::NotFound));
            continue;
        };
        let mut unknown = None;
        let mut wanted = Vec::new();
        for version in std::iter::once(cell).chain(world) {
            let source = found
                .sources
                .get(&version.plugin)
                .cloned()
                .unwrap_or_default();
            wanted.push(name_of(version.plugin).to_owned());
            match moves::referenced(&version.record, &source, name_of(version.plugin)) {
                Ok(names) => wanted.extend(names),
                Err(field) => unknown = Some(field),
            }
        }
        if let Some(field) = unknown {
            refused.push((record.navmesh.clone(), PathRefusal::UnknownField { field }));
            continue;
        }
        wanted.extend(navmesh_plugins(record));
        for name in wanted {
            // As the load order spells it.
            let name = plugins
                .iter()
                .find(|(known, _)| known.eq_ignore_ascii_case(&name))
                .map_or(name, |(known, _)| known.clone());
            if !needed.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
                needed.push(name);
            }
        }
        planned.push((record, cell, world));
    }
    needed.sort_by_key(|name| position(name));
    (planned, needed)
}

/// A plugin that repairs nothing.
fn nothing(refused: Vec<(RecordId, PathRefusal)>) -> PathsPlugin {
    PathsPlugin {
        bytes: None,
        masters: Vec::new(),
        light: false,
        repaired: Vec::new(),
        refused,
    }
}

/// Why copying a cell or worldspace failed, as a repair's refusal.
fn refusal(reason: MoveRefusal) -> PathRefusal {
    match reason {
        MoveRefusal::UnknownField { field } => PathRefusal::UnknownField { field },
        MoveRefusal::UnsupportedGame => PathRefusal::UnsupportedGame,
        _ => PathRefusal::NotFound,
    }
}

/// The plugins a repaired mesh names: its own, the one it was copied
/// from, and those of the records its field refers to.
fn navmesh_plugins(record: &NavmeshRecord) -> Vec<String> {
    let mut names = vec![record.plugin.clone(), record.navmesh.plugin.clone()];
    if let Some(mut data) = record
        .fields
        .iter()
        .find(|field| &field.signature == b"NVNM")
        .and_then(|field| NavmeshData::read(&field.data))
    {
        for form_id in data.form_ids_mut() {
            let (origin, _) = resolve_form_id(*form_id, &record.masters, &record.plugin);
            names.push(origin.to_owned());
        }
    }
    names
}

/// The versions the game uses of the cell and, outside, the worldspace of
/// `record`.
fn versions_of<'a>(
    found: &'a Found,
    record: &NavmeshRecord,
) -> Option<(&'a RecordVersion, Option<&'a RecordVersion>)> {
    let cell = found.cells.get(&record.cell)?;
    let world = match &record.place {
        Place::Exterior { worldspace } => Some(found.worlds.get(worldspace)?),
        Place::Interior { .. } => None,
    };
    Some((cell, world))
}

/// Reads the versions of the cells and worldspaces of `records` from every
/// plugin. Outside, only the blocks of cells around them are read; inside,
/// the interior cells without their children.
fn find(plugins: &[(String, PathBuf)], records: &[NavmeshRecord]) -> Found {
    let mut found = Found::default();
    let points: Vec<(i32, i32)> = records.iter().filter_map(|record| record.grid).collect();
    let inside = records
        .iter()
        .any(|record| matches!(record.place, Place::Interior { .. }));
    let near = |group: GroupLabel| moves::near_points(group, &points);
    let no_children = |group: GroupLabel| !CHILDREN.contains(&group.group_type);
    for (index, (name, path)) in plugins.iter().enumerate() {
        if !points.is_empty() {
            moves::read_into(&mut found, index, name, path, &[*b"WRLD"], &near, &[]);
        }
        if inside {
            moves::read_into(
                &mut found,
                index,
                name,
                path,
                &[*b"CELL"],
                &no_children,
                &[],
            );
        }
    }
    // Only what the records are in is kept.
    let wanted_cells: BTreeSet<&RecordId> = records.iter().map(|record| &record.cell).collect();
    found.cells.retain(|id, _| wanted_cells.contains(id));
    found
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::Cursor;

    use mallet_formats_bethesda::plugin;

    use super::*;
    use crate::navmesh_records::tests::{around, setup};
    use crate::navmesh_records::{Repaired, repaired_records};
    use crate::walkways::read_plugins;
    use mallet_core::facts::navmeshes::Beyond::Nothing;

    /// The docks' mesh with its second triangle taken out.
    fn records(plugins: &[(String, PathBuf)]) -> Vec<NavmeshRecord> {
        let near = read_plugins(plugins, &around());
        let before = near
            .navmeshes
            .iter()
            .find(|shape| shape.used)
            .unwrap()
            .clone();
        let mut after = before.clone();
        after.edges[0][2] = Nothing;
        after.edges[1] = [Nothing, Nothing, Nothing];
        let removed = BTreeSet::from([1]);
        let water = BTreeSet::new();
        repaired_records(
            plugins,
            &around(),
            &[Repaired {
                before: &before,
                after: &after,
                removed: &removed,
                water: &water,
            }],
        )
    }

    #[test]
    fn the_repaired_mesh_is_written_into_its_cell_after_its_masters() {
        let (_directory, plugins) = setup();
        let records = records(&plugins);
        assert_eq!(records.len(), 1);
        let built = build(Game::SkyrimSe, &plugins, &records, &|_, _| None).unwrap();
        assert_eq!(built.refused, Vec::new());
        assert_eq!(
            built.masters,
            vec!["Skyrim.esm".to_owned(), "Docks.esp".to_owned()]
        );
        assert!(built.light);
        assert_eq!(built.repaired, vec![records[0].navmesh.clone()]);

        // The plugin reads back: the worldspace, the cell, and the mesh in
        // the cell's children, as the game's own mesh.
        let bytes = built.bytes.unwrap();
        let read = plugin::read_request(
            &mut Cursor::new(bytes),
            &plugin::Request {
                flat: &[],
                nested: &[*b"WRLD"],
                nested_records: &[*b"WRLD", *b"CELL", *b"NAVM"],
                nested_overrides: &[],
            },
        )
        .unwrap();
        assert_eq!(read.header.masters, built.masters);
        let kinds: Vec<_> = read.records.iter().map(|record| record.signature).collect();
        assert_eq!(kinds, vec![*b"WRLD", *b"CELL", *b"NAVM"]);
        let navmesh = read.records.last().unwrap();
        assert_eq!(navmesh.form_id, 0x0000_0210);
        let geometry =
            mallet_formats_bethesda::navmesh::geometry(navmesh.first(b"NVNM").unwrap()).unwrap();
        assert_eq!(
            geometry.edges[1],
            [mallet_formats_bethesda::navmesh::Beyond::Nothing; 3]
        );
    }

    #[test]
    fn other_games_and_missing_cells_are_refused() {
        let (_directory, plugins) = setup();
        let records = records(&plugins);
        let built = build(Game::Fallout4, &plugins, &records, &|_, _| None).unwrap();
        assert_eq!(built.bytes, None);
        assert_eq!(built.refused[0].1, PathRefusal::UnsupportedGame);

        let mut lost = records.clone();
        lost[0].cell.local_id = 0x999;
        let built = build(Game::SkyrimSe, &plugins, &lost, &|_, _| None).unwrap();
        assert_eq!(built.bytes, None);
        assert_eq!(built.refused[0].1, PathRefusal::NotFound);
    }
}
