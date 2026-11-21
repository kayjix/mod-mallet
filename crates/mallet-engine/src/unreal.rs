//! The mods of a game's Unreal Engine side, each with the mod it came from
//! (ADR 0065). Only reads.

use std::path::Path;

use mallet_core::{GamePath, ModInfo, ModSource, UnrealPak};
use mallet_game_bethesda::{Game, locate, unreal};
use mallet_source_vortex::{VortexDeployment, manifests_in};

/// Manifests larger than this are not read; Vortex's are a few hundred
/// kilobytes for thousands of files.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;

/// The Unreal packages of `game`, whose data the checks read below
/// `game_dir`, with the mod each came from as Vortex recorded it in the
/// folder the package lies in, named as in `mods`. Empty for the games not
/// drawn with Unreal Engine.
pub(crate) fn paks(game: Game, game_dir: &Path, mods: &[ModInfo]) -> Vec<UnrealPak> {
    if game.unreal_pak_folders().is_empty() {
        return Vec::new();
    }
    let install = locate::install_dir(game, game_dir);
    let mut paks = unreal::paks(game, &install);
    // Vortex records a package in the folder of its mod type, or, for
    // mods of its default type, in the installation's top folder.
    let deployments: Vec<VortexDeployment> = game
        .unreal_pak_folders()
        .iter()
        .map(|(relative, _)| *relative)
        .chain([""])
        .flat_map(|relative| {
            manifests_in(&install.join(relative))
                .into_iter()
                .filter_map(move |manifest| read(&manifest, relative))
        })
        .collect();
    for pak in &mut paks {
        pak.provider = pak.extensions.iter().find_map(|extension| {
            let file = GamePath::new(&format!("{}.{extension}", pak.path));
            deployments
                .iter()
                .find_map(|deployment| deployment.provider_of(&file))
        });
        if let Some(provider) = &mut pak.provider
            && let Some(known) = mods.iter().find(|info| info.mod_ref.id == provider.id)
        {
            provider.name.clone_from(&known.mod_ref.name);
        }
    }
    tracing::info!(
        packages = paks.len(),
        attributed = paks.iter().filter(|pak| pak.provider.is_some()).count(),
        manifests = deployments.len(),
        "read the Unreal Engine mods"
    );
    paks
}

/// The deployment in the manifest at `manifest`, whose files lie below
/// `folder`; `None` if it cannot be read.
fn read(manifest: &Path, folder: &str) -> Option<VortexDeployment> {
    let small = std::fs::metadata(manifest).is_ok_and(|meta| meta.len() <= MAX_MANIFEST_BYTES);
    if !small {
        return None;
    }
    let bytes = std::fs::read(manifest).ok()?;
    VortexDeployment::parse_at(&String::from_utf8_lossy(&bytes), folder)
        .map_err(
            |error| tracing::warn!(%error, manifest = %manifest.display(), "manifest not read"),
        )
        .ok()
}

#[cfg(test)]
mod tests {
    use mallet_core::{ModRef, PakFolder};

    use super::*;

    #[test]
    fn packages_carry_the_mod_vortex_deployed_them_from() {
        let install = tempfile::tempdir().unwrap();
        let root = install
            .path()
            .join("OblivionRemastered/Content/Dev/ObvData");
        std::fs::create_dir_all(root.join("Data")).unwrap();
        let mods = install.path().join("OblivionRemastered/Content/Paks/~mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("Lamps_P.pak"), b"p").unwrap();
        std::fs::write(mods.join("Lamps_P.utoc"), b"t").unwrap();
        std::fs::write(mods.join("Loose_P.pak"), b"p").unwrap();
        std::fs::create_dir_all(mods.join("Menus")).unwrap();
        std::fs::write(mods.join("Menus/Blue_P.pak"), b"p").unwrap();
        std::fs::write(
            install.path().join("vortex.deployment.json"),
            r#"{
                "targetPath": "Z:\\Old\\Drive",
                "files": [
                    { "relPath": "OblivionRemastered\\Content\\Paks\\~mods\\Menus\\Blue_P.pak", "source": "Blue Menus-2-1-0" }
                ]
            }"#,
        )
        .unwrap();
        std::fs::write(
            mods.join("vortex.deployment.example-pak-modtype.json"),
            r#"{
                "targetPath": "Z:\\Old\\Drive\\Paks\\~mods",
                "stagingPath": "Z:\\Staging",
                "files": [
                    { "relPath": "Lamps_P.pak", "source": "Lamps-1-1-0" },
                    { "relPath": "Lamps_P.utoc", "source": "Lamps-1-1-0" }
                ]
            }"#,
        )
        .unwrap();
        let known = [ModInfo {
            mod_ref: ModRef {
                source: "vortex".to_owned(),
                id: "Lamps-1-1-0".to_owned(),
                name: "Brighter Lamps".to_owned(),
            },
            enabled: true,
            version: None,
            newer_version: None,
            author: None,
            page: None,
        }];

        let found = paks(Game::OblivionRemastered, &root, &known);

        let named: Vec<(&str, Option<&str>)> = found
            .iter()
            .map(|pak| {
                (
                    pak.path.rsplit('/').next().unwrap_or_default(),
                    pak.provider.as_ref().map(|provider| provider.name.as_str()),
                )
            })
            .collect();
        assert_eq!(
            named,
            [
                ("Lamps_P", Some("Brighter Lamps")),
                ("Loose_P", None),
                ("Blue_P", Some("Blue Menus-2-1-0")),
            ]
        );
        assert_eq!(found[0].folder, PakFolder::Mods);
        assert_eq!(paks(Game::Oblivion, &root, &known), Vec::new());
    }
}
