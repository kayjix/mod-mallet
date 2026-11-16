//! What a plugin says about the game's characters.
//!
//! A character (an `NPC_` record) describes a face through a handful of
//! fields: the race, the head parts, hair colour, face texture, and the
//! sliders and tints of the character editor. The game does not build faces
//! from these fields while playing. It loads a face mesh and a tint texture
//! that were generated from them in advance, the `FaceGen` files. When the
//! record that wins in the load order describes a different face than those
//! files were generated from, the game shows the well-known dark face.
//!
//! [`npc_facts`] extracts the face-related fields in a form that compares
//! equal exactly when two plugins describe the same face.

use mallet_core::facts::{NpcFacts, RecordId};
use mallet_formats_bethesda::plugin::{self, Plugin, Record, Signature};

/// The record type of characters.
pub const NPC: Signature = *b"NPC_";

/// Fields that refer to another record: race, head part, hair colour, face
/// texture set.
const REFERENCES: [&Signature; 4] = [b"RNAM", b"PNAM", b"HCLF", b"FTST"];

/// Fields whose bytes describe the face directly: texture lighting, face
/// morphs, face parts, and the four fields of each tint layer.
const VALUES: [&Signature; 7] = [
    b"QNAM", b"NAM9", b"NAMA", b"TINI", b"TINC", b"TINV", b"TIAS",
];

/// Offset of the template flags inside the `ACBS` field.
const TEMPLATE_FLAGS_OFFSET: usize = 18;

/// Template flag "use traits": the character takes its looks from its
/// template.
const TEMPLATE_USE_TRAITS: u16 = 0x0001;

/// The characters a plugin defines or changes.
///
/// `plugin_name` is the plugin's file name; `plugin` must have been read
/// with [`NPC`] among the wanted record types.
pub fn npc_facts(plugin_name: &str, plugin: &Plugin) -> Vec<NpcFacts> {
    plugin
        .records
        .iter()
        .filter(|record| record.signature == NPC)
        .map(|record| NpcFacts {
            id: record_id(record.form_id, &plugin.header.masters, plugin_name),
            editor_id: record
                .first(b"EDID")
                .map(plugin::zstring)
                .filter(|name| !name.is_empty()),
            inherits_appearance: inherits_appearance(record),
            appearance: appearance(record, &plugin.header.masters, plugin_name),
        })
        .collect()
}

fn record_id(form_id: u32, masters: &[String], plugin_name: &str) -> RecordId {
    let (origin, local_id) = plugin::resolve_form_id(form_id, masters, plugin_name);
    RecordId {
        plugin: origin.to_lowercase(),
        local_id,
    }
}

/// Whether the record has a template and is flagged to take its traits,
/// which include the face, from it.
fn inherits_appearance(record: &Record) -> bool {
    let has_template = record
        .first(b"TPLT")
        .is_some_and(|template| template.iter().any(|&byte| byte != 0));
    let flags = record
        .first(b"ACBS")
        .and_then(|data| data.get(TEMPLATE_FLAGS_OFFSET..TEMPLATE_FLAGS_OFFSET + 2))
        .and_then(|flags| flags.try_into().ok())
        .map_or(0, u16::from_le_bytes);
    has_template && flags & TEMPLATE_USE_TRAITS != 0
}

