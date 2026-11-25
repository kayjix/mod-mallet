//! Everything placed in one interior cell, found by its editor ID, in the
//! version the game uses (ADR 0042).
//!
//! The interior cells of every plugin are listed once, without unpacking
//! what is placed in them; looking at a cell then reads only that cell's
//! children, and only from the plugins that have a version of it.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, GroupLabel, Record};

use super::{AreaObject, AreaReader, area_object};
use crate::cells::CHILDREN;
use crate::records::{editor_id, record_id};

/// An interior cell as the plugins have it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Interior {
    /// The cell, by the plugin that made it and its number there.
    cell: RecordId,
    /// Each plugin with a version of the cell, by its place in the load
    /// order, and the cell's form ID as that plugin writes it.
    versions: Vec<(usize, u32)>,
}

/// A cell as listed: the editor ID of its latest version, if any, and
/// each plugin with a version of it with the form ID it writes.
type Listed = (Option<String>, Vec<(usize, u32)>);

impl AreaReader {
    /// The interior cell whose editor ID is `cell` (case does not matter),
    /// by the plugin that made it; `None` if no plugin has such a cell.
    pub fn interior_cell(&mut self, cell: &str) -> Option<RecordId> {
        if self.interiors.is_none() {
            self.interiors = Some(self.list_interiors());
        }
        self.interiors
            .as_ref()?
            .get(&cell.to_ascii_lowercase())
            .map(|interior| interior.cell.clone())
    }

    /// The objects placed in the interior cell whose editor ID is `cell`
    /// (case does not matter), each in the version the game uses: the
    /// cell's persistent and temporary children. Empty if no plugin has
    /// such a cell. The first call lists the interior cells of every
    /// plugin.
    pub fn interior(&mut self, cell: &str) -> Vec<AreaObject> {
        if self.interiors.is_none() {
            self.interiors = Some(self.list_interiors());
        }
        let Some(interior) = self
            .interiors
            .as_ref()
            .and_then(|interiors| interiors.get(&cell.to_ascii_lowercase()))
            .cloned()
        else {
            return Vec::new();
        };
        let request = plugin::Request {
            flat: &[],
            nested: &[*b"CELL"],
            nested_records: &[*b"REFR"],
            nested_overrides: &[],
        };
        let mut found: HashMap<RecordId, Option<AreaObject>> = HashMap::new();
        for (index, form_id) in interior.versions {
            let Some((name, path)) = self.plugins.get(index) else {
                continue;
            };
            let descend =
                |group: GroupLabel| !CHILDREN.contains(&group.group_type) || group.label == form_id;
            let Ok(file) = File::open(path) else {
                continue;
            };
            let Ok(read) =
                plugin::read_request_where(&mut BufReader::new(file), &request, &descend)
            else {
                continue;
            };
            let masters = &read.header.masters;
            for record in read
                .records
                .iter()
                .filter(|record| in_cell(record, form_id))
            {
                let id = record_id(record.form_id, masters, name);
                found.insert(id.clone(), area_object(record, masters, name, id));
            }
        }
        let mut objects: Vec<AreaObject> = found.into_values().flatten().collect();
        objects.sort_by(|a, b| a.id.cmp(&b.id));
        objects
    }

    /// The interior cells of every plugin with an editor ID, by that ID in
    /// lower case as the version the game uses spells it.
    fn list_interiors(&self) -> HashMap<String, Interior> {
        let request = plugin::Request {
            flat: &[],
            nested: &[*b"CELL"],
            nested_records: &[*b"CELL"],
            nested_overrides: &[],
        };
        // Nothing placed in a cell is needed to list it.
        let descend = |group: GroupLabel| !CHILDREN.contains(&group.group_type);
        let mut cells: HashMap<RecordId, Listed> = HashMap::new();
        for (index, (name, path)) in self.plugins.iter().enumerate() {
            let Ok(file) = File::open(path) else {
                continue;
            };
            let Ok(read) =
                plugin::read_request_where(&mut BufReader::new(file), &request, &descend)
            else {
                continue;
            };
            let masters = &read.header.masters;
            for record in read
                .records
                .iter()
                .filter(|record| &record.signature == b"CELL")
            {
                let entry = cells
                    .entry(record_id(record.form_id, masters, name))
                    .or_default();
                // The later version's name wins, as everything else does.
                entry.0 = editor_id(record).or(entry.0.take());
                entry.1.push((index, record.form_id));
            }
        }
        index_by_name(cells)
    }
}

