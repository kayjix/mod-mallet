//! The mods of the Unreal Engine side of a game: Oblivion Remastered draws
//! with Unreal Engine, which loads packages (`.pak` with `.utoc` and
//! `.ucas`) from `Content/Paks/~mods` and, through UE4SS, from
//! `Content/Paks/LogicMods` (ADR 0065).
//!
//! Only folder listings are read; no package is opened.

use std::collections::BTreeMap;
use std::path::Path;

use mallet_core::UnrealPak;

use crate::game::Game;

/// The extensions of the files an Unreal package is made of.
const PACKAGE_EXTENSIONS: [&str; 4] = ["pak", "utoc", "ucas", "sig"];

/// How deep below a pak folder packages are looked for. Mods put them in
/// a folder of their own, rarely deeper.
const MAX_DEPTH: usize = 6;

/// The Unreal packages in the pak folders of `game` installed in
/// `install`, sorted by path, each with the files that belong to it. A
/// folder that does not exist or cannot be listed is skipped. The mod
/// each came from is left to the caller.
pub fn paks(game: Game, install: &Path) -> Vec<UnrealPak> {
    let mut found = Vec::new();
    for (relative, folder) in game.unreal_pak_folders() {
        let mut packages: BTreeMap<String, UnrealPak> = BTreeMap::new();
        let mut pending = vec![(install.join(relative), (*relative).to_owned(), 0)];
        while let Some((dir, prefix, depth)) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = format!("{prefix}/{name}");
                if meta.is_dir() {
                    if depth < MAX_DEPTH {
                        pending.push((entry.path(), path, depth + 1));
                    }
                    continue;
                }
                let Some((stem, extension)) = package_part(&path) else {
                    continue;
                };
                let package = packages
                    .entry(stem.to_lowercase())
                    .or_insert_with(|| UnrealPak {
                        path: stem.to_owned(),
                        folder: *folder,
                        extensions: Vec::new(),
                        bytes: 0,
                        provider: None,
                    });
                package.extensions.push(extension);
                package.bytes = package.bytes.saturating_add(meta.len());
            }
        }
        for mut package in packages.into_values() {
            package.extensions.sort_unstable();
            package.extensions.dedup();
            found.push(package);
        }
    }
    found
}

/// The package a file belongs to and the file's extension in lower case,
/// if the file is part of a package.
fn package_part(path: &str) -> Option<(&str, String)> {
    let (stem, extension) = path.rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();
    (!stem.is_empty() && !stem.ends_with('/') && PACKAGE_EXTENSIONS.contains(&extension.as_str()))
        .then_some((stem, extension))
}

#[cfg(test)]
mod tests {
    use mallet_core::PakFolder;

    use super::*;

    #[test]
    fn packages_are_listed_with_their_parts_and_other_files_are_not() {
        let install = tempfile::tempdir().unwrap();
        let mods = install.path().join("OblivionRemastered/Content/Paks/~mods");
        let logic = install
            .path()
            .join("OblivionRemastered/Content/Paks/LogicMods");
        std::fs::create_dir_all(mods.join("Menus")).unwrap();
        std::fs::create_dir_all(&logic).unwrap();
        std::fs::write(mods.join("0_Inputs_P.pak"), b"pak").unwrap();
        std::fs::write(mods.join("0_Inputs_P.utoc"), b"utoc").unwrap();
        std::fs::write(mods.join("0_Inputs_P.ucas"), b"ucas!").unwrap();
        std::fs::write(mods.join("Menus/Colors_P.PAK"), b"p").unwrap();
        std::fs::write(mods.join("vortex.deployment.example.json"), b"{}").unwrap();
        std::fs::write(mods.join("readme.txt"), b"hi").unwrap();
        std::fs::write(logic.join("Lantern.pak"), b"pak").unwrap();

        let found = paks(Game::OblivionRemastered, install.path());

        let paths: Vec<&str> = found.iter().map(|pak| pak.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "OblivionRemastered/Content/Paks/~mods/0_Inputs_P",
                "OblivionRemastered/Content/Paks/~mods/Menus/Colors_P",
                "OblivionRemastered/Content/Paks/LogicMods/Lantern",
            ]
        );
        assert_eq!(found[0].extensions, ["pak", "ucas", "utoc"]);
        assert_eq!(found[0].bytes, 12);
        assert_eq!(found[0].folder, PakFolder::Mods);
        assert_eq!(found[1].extensions, ["pak"]);
        assert_eq!(found[2].folder, PakFolder::LogicMods);
    }

    #[test]
    fn games_without_unreal_have_no_packages() {
        let install = tempfile::tempdir().unwrap();
        assert_eq!(paks(Game::Oblivion, install.path()), Vec::new());
        assert_eq!(paks(Game::OblivionRemastered, install.path()), Vec::new());
    }

    proptest::proptest! {
        #[test]
        fn file_names_never_panic(path in ".*") {
            let _ = package_part(&path);
        }
    }
}
