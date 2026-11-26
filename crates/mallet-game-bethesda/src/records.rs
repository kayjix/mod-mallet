//! What every reader of plugin records needs: who a record is, what it is
//! called, and the flags that say whether it counts.
//!
//! The readers of objects, places, doors, markers, navigation meshes and
//! the catalogue all turn form IDs into [`RecordId`]s and read the same
//! few fields. They do it here, so that every reader agrees on, for
//! example, how a plugin's name is spelt in an identity.

use std::collections::HashMap;

use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, Record};

/// Record flag: the record is deleted.
pub(crate) const FLAG_DELETED: u32 = 0x0000_0020;

/// Record flag of references: the object is switched off when the game
/// starts and only appears if a script switches it on.
pub(crate) const FLAG_INITIALLY_DISABLED: u32 = 0x0000_0800;

/// Record flag of references: the object is persistent.
pub(crate) const FLAG_PERSISTENT: u32 = 0x0000_0400;

/// Record flag of references: the object is part of the distant view.
pub(crate) const FLAG_VISIBLE_WHEN_DISTANT: u32 = 0x0000_8000;

/// Positions farther out than this are not real places. The world is a
/// few hundred thousand units wide.
pub(crate) const MAX_COORDINATE: f32 = 1.0e7;

/// The identity of the record that `form_id`, read from the plugin
/// `plugin_name` with `masters`, names: the plugin that defines it, in
/// lower case, and its number there.
pub(crate) fn record_id(form_id: u32, masters: &[String], plugin_name: &str) -> RecordId {
    let (origin, local_id) = plugin::resolve_form_id(form_id, masters, plugin_name);
    RecordId::new(origin, local_id)
}

/// The internal name of a record (`EDID`), if it has a non-empty one.
pub(crate) fn editor_id(record: &Record) -> Option<String> {
    record
        .first(b"EDID")
        .map(plugin::zstring)
        .filter(|name| !name.is_empty())
}

/// The name players see (`FULL`) where the plugin spells it out; `None`
/// for a localized plugin, which numbers it in a string table instead.
pub(crate) fn spelt_name(record: &Record, localized: bool) -> Option<String> {
    (!localized)
        .then(|| record.first(b"FULL").map(plugin::zstring))
        .flatten()
        .filter(|name| !name.is_empty())
}

/// The name players see (`FULL`), looked up in `names`, the string table
/// by number, for a localized plugin. Untrimmed, and possibly empty.
///
/// The names of Fallout 4's object templates, between `OBTE` and `STOP`,
/// are not the thing's.
pub(crate) fn full_name(
    record: &Record,
    localized: bool,
    names: &HashMap<u32, String>,
) -> Option<String> {
    let mut in_templates = false;
    let full = record.subrecords.iter().find_map(|field| {
        match &field.signature {
            b"OBTE" => in_templates = true,
            b"STOP" => in_templates = false,
            b"FULL" if !in_templates => return Some(field.data.as_slice()),
            _ => {}
        }
        None
    });
    full.and_then(|data| {
        if localized {
            let number = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
            names.get(&number).cloned()
        } else {
            Some(plugin::zstring(data))
        }
    })
}

/// The little-endian numbers in a field, such as a reference's position
/// and rotation in `DATA`. Bytes past the last whole number are ignored.
pub(crate) fn floats(data: &[u8]) -> Vec<f32> {
    data.as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect()
}

/// A coordinate rounded to a whole number, or `None` if it is not one a
/// real place could have.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn whole(value: f32) -> Option<i32> {
    // The range check makes the conversion exact up to rounding.
    (value.is_finite() && value.abs() <= MAX_COORDINATE).then(|| value.round() as i32)
}

/// The model of a record (`MODL`), as a path below the data directory.
pub(crate) fn model_path(record: &Record) -> Option<String> {
    let model = record.first(b"MODL").map(plugin::zstring)?;
    let model = model.replace('\\', "/").to_ascii_lowercase();
    if model.is_empty() {
        return None;
    }
    Some(if model.starts_with("meshes/") {
        model
    } else {
        format!("meshes/{model}")
    })
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::plugin::Subrecord;
    use proptest::prelude::*;

    use super::*;

    fn record(fields: Vec<([u8; 4], Vec<u8>)>) -> Record {
        Record {
            signature: *b"STAT",
            flags: 0,
            form_id: 0x0100_0800,
            groups: Vec::new(),
            subrecords: fields
                .into_iter()
                .map(|(signature, data)| Subrecord { signature, data })
                .collect(),
        }
    }

    #[test]
    fn form_ids_name_the_defining_plugin_in_lower_case() {
        let masters = ["Skyrim.esm".to_owned()];
        let game = record_id(0x0000_0123, &masters, "Docks.esp");
        let own = record_id(0x0100_0800, &masters, "Docks.esp");
        assert_eq!((game.plugin.as_str(), game.local_id), ("skyrim.esm", 0x123));
        assert_eq!((own.plugin.as_str(), own.local_id), ("docks.esp", 0x800));
    }

    #[test]
    fn names_are_read_spelt_out_or_from_the_string_table() {
        let spelt = record(vec![
            (*b"EDID", b"Chair\0".to_vec()),
            (*b"FULL", b"Chair\0".to_vec()),
        ]);
        assert_eq!(editor_id(&spelt).as_deref(), Some("Chair"));
        assert_eq!(spelt_name(&spelt, false).as_deref(), Some("Chair"));
        assert_eq!(spelt_name(&spelt, true), None);

        let numbered = record(vec![(*b"FULL", 7u32.to_le_bytes().to_vec())]);
        let names = HashMap::from([(7, " Stuhl ".to_owned())]);
        assert_eq!(
            full_name(&numbered, true, &names).as_deref(),
            Some(" Stuhl ")
        );
        assert_eq!(editor_id(&record(vec![(*b"EDID", b"\0".to_vec())])), None);
    }

    #[test]
    fn models_lie_below_the_meshes_folder() {
        let model = |path: &str| {
            let mut data = path.as_bytes().to_vec();
            data.push(0);
            model_path(&record(vec![(*b"MODL", data)]))
        };
        assert_eq!(
            model(r"Clutter\Chair.NIF").as_deref(),
            Some("meshes/clutter/chair.nif")
        );
        assert_eq!(model("meshes/a.nif").as_deref(), Some("meshes/a.nif"));
        assert_eq!(model(""), None);
    }

    #[test]
    fn coordinates_out_of_the_world_are_not_whole() {
        assert_eq!(whole(12.6), Some(13));
        assert_eq!(whole(f32::NAN), None);
        assert_eq!(whole(2.0e7), None);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_fields(
            data in proptest::collection::vec(any::<u8>(), 0..64),
            form_id in any::<u32>(),
        ) {
            let fields = [*b"EDID", *b"FULL", *b"MODL"]
                .into_iter()
                .map(|signature| (signature, data.clone()))
                .collect();
            let record = record(fields);
            let _ = editor_id(&record);
            let _ = spelt_name(&record, false);
            let _ = full_name(&record, true, &HashMap::new());
            let _ = model_path(&record);
            let _ = floats(&data);
            let _ = record_id(form_id, &[], "a.esp");
        }
    }
}
