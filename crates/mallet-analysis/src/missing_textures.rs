//! Loose meshes that name textures nothing in the game provides.
//!
//! The game then draws the surface in a glaring purple, or dark where a
//! normal map is missing. A texture counts as provided when it lies as a
//! loose file or in an archive the game loads. Reported per mod, since the
//! meshes of one mod usually miss textures for the same reason: a part of
//! the mod that was not installed, or a texture mod it was made to go with.
//! Where a mod that is installed but switched off brings the textures, the
//! finding says so and offers to switch it on (ADR 0052).

use std::collections::{BTreeMap, BTreeSet};

use mallet_core::{Action, Finding, GamePath, ModRef, Subject, kinds};

use crate::{Analysis, Context, examples, is_used};

/// How many textures a finding names in its text.
const EXAMPLE_LIMIT: usize = 5;

/// How many textures the full list of a finding holds.
const LIST_LIMIT: usize = 500;

/// How many switched-off mods a finding offers to switch on.
const ENABLE_LIMIT: usize = 3;

/// Reports loose meshes whose textures are missing.
pub struct MissingTextures;

/// What is missing for the meshes of one mod: each texture, with the
/// meshes that name it.
#[derive(Default)]
struct Missing<'a> {
    meshes: BTreeSet<&'a GamePath>,
    textures: BTreeMap<&'a GamePath, BTreeSet<&'a GamePath>>,
}

impl Analysis for MissingTextures {
    fn id(&self) -> &'static str {
        "missing-textures"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mut by_mod: BTreeMap<Option<ModRef>, Missing<'_>> = BTreeMap::new();
        for mesh in &context.facts.mesh_textures {
            let archive = mesh.archive.as_ref();
            if !is_used(context, &mesh.path, archive) {
                continue;
            }
            let missing: Vec<&GamePath> = mesh
                .textures
                .iter()
                .filter(|texture| context.assets.source(texture).is_none())
                .collect();
            if missing.is_empty() {
                continue;
            }
            let entry = by_mod
                .entry(context.mods.provider_of(archive.unwrap_or(&mesh.path)))
                .or_default();
            entry.meshes.insert(&mesh.path);
            for texture in missing {
                entry
                    .textures
                    .entry(texture)
                    .or_default()
                    .insert(&mesh.path);
            }
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
                let switched_off = switched_off_with(context, &missing);
                let found: usize = switched_off.iter().map(|(_, found)| found).sum();
                let mut finding = Finding::new(&kinds::MESHES_TEXTURE_MISSING, subject)
                    .with_arg("meshes", count(missing.meshes.len()))
                    .with_arg("count", count(missing.textures.len()))
                    .with_arg(
                        "examples",
                        examples(
                            missing.textures.keys().map(|path| path.as_str()),
                            EXAMPLE_LIMIT,
                        ),
                    )
                    .with_arg("list", list(&missing))
                    .with_arg("offCount", count(found))
                    .with_arg(
                        "offMods",
                        switched_off
                            .iter()
                            .map(|(mod_ref, _)| mod_ref.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "),
                    )
                    .with_provider(provider);
                for (mod_ref, _) in switched_off.into_iter().take(ENABLE_LIMIT) {
                    finding = finding.with_action(Action::EnableMod { mod_ref });
                }
                Some(finding)
            })
            .collect()
    }
}

/// The switched-off mods that bring missing textures, with how many each
/// brings, the most first. A texture several of them bring counts for each.
fn switched_off_with(context: &Context<'_>, missing: &Missing<'_>) -> Vec<(ModRef, usize)> {
    let mut found: BTreeMap<ModRef, usize> = BTreeMap::new();
    for texture in missing.textures.keys() {
        for mod_ref in context.mods.switched_off_with(texture) {
            *found.entry(mod_ref).or_default() += 1;
        }
    }
    let mut found: Vec<(ModRef, usize)> = found.into_iter().collect();
    found.sort_by(|(a, a_count), (b, b_count)| b_count.cmp(a_count).then_with(|| a.cmp(b)));
    found
}

/// Every missing texture, one per line, with the meshes that name it after
/// a tab: `texture\tmesh, mesh`. Cut after [`LIST_LIMIT`] textures.
fn list(missing: &Missing<'_>) -> String {
    missing
        .textures
        .iter()
        .take(LIST_LIMIT)
        .map(|(texture, meshes)| {
            let meshes: Vec<&str> = meshes.iter().map(|mesh| mesh.as_str()).collect();
            format!("{}\t{}", texture.as_str(), meshes.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::{ArchiveFacts, MeshTextures};
    use mallet_core::{ArgValue, Confidence, Facts, ModSource, NoModSource, Severity};

    use super::*;
    use crate::assets::testing::Mods;
    use crate::run_all;

    fn mesh(path: &str, textures: &[&str]) -> MeshTextures {
        MeshTextures {
            path: GamePath::new(path),
            archive: None,
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
        assert_eq!(finding.confidence, Confidence::Hint);
        assert_eq!(finding.id.as_str(), "meshes.texture-missing:mod:test:Farm");
        assert_eq!(finding.args["meshes"], ArgValue::Number(2));
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("data/textures/shed.dds")
        );
    }

    /// A mod manager with Farm deployed and Farm Textures installed but
    /// switched off.
    struct WithSwitchedOff(Mods);

    impl ModSource for WithSwitchedOff {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            self.0.provider_of(path)
        }

        fn switched_off_with(&self, path: &GamePath) -> Vec<ModRef> {
            if path.as_str().starts_with("data/textures/farm/") {
                vec![crate::assets::testing::mod_ref("Farm Textures")]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn every_missing_texture_is_listed_with_its_meshes() {
        let facts = Facts {
            mesh_textures: vec![
                mesh("data/meshes/shed.nif", &["data/textures/shed.dds"]),
                mesh(
                    "data/meshes/barn.nif",
                    &["data/textures/shed.dds", "data/textures/barn.dds"],
                ),
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
        assert_eq!(
            finding.args["list"],
            ArgValue::from(
                "data/textures/barn.dds\tdata/meshes/barn.nif\n\
                 data/textures/shed.dds\tdata/meshes/barn.nif, data/meshes/shed.nif"
            )
        );
        assert_eq!(finding.args["offCount"], ArgValue::Number(0));
        assert_eq!(finding.actions, Vec::<Action>::new());
    }

    #[test]
    fn textures_a_switched_off_mod_brings_offer_to_switch_it_on() {
        let facts = Facts {
            mesh_textures: vec![mesh(
                "data/meshes/shed.nif",
                &[
                    "data/textures/farm/shed.dds",
                    "data/textures/farm/shed_n.dds",
                    "data/textures/sky.dds",
                ],
            )],
            ..Facts::default()
        };
        let mods = WithSwitchedOff(Mods(vec![("data/meshes/shed.nif", "Farm")]));
        let found = findings(&facts, &mods);
        let [finding] = found.as_slice() else {
            panic!("expected one finding, got {found:?}");
        };
        assert_eq!(finding.args["count"], ArgValue::Number(3));
        assert_eq!(finding.args["offCount"], ArgValue::Number(2));
        assert_eq!(finding.args["offMods"], ArgValue::from("Farm Textures"));
        assert_eq!(
            finding.actions,
            [Action::EnableMod {
                mod_ref: crate::assets::testing::mod_ref("Farm Textures")
            }]
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