/// The cells by their editor ID in lower case. Where two cells share one,
/// the one that comes first by its record keeps it.
fn index_by_name(cells: HashMap<RecordId, Listed>) -> HashMap<String, Interior> {
    let mut sorted: Vec<(RecordId, Listed)> = cells.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut interiors = HashMap::new();
    for (cell, (name, versions)) in sorted {
        if let Some(name) = name.filter(|name| !name.is_empty()) {
            interiors
                .entry(name.to_ascii_lowercase())
                .or_insert(Interior { cell, versions });
        }
    }
    interiors
}

/// Whether `record` lies in the children of the cell with form ID
/// `form_id`, as its plugin writes it.
fn in_cell(record: &Record, form_id: u32) -> bool {
    &record.signature == b"REFR"
        && record
            .groups
            .iter()
            .any(|group| CHILDREN.contains(&group.group_type) && group.label == form_id)
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::plugin::group_type;

    use super::*;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    #[test]
    fn cells_are_found_by_name_whatever_its_case() {
        let mut cells = HashMap::new();
        cells.insert(
            id("skyrim.esm", 0x10),
            (
                Some("WhiterunBanneredMare".to_owned()),
                vec![(0, 0x10), (3, 0x10)],
            ),
        );
        cells.insert(id("skyrim.esm", 0x11), (None, vec![(0, 0x11)]));
        let interiors = index_by_name(cells);
        assert_eq!(interiors.len(), 1);
        let found = interiors.get("whiterunbanneredmare").unwrap();
        assert_eq!(found.cell, id("skyrim.esm", 0x10));
        assert_eq!(found.versions, vec![(0, 0x10), (3, 0x10)]);
    }

    #[test]
    fn a_shared_name_stays_with_the_first_cell() {
        let mut cells = HashMap::new();
        cells.insert(
            id("b.esp", 0x20),
            (Some("Cave".to_owned()), vec![(2, 0x0100_0020)]),
        );
        cells.insert(
            id("a.esp", 0x20),
            (Some("cave".to_owned()), vec![(1, 0x0100_0020)]),
        );
        let interiors = index_by_name(cells);
        assert_eq!(interiors.get("cave").unwrap().cell, id("a.esp", 0x20));
    }

    #[test]
    fn only_references_in_the_cells_children_count() {
        let reference = |groups: Vec<GroupLabel>| Record {
            signature: *b"REFR",
            form_id: 1,
            flags: 0,
            subrecords: Vec::new(),
            groups,
        };
        let child = |group_type: u32, label: u32| GroupLabel { group_type, label };
        assert!(in_cell(
            &reference(vec![
                child(group_type::INTERIOR_BLOCK, 3),
                child(group_type::CELL_CHILDREN, 0x10),
                child(group_type::CELL_TEMPORARY_CHILDREN, 0x10),
            ]),
            0x10
        ));
        assert!(in_cell(
            &reference(vec![child(group_type::CELL_PERSISTENT_CHILDREN, 0x10)]),
            0x10
        ));
        assert!(!in_cell(
            &reference(vec![child(group_type::CELL_CHILDREN, 0x11)]),
            0x10
        ));
        // An interior block labelled like the cell is no child of it.
        assert!(!in_cell(
            &reference(vec![child(group_type::INTERIOR_BLOCK, 0x10)]),
            0x10
        ));
    }
}
