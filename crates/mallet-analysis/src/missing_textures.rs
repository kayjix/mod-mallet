//! Loose meshes that name textures nothing in the game provides.
//!
//! The game then draws the surface in a glaring purple, or dark where a
//! normal map is missing. A texture counts as provided when it lies as a
//! loose file or in an archive the game loads. Reported per mod, since the
//! meshes of one mod usually miss textures for the same reason: a part of
//! the mod that was not installed, or a texture mod it was made to go with.

use std::collections::{BTreeMap, BTreeSet};

use mallet_core::{Finding, GamePath, ModRef, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many textures a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Reports loose meshes whose textures are missing.
pub struct MissingTextures;

/// What is missing for the meshes of one mod.
#[derive(Default)]
struct Missing<'a> {
    meshes: BTreeSet<&'a GamePath>,
    textures: BTreeSet<&'a GamePath>,
}

impl Analysis for MissingTextures {
    fn id(&self) -> &'static str {
        "missing-textures"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mut by_mod: BTreeMap<Option<ModRef>, Missing<'_>> = BTreeMap::new();
        for mesh in &context.facts.mesh_textures {
            let missing: Vec<&GamePath> = mesh
                .textures
                .iter()
                .filter(|texture| context.assets.source(texture).is_none())
                .collect();
            if missing.is_empty() {
                continue;
            }
            let entry = by_mod
                .entry(context.mods.provider_of(&mesh.path))
                .or_default();
            entry.meshes.insert(&mesh.path);
            entry.textures.extend(missing);
        }

        by_mod
            .into_iter()
            .filter_map(|(provider, missing)| {
                let subject = match &provider {
                    Some(mod_ref) => Subject::Mod {
                        source: mod_ref.source.clone(),
                        id: mod_ref.id.clone(),
                    },
                    None => Subject::File {
                        path: (*missing.meshes.first()?).clone(),
                    },
                };
                let count = |items: usize| i64::try_from(items).unwrap_or(i64::MAX);
                Some(
                    Finding::new(&kinds::MESHES_TEXTURE_MISSING, subject)
                        .with_arg("meshes", count(missing.meshes.len()))
                        .with_arg("count", count(missing.textures.len()))
                        .with_arg(
                            "examples",
                            examples(
                                missing.textures.iter().map(|path| path.as_str()),
                                EXAMPLE_LIMIT,
                            ),
                        )
                        .with_provider(provider),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::{ArchiveFacts, MeshTextures};
    use mallet_core::{ArgValue, Confidence, Facts, NoModSource, Severity};

    use super::*;
    use crate::assets::testing::Mods;
    use crate::run_all;

    fn mesh(path: &str, textures: &[&str]) -> MeshTextures {
        MeshTextures {
            path: GamePath::new(path),
            textures: textures
                .iter()
                .map(|texture| GamePath::new(texture))
                .collect(),
        }
    }

    fn findings(facts: &Facts, mods: &dyn mallet_core::ModSource) -> Vec<Finding> {
        run_all(&Context::new(facts, mods))
            .into_iter()
            .filter(|finding| finding.kind == "meshes.texture-missing")
            .collect()
    }

    #[test]
    fn textures_that_are_neither_loose_nor_packed_are_missing() {
        let facts = Facts {
            loose_files: vec![GamePath::new("data/textures/house.dds")],
            archives: vec![ArchiveFacts {
                path: GamePath::new("data/houses.bsa"),
                files: vec![GamePath::new("data/textures/house_n.dds")],
            }],
            mesh_textures: vec![
                mesh(
                    "data/meshes/house.nif",
                    &["data/textures/house.dds", "data/textures/house_n.dds"],
                ),
                mesh(
                    "data/meshes/shed.nif",
                    &["data/textures/shed.dds", "data/textures/house_n.dds"],
                ),
                mesh("data/meshes/barn.nif", &["data/textures/shed.dds"]),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            ("data/meshes/shed.nif", "Farm"),
            ("data/meshes/barn.nif", "Farm"),
        ]);
        let found = findings(&facts, &mods);

        let [finding] = found.as_slice() else {
            panic!("expected one finding, got {found:?}");
        };
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(finding.id.as_str(), "meshes.texture-missing:mod:test:Farm");
        assert_eq!(finding.args["meshes"], ArgValue::Number(2));
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("data/textures/shed.dds")
        );
    }

    #[test]
    fn meshes_whose_textures_are_all_there_are_not_reported() {
        let facts = Facts {
            loose_files: vec![GamePath::new("data/textures/house.dds")],
            mesh_textures: vec![mesh("data/meshes/house.nif", &["data/textures/house.dds"])],
            ..Facts::default()
        };
        assert_eq!(findings(&facts, &NoModSource), Vec::new());
    }
}
