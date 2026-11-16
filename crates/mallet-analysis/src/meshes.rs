//! Replacement meshes that slip out from under a texture mod.
//!
//! A mesh names the textures it uses by path. A texture mod works by
//! providing new files under the paths the original meshes name. When
//! another mod replaces a mesh, and its version names other paths (usually
//! textures in that mod's own folder), the texture mod no longer reaches
//! the object. The object keeps one look while everything around it gets
//! the new one.
//!
//! The analysis looks at every loose mesh that replaces a mesh from an
//! archive and reports it when all of this is true:
//!
//! * the original mesh used a texture that the replacement no longer uses,
//! * that texture is provided by a mod, which is neither the mod of the
//!   replacement mesh nor the owner of the original mesh; in other words,
//!   somebody installed a retexture for it,
//! * the replacement uses a texture the original did not, and that texture
//!   comes from the same source as the replacement mesh.
//!
//! This is *likely* a visible mismatch, not certainly: how much it shows
//! depends on how different the two sets of textures are.

use std::collections::{BTreeMap, HashSet};

use mallet_core::facts::MeshOverride;
use mallet_core::{Finding, GamePath, ModRef, Party, Subject, kinds};

use crate::{Analysis, AssetSource, Context, examples};

/// How many mesh names a finding quotes.
const EXAMPLE_LIMIT: usize = 3;

/// Reports replacement meshes that no longer use textures a texture mod
/// replaces.
pub struct BypassedRetextures;

impl Analysis for BypassedRetextures {
    fn id(&self) -> &'static str {
        "bypassed-retextures"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        // Meshes, grouped by the mod of the mesh and the texture mod.
        let mut groups: BTreeMap<(Option<ModRef>, ModRef), Vec<&MeshOverride>> = BTreeMap::new();

        for mesh in &context.facts.mesh_overrides {
            let mesh_mod = context.mods.provider_of(&mesh.path);
            let original_owner = context.mods.provider_of(&mesh.replaced_in);
            let new: HashSet<&GamePath> = mesh.textures.iter().collect();
            let old: HashSet<&GamePath> = mesh.replaced_textures.iter().collect();

            let brings_own_textures = new
                .difference(&old)
                .any(|texture| provider(context, texture) == Some(mesh_mod.clone()));
            if !brings_own_textures {
                continue;
            }

            // Every texture mod whose work this mesh no longer shows.
            let bypassed: HashSet<ModRef> = old
                .difference(&new)
                .filter_map(|texture| provider(context, texture).flatten())
                .filter(|texture_mod| {
                    Some(texture_mod) != mesh_mod.as_ref()
                        && Some(texture_mod) != original_owner.as_ref()
                })
                .collect();
            for texture_mod in bypassed {
                groups
                    .entry((mesh_mod.clone(), texture_mod))
                    .or_default()
                    .push(mesh);
            }
        }

        groups
            .into_iter()
            .map(|((mesh_mod, texture_mod), meshes)| {
                finding(mesh_mod.as_ref(), &texture_mod, &meshes)
            })
            .collect()
    }
}

/// Who provides the texture the game uses under `path`: `Some(None)` for
/// the game itself or files nobody manages, `None` if nothing provides it.
#[allow(clippy::option_option)]
fn provider(context: &Context<'_>, path: &GamePath) -> Option<Option<ModRef>> {
    let provided_by = match context.assets.source(path)? {
        AssetSource::Loose => path.clone(),
        AssetSource::Archive(index) => context.facts.archives.get(index)?.path.clone(),
    };
    Some(context.mods.provider_of(&provided_by))
}