/// The face-related fields in a canonical form.
///
/// References are written as "defining plugin, local number" instead of the
/// raw form ID, because the raw number depends on the masters of the plugin
/// it was read from. Head parts are sorted, because their order in a record
/// does not matter to the game.
fn appearance(record: &Record, masters: &[String], plugin_name: &str) -> Vec<u8> {
    let mut references: Vec<Vec<u8>> = Vec::new();
    let mut values: Vec<u8> = Vec::new();

    for subrecord in &record.subrecords {
        if REFERENCES.contains(&&subrecord.signature) {
            let Some(form_id) = subrecord
                .data
                .get(..4)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u32::from_le_bytes)
            else {
                continue;
            };
            let id = record_id(form_id, masters, plugin_name);
            let mut entry = subrecord.signature.to_vec();
            entry.extend(id.plugin.as_bytes());
            entry.push(0);
            entry.extend(id.local_id.to_le_bytes());
            references.push(entry);
        } else if VALUES.contains(&&subrecord.signature) {
            values.extend(subrecord.signature);
            values.extend(
                u32::try_from(subrecord.data.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            values.extend(&subrecord.data);
        }
    }

    references.sort();
    let mut canonical: Vec<u8> = references.into_iter().flatten().collect();
    canonical.extend(values);
    canonical
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};

    use super::*;

    /// An `ACBS` field with the given template flags.
    fn acbs(template_flags: u16) -> Vec<u8> {
        let mut data = vec![0u8; 24];
        data[TEMPLATE_FLAGS_OFFSET..TEMPLATE_FLAGS_OFFSET + 2]
            .copy_from_slice(&template_flags.to_le_bytes());
        data
    }

    fn read(masters: &[&str], records: Vec<RecordSpec>, name: &str) -> Vec<NpcFacts> {
        let bytes = fixture::plugin(masters, 0, &[(NPC, records)]);
        let plugin = plugin::read(&mut Cursor::new(bytes), &[NPC]).unwrap();
        npc_facts(name, &plugin)
    }

    fn form(id: u32) -> Vec<u8> {
        id.to_le_bytes().to_vec()
    }

    #[test]
    fn identifies_characters_by_their_defining_plugin() {
        let npcs = read(
            &["Skyrim.esm"],
            vec![
                RecordSpec::new(NPC, 0x0001_3BA1).with(*b"EDID", fixture::zstring("Uthgerd")),
                RecordSpec::new(NPC, 0x0100_0800),
            ],
            "Faces.esp",
        );
        assert_eq!(
            npcs[0].id,
            RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id: 0x01_3BA1
            }
        );
        assert_eq!(npcs[0].editor_id.as_deref(), Some("Uthgerd"));
        assert_eq!(
            npcs[1].id,
            RecordId {
                plugin: "faces.esp".to_owned(),
                local_id: 0x800
            }
        );
        assert_eq!(npcs[1].editor_id, None);
    }

    #[test]
    fn the_same_face_compares_equal_across_plugins_with_different_masters() {
        // Both plugins give the character the head part 0x900 of Hair.esp.
        // Hair.esp is master 1 in the first plugin and master 2 in the
        // second, so the raw form IDs differ.
        let first = read(
            &["Skyrim.esm", "Hair.esp"],
            vec![
                RecordSpec::new(NPC, 0x0001_3BA1)
                    .with(*b"PNAM", form(0x0100_0900))
                    .with(*b"PNAM", form(0x0000_1234))
                    .with(*b"QNAM", vec![1; 12]),
            ],
            "First.esp",
        );
        let second = read(
            &["Skyrim.esm", "Update.esm", "Hair.esp"],
            vec![
                RecordSpec::new(NPC, 0x0001_3BA1)
                    // Same head parts in another order.
                    .with(*b"PNAM", form(0x0000_1234))
                    .with(*b"PNAM", form(0x0200_0900))
                    .with(*b"QNAM", vec![1; 12])
                    // A field that has nothing to do with the face.
                    .with(*b"FULL", vec![9, 9, 9, 9]),
            ],
            "Second.esp",
        );
        assert_eq!(first[0].appearance, second[0].appearance);
    }

    #[test]
    fn a_different_head_part_is_a_different_face() {
        let with = |head_part: u32| {
            read(
                &["Skyrim.esm"],
                vec![RecordSpec::new(NPC, 0x0001_3BA1).with(*b"PNAM", form(head_part))],
                "Faces.esp",
            )
        };
        assert_ne!(with(0x1234)[0].appearance, with(0x1235)[0].appearance);
    }

    #[test]
    fn different_tints_are_a_different_face() {
        let with = |tint: u8| {
            read(
                &["Skyrim.esm"],
                vec![RecordSpec::new(NPC, 0x0001_3BA1).with(*b"TINC", vec![tint, 0, 0, 0])],
                "Faces.esp",
            )
        };
        assert_ne!(with(1)[0].appearance, with(2)[0].appearance);
    }

    #[test]
    fn a_character_that_uses_its_templates_traits_inherits_its_face() {
        let npcs = read(
            &["Skyrim.esm"],
            vec![
                RecordSpec::new(NPC, 0x801)
                    .with(*b"ACBS", acbs(TEMPLATE_USE_TRAITS))
                    .with(*b"TPLT", form(0x0001_0000)),
                // The flag without a template means nothing.
                RecordSpec::new(NPC, 0x802).with(*b"ACBS", acbs(TEMPLATE_USE_TRAITS)),
                // A template used for other things than traits.
                RecordSpec::new(NPC, 0x803)
                    .with(*b"ACBS", acbs(0x0002))
                    .with(*b"TPLT", form(0x0001_0000)),
            ],
            "Faces.esp",
        );
        let inherits: Vec<_> = npcs.iter().map(|npc| npc.inherits_appearance).collect();
        assert_eq!(inherits, [true, false, false]);
    }

    #[test]
    fn damaged_fields_are_ignored() {
        let npcs = read(
            &["Skyrim.esm"],
            vec![
                RecordSpec::new(NPC, 0x801)
                    .with(*b"PNAM", vec![1, 2]) // too short for a form ID
                    .with(*b"ACBS", vec![0; 4]), // too short for the flags
            ],
            "Faces.esp",
        );
        assert_eq!(npcs[0].appearance, Vec::<u8>::new());
        assert!(!npcs[0].inherits_appearance);
    }
}
