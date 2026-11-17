//! Meshes made for another game, or for another edition of this one.
//!
//! The meshes of one mod usually come from the same place, so they are
//! reported per mod and per game they were made for. Meshes that bend with
//! a skeleton are named first: they are the ones that make the game crash
//! when someone puts them on.

use std::collections::BTreeMap;

use mallet_core::{Finding, ModRef, Subject, kinds};

use crate::{Analysis, Context};

/// How many meshes a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Reports loose meshes in the format of another game or edition.
pub struct ForeignMeshes;

/// The game a variant of the mesh format belongs to, by its name.
fn made_for(bethesda_version: u32) -> Option<&'static str> {
    match bethesda_version {
        34 => Some("Fallout 3 / New Vegas"),
        83 => Some("Skyrim (2011)"),
        100 => Some("Skyrim Special Edition"),
        130 => Some("Fallout 4"),
        _ => None,
    }
}

impl Analysis for ForeignMeshes {
    fn id(&self) -> &'static str {
        "foreign-meshes"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        // By the mod that brings them and the game they were made for. A
        // `BTreeMap` keeps the order of the meshes stable.
        let mut groups: BTreeMap<(Option<ModRef>, &str), Vec<_>> = BTreeMap::new();
        for mesh in &context.facts.foreign_meshes {
            let Some(game) = made_for(mesh.bethesda_version) else {
                continue;
            };
            groups
                .entry((context.mods.provider_of(&mesh.path), game))
                .or_default()
                .push(mesh);
        }

        groups
            .into_iter()
            .filter_map(|((provider, game), mut meshes)| {
                meshes.sort_by_key(|mesh| (!mesh.skinned, mesh.path.clone()));
                let first = meshes.first()?;
                let subject = match &provider {
                    Some(mod_ref) => Subject::Mod {
                        source: mod_ref.source.clone(),
                        id: mod_ref.id.clone(),
                    },
                    None => Subject::File {
                        path: first.path.clone(),
                    },
                };
                let skinned = meshes.iter().filter(|mesh| mesh.skinned).count();
                let examples: Vec<&str> = meshes
                    .iter()
                    .take(EXAMPLE_LIMIT)
                    .map(|mesh| mesh.path.as_str())
                    .collect();
                Some(
                    Finding::new(&kinds::MESHES_MADE_FOR_OTHER_GAME, subject)
                        .with_arg("count", i64::try_from(meshes.len()).unwrap_or(i64::MAX))
                        .with_arg("skinned", i64::try_from(skinned).unwrap_or(i64::MAX))
                        .with_arg("madeFor", game)
                        .with_arg("examples", examples.join(", "))
                        .with_provider(provider),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::ForeignMesh;
    use mallet_core::{ArgValue, Confidence, Facts, GamePath, NoModSource, Severity};

    use super::*;
    use crate::assets::testing::Mods;
    use crate::run_all;

    fn mesh(path: &str, version: u32, skinned: bool) -> ForeignMesh {
        ForeignMesh {
            path: GamePath::new(path),
            bethesda_version: version,
            skinned,
        }
    }

    fn findings(facts: &Facts, mods: &dyn mallet_core::ModSource) -> Vec<Finding> {
        run_all(&Context::new(facts, mods))
            .into_iter()
            .filter(|finding| finding.kind == "meshes.made-for-other-game")
            .collect()
    }

    #[test]
    fn old_meshes_of_a_mod_are_reported_together_skinned_first() {
        let facts = Facts {
            foreign_meshes: vec![
                mesh("data/meshes/clutter/barrel.nif", 83, false),
                mesh("data/meshes/armor/mask.nif", 83, true),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            ("data/meshes/clutter/barrel.nif", "Masks"),
            ("data/meshes/armor/mask.nif", "Masks"),
        ]);
        let found = findings(&facts, &mods);

        let [finding] = found.as_slice() else {
            panic!("expected one finding, got {found:?}");
        };
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "meshes.made-for-other-game:mod:test:Masks"
        );
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(finding.args["skinned"], ArgValue::Number(1));
        assert_eq!(finding.args["madeFor"], ArgValue::from("Skyrim (2011)"));
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("data/meshes/armor/mask.nif, data/meshes/clutter/barrel.nif")
        );
    }

    #[test]
    fn meshes_of_no_known_mod_are_reported_by_file_and_unknown_variants_not_at_all() {
        let facts = Facts {
            foreign_meshes: vec![
                mesh("data/meshes/a.nif", 83, false),
                mesh("data/meshes/b.nif", 99, false),
            ],
            ..Facts::default()
        };
        let found = findings(&facts, &NoModSource);
        let [finding] = found.as_slice() else {
            panic!("expected one finding, got {found:?}");
        };
        assert_eq!(
            finding.id.as_str(),
            "meshes.made-for-other-game:file:data/meshes/a.nif"
        );
    }
}