fn finding(mesh_mod: Option<&ModRef>, texture_mod: &ModRef, meshes: &[&MeshOverride]) -> Finding {
    // For the first mesh: the texture it brings, and the one the texture mod
    // replaced for the mesh it hides.
    // The first mesh with its texture and the one it bypasses, then the
    // other meshes, up to 40 files in all.
    let mut previews: Vec<GamePath> = meshes.first().map_or_else(Vec::new, |mesh| {
        std::iter::once(&mesh.path)
            .chain(mesh.textures.first())
            .chain(mesh.replaced_textures.first())
            .cloned()
            .collect()
    });
    previews.extend(meshes.iter().skip(1).take(37).map(|mesh| mesh.path.clone()));
    Finding::new(
        &kinds::MESHES_RETEXTURE_BYPASSED,
        Subject::Override {
            winner: Party::from_provider(mesh_mod),
            loser: Party::Mod {
                source: texture_mod.source.clone(),
                id: texture_mod.id.clone(),
            },
        },
    )
    .with_arg("count", i64::try_from(meshes.len()).unwrap_or(i64::MAX))
    .with_arg(
        "examples",
        examples(
            meshes.iter().map(|mesh| mesh.path.file_name()),
            EXAMPLE_LIMIT,
        ),
    )
    .with_arg(
        "meshOrigin",
        if mesh_mod.is_some() {
            "mod"
        } else {
            "unmanaged"
        },
    )
    .with_arg(
        "meshMod",
        mesh_mod.map_or("", |mesh_mod| mesh_mod.name.as_str()),
    )
    .with_arg("textureMod", texture_mod.name.as_str())
    .with_previews(previews)
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::ArchiveFacts;
    use mallet_core::{ArgValue, Confidence, Facts, Severity};

    use super::*;
    use crate::assets::testing::Mods;

    const WALL_MESH: &str = "data/meshes/architecture/wall.nif";
    const WALL_TEXTURE: &str = "data/textures/architecture/wall.dds";
    const WALL_NORMAL: &str = "data/textures/architecture/wall_n.dds";
    const OWN_TEXTURE: &str = "data/textures/meshmod/wall.dds";

    fn paths(paths: &[&str]) -> Vec<GamePath> {
        paths.iter().map(|path| GamePath::new(path)).collect()
    }

    /// The game's wall, a mesh mod that replaces the wall with a version
    /// that uses its own colour texture, and a texture mod that replaces
    /// the game's wall textures.
    fn setup() -> (Facts, Mods) {
        let facts = Facts {
            loose_files: paths(&[WALL_MESH, WALL_TEXTURE, WALL_NORMAL, OWN_TEXTURE]),
            archives: vec![ArchiveFacts {
                path: GamePath::new("data/skyrim - meshes0.bsa"),
                files: paths(&[WALL_MESH]),
            }],
            mesh_overrides: vec![MeshOverride {
                path: GamePath::new(WALL_MESH),
                textures: paths(&[OWN_TEXTURE, WALL_NORMAL]),
                replaced_in: GamePath::new("data/skyrim - meshes0.bsa"),
                replaced_textures: paths(&[WALL_TEXTURE, WALL_NORMAL]),
            }],
            ..Facts::default()
        };
        let mods = Mods(vec![
            (WALL_MESH, "Mesh Mod"),
            (OWN_TEXTURE, "Mesh Mod"),
            (WALL_TEXTURE, "Texture Mod"),
            (WALL_NORMAL, "Texture Mod"),
        ]);
        (facts, mods)
    }

    #[test]
    fn a_mesh_that_leaves_a_retextured_texture_for_its_own_is_reported() {
        let (facts, mods) = setup();
        let findings = BypassedRetextures.run(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "meshes.retexture-bypassed");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "meshes.retexture-bypassed:override:mod:test:Mesh Mod>mod:test:Texture Mod"
        );
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["examples"], ArgValue::from("wall.nif"));
        assert_eq!(finding.args["meshOrigin"], ArgValue::from("mod"));
        assert_eq!(finding.args["meshMod"], ArgValue::from("Mesh Mod"));
        assert_eq!(finding.args["textureMod"], ArgValue::from("Texture Mod"));
    }

    #[test]
    fn a_mesh_that_keeps_the_original_textures_is_fine() {
        let (mut facts, mods) = setup();
        facts.mesh_overrides[0].textures = paths(&[WALL_TEXTURE, WALL_NORMAL]);
        assert_eq!(
            BypassedRetextures.run(&Context::new(&facts, &mods)),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn without_a_texture_mod_there_is_nothing_to_bypass() {
        let (facts, _) = setup();
        // The dropped texture is the game's own: nobody retextured it.
        let mods = Mods(vec![(WALL_MESH, "Mesh Mod"), (OWN_TEXTURE, "Mesh Mod")]);
        assert_eq!(
            BypassedRetextures.run(&Context::new(&facts, &mods)),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn a_mesh_mod_that_retextures_the_old_path_itself_is_fine() {
        let (facts, _) = setup();
        let mods = Mods(vec![
            (WALL_MESH, "Mesh Mod"),
            (OWN_TEXTURE, "Mesh Mod"),
            (WALL_TEXTURE, "Mesh Mod"),
        ]);
        assert_eq!(
            BypassedRetextures.run(&Context::new(&facts, &mods)),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn replacing_another_mods_mesh_and_textures_is_not_a_bypass() {
        let (mut facts, _) = setup();
        // The original mesh and its texture both belong to one mod, packed
        // in that mod's archive: a mesh replacer for that mod is expected
        // to bring its own textures.
        facts.archives = vec![ArchiveFacts {
            path: GamePath::new("data/house.bsa"),
            files: paths(&[WALL_MESH, WALL_TEXTURE]),
        }];
        facts.loose_files = paths(&[WALL_MESH, OWN_TEXTURE]);
        facts.mesh_overrides[0].replaced_in = GamePath::new("data/house.bsa");
        let mods = Mods(vec![
            ("data/house.bsa", "House Mod"),
            (WALL_MESH, "Mesh Mod"),
            (OWN_TEXTURE, "Mesh Mod"),
        ]);
        assert_eq!(
            BypassedRetextures.run(&Context::new(&facts, &mods)),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn a_mesh_that_merely_uses_fewer_textures_is_fine() {
        let (mut facts, mods) = setup();
        facts.mesh_overrides[0].textures = paths(&[WALL_NORMAL]);
        assert_eq!(
            BypassedRetextures.run(&Context::new(&facts, &mods)),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn meshes_nobody_manages_are_reported_as_unmanaged() {
        let (facts, _) = setup();
        let mods = Mods(vec![
            (WALL_TEXTURE, "Texture Mod"),
            (WALL_NORMAL, "Texture Mod"),
        ]);
        let findings = BypassedRetextures.run(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.args["meshOrigin"], ArgValue::from("unmanaged"));
        assert_eq!(finding.args["meshMod"], ArgValue::from(""));
    }

    #[test]
    fn several_meshes_between_the_same_two_mods_are_one_finding() {
        let (mut facts, mods) = setup();
        let mut second = facts.mesh_overrides[0].clone();
        second.path = GamePath::new("data/meshes/architecture/gate.nif");
        facts.mesh_overrides.push(second);
        let mods = Mods(
            mods.0
                .into_iter()
                .chain([("data/meshes/architecture/gate.nif", "Mesh Mod")])
                .collect(),
        );
        let findings = BypassedRetextures.run(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("gate.nif, wall.nif")
        );
    }
}
