//! Characters whose face files do not match the record that wins.
//!
//! The game does not build a character's face while playing. It loads a
//! face mesh that was generated in advance from the character's record, and
//! compares the two. If the record that wins in the load order describes
//! other head parts than the mesh was generated from, the head is drawn
//! with the wrong colours: the "dark face" or "black face" bug.
//!
//! This happens when one mod provides the face files and a plugin of
//! another mod, loaded later, changes the same character. The analysis
//! finds it by joining three facts that no single tool shows together:
//!
//! 1. which source the game takes the face mesh from (loose file or
//!    archive, and whose),
//! 2. which plugin's version of the character wins,
//! 3. whether that version describes the same face as the version from the
//!    provider of the mesh.
//!
//! The result is *likely*, not certain: a plugin can change a character's
//! record in ways that look different on paper and the same in the game.

use std::collections::{BTreeMap, HashMap};

use mallet_core::facts::{NpcFacts, PluginFacts, RecordId};
use mallet_core::{Finding, GamePath, ModRef, Party, Subject, kinds};

use crate::{Analysis, AssetSource, Context, examples};

/// How many characters a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Where the game looks for face meshes, per defining plugin.
const FACE_MESH_DIR: &str = "data/meshes/actors/character/facegendata/facegeom";

/// Reports characters that will probably be drawn with a dark or
/// mismatched face.
pub struct NpcFaces;

impl Analysis for NpcFaces {
    fn id(&self) -> &'static str {
        "npc-faces"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let plugins = &context.facts.plugins;
        let providers: Vec<Option<ModRef>> = plugins
            .iter()
            .map(|plugin| context.mods.provider_of(&plugin.path))
            .collect();

        // Every version of every character, in load order.
        let mut versions: HashMap<&RecordId, Vec<(usize, &NpcFacts)>> = HashMap::new();
        for (index, plugin) in plugins.iter().enumerate() {
            for npc in &plugin.npcs {
                versions.entry(&npc.id).or_default().push((index, npc));
            }
        }

        // Mismatches, grouped by the two plugins involved.
        let mut mismatches: BTreeMap<(usize, usize), Vec<&NpcFacts>> = BTreeMap::new();
        for (id, versions) in &versions {
            let Some(&(winner_index, winner)) = versions.last() else {
                continue;
            };
            if winner.inherits_appearance {
                continue;
            }
            let Some(FaceFiles {
                provider: face_provider,
            }) = face_files(context, id)
            else {
                continue;
            };
            let provider_of = |index: usize| providers.get(index).and_then(Option::as_ref);
            if provider_of(winner_index) == face_provider.as_ref() {
                continue;
            }
            // The version of the character that the face was made for: the
            // last one from whoever provides the face mesh.
            let face_version = versions
                .iter()
                .rev()
                .find(|(index, _)| provider_of(*index) == face_provider.as_ref());
            if let Some(&(face_index, face)) = face_version
                && face.appearance != winner.appearance
            {
                mismatches
                    .entry((winner_index, face_index))
                    .or_default()
                    .push(winner);
            }
        }

        mismatches
            .into_iter()
            .filter_map(|((winner_index, face_index), npcs)| {
                let winner = plugins.get(winner_index)?;
                let face = plugins.get(face_index)?;
                let face_mod = providers.get(face_index).and_then(Option::as_ref);
                Some(finding(winner, face, face_mod, &npcs))
            })
            .collect()
    }
}

/// The face mesh the game uses for a character.
struct FaceFiles {
    /// The mod that provides the mesh, or `None` for the game itself and
    /// for files nobody manages.
    provider: Option<ModRef>,
}

/// The face mesh the game uses for the character `id`, or `None` if there
/// is no face mesh at all.
fn face_files(context: &Context<'_>, id: &RecordId) -> Option<FaceFiles> {
    let mesh = GamePath::new(&format!(
        "{FACE_MESH_DIR}/{}/{:08x}.nif",
        id.plugin, id.local_id
    ));
    let provided_by = match context.assets.source(&mesh)? {
        AssetSource::Loose => mesh,
        AssetSource::Archive(index) => context.facts.archives.get(index)?.path.clone(),
    };
    Some(FaceFiles {
        provider: context.mods.provider_of(&provided_by),
    })
}

fn finding(
    winner: &PluginFacts,
    face: &PluginFacts,
    face_mod: Option<&ModRef>,
    npcs: &[&NpcFacts],
) -> Finding {
    let names = npcs.iter().filter_map(|npc| npc.editor_id.as_deref());
    Finding::new(
        &kinds::NPCS_FACE_MISMATCH,
        Subject::Override {
            winner: Party::File {
                path: winner.path.clone(),
            },
            loser: Party::File {
                path: face.path.clone(),
            },
        },
    )
    .with_arg("count", i64::try_from(npcs.len()).unwrap_or(i64::MAX))
    .with_arg("examples", examples(names, EXAMPLE_LIMIT))
    .with_arg("winnerPlugin", winner.name.as_str())
    .with_arg("facePlugin", face.name.as_str())
    .with_arg(
        "faceOrigin",
        if face_mod.is_some() { "mod" } else { "game" },
    )
    .with_arg(
        "faceMod",
        face_mod.map_or("", |face_mod| face_mod.name.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::ArchiveFacts;
    use mallet_core::{ArgValue, Confidence, Facts, Severity};

    use super::*;
    use crate::assets::testing::Mods;
    use crate::run_all;

    const UTHGERD: u32 = 0x01_3BA1;
    const UTHGERD_MESH: &str =
        "data/meshes/actors/character/facegendata/facegeom/skyrim.esm/00013ba1.nif";

    fn npc(local_id: u32, name: &str, appearance: &[u8]) -> NpcFacts {
        NpcFacts {
            id: RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id,
            },
            editor_id: Some(name.to_owned()),
            inherits_appearance: false,
            appearance: appearance.to_vec(),
        }
    }

    fn plugin(name: &str, npcs: Vec<NpcFacts>) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: Vec::new(),
            npcs,
        }
    }

    /// The game, a face overhaul with loose face meshes, and a later plugin
    /// that changes the same character.
    fn overhaul_setup(later_appearance: &[u8]) -> (Facts, Mods) {
        let facts = Facts {
            loose_files: vec![GamePath::new(UTHGERD_MESH)],
            archives: vec![ArchiveFacts {
                path: GamePath::new("data/skyrim - meshes0.bsa"),
                files: vec![GamePath::new(UTHGERD_MESH)],
            }],
            plugins: vec![
                plugin("Skyrim.esm", vec![npc(UTHGERD, "Uthgerd", b"vanilla")]),
                plugin("Faces.esp", vec![npc(UTHGERD, "Uthgerd", b"overhaul")]),
                plugin(
                    "College.esp",
                    vec![npc(UTHGERD, "Uthgerd", later_appearance)],
                ),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            (UTHGERD_MESH, "Face Overhaul"),
            ("data/faces.esp", "Face Overhaul"),
            ("data/college.esp", "College Mod"),
        ]);
        (facts, mods)
    }

    #[test]
    fn a_later_plugin_with_another_face_than_the_face_files_is_reported() {
        let (facts, mods) = overhaul_setup(b"vanilla");
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "npcs.face-mismatch");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "npcs.face-mismatch:override:file:data/college.esp>file:data/faces.esp"
        );
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["examples"], ArgValue::from("Uthgerd"));
        assert_eq!(finding.args["winnerPlugin"], ArgValue::from("College.esp"));
        assert_eq!(finding.args["facePlugin"], ArgValue::from("Faces.esp"));
        assert_eq!(finding.args["faceOrigin"], ArgValue::from("mod"));
        assert_eq!(finding.args["faceMod"], ArgValue::from("Face Overhaul"));
    }

    #[test]
    fn a_later_plugin_that_keeps_the_overhauls_face_is_fine() {
        let (facts, mods) = overhaul_setup(b"overhaul");
        assert_eq!(run_all(&Context::new(&facts, &mods)), Vec::<Finding>::new());
    }

    #[test]
    fn the_overhaul_winning_is_fine() {
        let (mut facts, mods) = overhaul_setup(b"vanilla");
        // Load the overhaul last.
        facts.plugins.swap(1, 2);
        assert_eq!(run_all(&Context::new(&facts, &mods)), Vec::<Finding>::new());
    }

    #[test]
    fn a_plugin_that_changes_a_face_without_shipping_face_files_is_reported() {
        let facts = Facts {
            archives: vec![ArchiveFacts {
                path: GamePath::new("data/skyrim - meshes0.bsa"),
                files: vec![GamePath::new(UTHGERD_MESH)],
            }],
            plugins: vec![
                plugin("Skyrim.esm", vec![npc(UTHGERD, "Uthgerd", b"vanilla")]),
                plugin("Makeover.esp", vec![npc(UTHGERD, "Uthgerd", b"new look")]),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![("data/makeover.esp", "Makeover")]);
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.args["winnerPlugin"], ArgValue::from("Makeover.esp"));
        assert_eq!(finding.args["facePlugin"], ArgValue::from("Skyrim.esm"));
        assert_eq!(finding.args["faceOrigin"], ArgValue::from("game"));
        assert_eq!(finding.args["faceMod"], ArgValue::from(""));
    }

    #[test]
    fn characters_without_face_files_are_not_examined() {
        let (mut facts, mods) = overhaul_setup(b"vanilla");
        facts.loose_files.clear();
        facts.archives.clear();
        assert_eq!(run_all(&Context::new(&facts, &mods)), Vec::<Finding>::new());
    }

    #[test]
    fn characters_that_inherit_their_face_are_not_examined() {
        let (mut facts, mods) = overhaul_setup(b"vanilla");
        facts.plugins[2].npcs[0].inherits_appearance = true;
        assert_eq!(run_all(&Context::new(&facts, &mods)), Vec::<Finding>::new());
    }

    #[test]
    fn several_characters_between_the_same_two_plugins_are_one_finding() {
        let (mut facts, mods) = overhaul_setup(b"vanilla");
        let second = 0x01_3BA2;
        let second_mesh = UTHGERD_MESH.replace("00013ba1", "00013ba2");
        facts.loose_files.push(GamePath::new(&second_mesh));
        for (plugin, appearance) in
            facts
                .plugins
                .iter_mut()
                .zip([b"vanilla".as_slice(), b"overhaul", b"vanilla"])
        {
            plugin.npcs.push(npc(second, "Aela", appearance));
        }
        let mods = Mods(
            mods.0
                .into_iter()
                .chain([(
                    "data/meshes/actors/character/facegendata/facegeom/skyrim.esm/00013ba2.nif",
                    "Face Overhaul",
                )])
                .collect(),
        );
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(finding.args["examples"], ArgValue::from("Aela, Uthgerd"));
    }
}
