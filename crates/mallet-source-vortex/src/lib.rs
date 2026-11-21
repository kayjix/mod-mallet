//! Mod Mallet's adapter for the Vortex mod manager.
//!
//! Vortex keeps each mod in its own folder and links the files of all
//! enabled mods into the game. It records what it linked in a deployment
//! manifest, `vortex.deployment.json`, inside the directory it deployed to.
//! [`VortexDeployment`] reads that manifest and answers which mod a file in
//! the game came from.
//!
//! [`VortexState`] reads Vortex's saved state: the installed mods, the
//! names Vortex shows for them, and what was chosen in their installers.
//!
//! [`link`] asks a running Vortex to act, through a small Vortex extension.
//!
//! Reading needs neither Vortex running nor the extension. Changes are
//! never made to Vortex's data directly; they are requested from Vortex
//! (ADR 0005, ADR 0009, ADR 0020).

pub mod app;
pub mod link;
pub mod state;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mallet_core::{GamePath, ModRef, ModSource};
use serde::Deserialize;

pub use state::{VortexMod, VortexState, VortexTool};

/// The value of [`ModRef::source`] for mods managed by Vortex.
pub const SOURCE_ID: &str = "vortex";

/// The name of the manifest Vortex writes into a directory it deployed to.
pub const MANIFEST_FILE: &str = "vortex.deployment.json";

/// The manifest of the game's `Data` directory, where almost all mod files
/// of a Bethesda game end up.
pub fn data_manifest_path(game_dir: &Path) -> PathBuf {
    game_dir.join("Data").join(MANIFEST_FILE)
}

/// The manifest of the `Data` directory of the game Vortex knows as
/// `game_id`, below `game_dir`. Most games deploy their mods there as
/// Vortex's default; for Oblivion Remastered, whose default goes into the
/// installation's top folder, the data folder is a mod type of its own,
/// and the manifest is named after it (ADR 0065).
pub fn data_manifest_path_for(game_dir: &Path, game_id: &str) -> PathBuf {
    match game_id {
        "oblivionremastered" => game_dir
            .join("Data")
            .join("vortex.deployment.oblivionremastered-data-folder.json"),
        _ => data_manifest_path(game_dir),
    }
}

/// The deployment manifests directly in `dir`: Vortex's default one and
/// those of its mod types (`vortex.deployment.<type>.json`), sorted. A
/// folder that cannot be listed has none.
pub fn manifests_in(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                    name.starts_with("vortex.deployment")
                        && Path::new(&name)
                            .extension()
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                })
                .map(|entry| entry.path())
                .collect()
        },
    );
    found.sort_unstable();
    found
}

/// The files below one mod's folder in Vortex's staging folder, as paths
/// relative to it with backslashes, the way the deployment manifest names
/// them. Only the folder listings are read, no file is opened. Entries
/// that cannot be read are left out.
///
/// # Errors
///
/// If the folder itself cannot be listed.
pub fn list_mod_folder(folder: &Path) -> std::io::Result<Vec<String>> {
    let mut files = Vec::new();
    let mut pending = vec![(std::fs::read_dir(folder)?, String::new())];
    while let Some((entries, prefix)) = pending.pop() {
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if prefix.is_empty() {
                name
            } else {
                format!(r"{prefix}\{name}")
            };
            if kind.is_dir() {
                if let Ok(inner) = std::fs::read_dir(entry.path()) {
                    pending.push((inner, path));
                }
            } else {
                files.push(path);
            }
        }
    }
    files.sort_unstable();
    Ok(files)
}

/// Why a deployment manifest could not be used.
#[derive(Debug, thiserror::Error)]
pub enum VortexError {
    /// The file is not a deployment manifest Mod Mallet understands.
    #[error("the Vortex deployment manifest could not be read: {0}")]
    InvalidManifest(#[from] serde_json::Error),
    /// The manifest describes a deployment into a different installation.
    #[error(
        "the Vortex deployment targets `{target}`, which is not inside the game directory `{game}`"
    )]
    TargetOutsideGame {
        /// The directory the manifest says Vortex deployed to.
        target: String,
        /// The game directory that was scanned.
        game: String,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    target_path: String,
    #[serde(default)]
    staging_path: Option<String>,
    #[serde(default)]
    files: Vec<RawFile>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    rel_path: String,
    source: String,
}

/// The files Vortex deployed into a game, and the mod each one came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VortexDeployment {
    /// Folder names of the mods, as Vortex names them in its staging folder.
    mods: Vec<String>,
    /// Index into `mods` for every deployed file.
    by_path: HashMap<GamePath, usize>,
    /// The name Vortex shows for a mod, by folder name.
    names: HashMap<String, String>,
    /// Where Vortex keeps the mods' folders, as the manifest names it.
    staging_path: Option<String>,
    /// Where the deployed files lie below the game directory, such as
    /// `data`.
    base: Option<GamePath>,
    /// Every file of each mod's folder, deployed or not, by folder name.
    staged: HashMap<String, Vec<GamePath>>,
    /// The folders of the switched-off mods that hold each file.
    switched_off: HashMap<GamePath, Vec<String>>,
}

impl VortexDeployment {
    /// Reads the text of a deployment manifest.
    ///
    /// `game_dir` is the installation the manifest was found in. It is needed
    /// because the manifest names files relative to the directory Vortex
    /// deployed to, while Mod Mallet names them relative to the game
    /// directory.
    pub fn parse(manifest_json: &str, game_dir: &str) -> Result<Self, VortexError> {
        let raw: RawManifest = serde_json::from_str(manifest_json)?;

        let target = GamePath::new(&raw.target_path);
        let game = GamePath::new(game_dir);
        let base = if target == game {
            GamePath::new("")
        } else {
            target
                .strip_base(&game)
                .ok_or_else(|| VortexError::TargetOutsideGame {
                    target: raw.target_path.clone(),
                    game: game_dir.to_owned(),
                })?
        };

        Ok(Self::from_raw(raw, &base))
    }

    /// Reads the text of a deployment manifest whose files lie below
    /// `base`, a folder below the game directory, whatever folder the
    /// manifest says Vortex deployed to. For a manifest found in the folder
    /// it describes: after the game was moved to another drive, the folder
    /// it names no longer exists, but its files are still where the
    /// manifest lies.
    pub fn parse_at(manifest_json: &str, base: &str) -> Result<Self, VortexError> {
        let raw: RawManifest = serde_json::from_str(manifest_json)?;
        Ok(Self::from_raw(raw, &GamePath::new(base)))
    }

    /// The files of `raw`, below `base`.
    fn from_raw(raw: RawManifest, base: &GamePath) -> Self {
        let mut deployment = Self {
            staging_path: raw.staging_path.filter(|path| !path.is_empty()),
            base: Some(base.clone()),
            ..Self::default()
        };
        let mut mod_index: HashMap<String, usize> = HashMap::new();
        for file in raw.files {
            let index = if let Some(index) = mod_index.get(&file.source) {
                *index
            } else {
                let index = deployment.mods.len();
                deployment.mods.push(file.source.clone());
                mod_index.insert(file.source, index);
                index
            };
            deployment.by_path.insert(base.join(&file.rel_path), index);
        }
        deployment
    }

    /// Uses the names Vortex shows for the mods instead of their folder
    /// names. `names` maps a staging folder to the name; see
    /// [`VortexState::names`].
    #[must_use]
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.names = names;
        self
    }

    /// The folder Vortex keeps every mod's files in, as the manifest names
    /// it.
    pub fn staging_path(&self) -> Option<&str> {
        self.staging_path.as_deref()
    }

    /// The folder names of the mods that contributed at least one file.
    pub fn mod_folders(&self) -> &[String] {
        &self.mods
    }

    /// Adds the files of the mods' folders, deployed or not: by folder
    /// name, the paths below each folder as [`list_mod_folder`] lists them.
    #[must_use]
    pub fn with_staged_files(mut self, staged: HashMap<String, Vec<String>>) -> Self {
        self.staged = staged
            .into_iter()
            .map(|(folder, files)| {
                let base = self.base.clone().unwrap_or_else(|| GamePath::new(""));
                let files = files.iter().map(|file| base.join(file)).collect();
                (folder, files)
            })
            .collect();
        self
    }

    /// Adds the files of the folders of mods that are installed but
    /// switched off: by folder name, the paths below each folder as
    /// [`list_mod_folder`] lists them.
    #[must_use]
    pub fn with_switched_off_files(mut self, folders: HashMap<String, Vec<String>>) -> Self {
        let base = self.base.clone().unwrap_or_else(|| GamePath::new(""));
        let mut switched_off: HashMap<GamePath, Vec<String>> = HashMap::new();
        for (folder, files) in folders {
            for file in files {
                switched_off
                    .entry(base.join(&file))
                    .or_default()
                    .push(folder.clone());
            }
        }
        for folders in switched_off.values_mut() {
            folders.sort_unstable();
            folders.dedup();
        }
        self.switched_off = switched_off;
        self
    }

    /// The mod that lives in the staging folder `folder`.
    pub fn mod_ref(&self, folder: &str) -> ModRef {
        ModRef {
            source: SOURCE_ID.to_owned(),
            id: folder.to_owned(),
            name: self
                .names
                .get(folder)
                .cloned()
                .unwrap_or_else(|| folder.to_owned()),
        }
    }

    /// Whether the deployment contains files of the mod in the staging
    /// folder `folder`.
    pub fn has_mod(&self, folder: &str) -> bool {
        self.mods.iter().any(|deployed| deployed == folder)
    }

    /// How many files the deployment covers.
    pub fn file_count(&self) -> usize {
        self.by_path.len()
    }

    /// How many mods contributed at least one file.
    pub fn mod_count(&self) -> usize {
        self.mods.len()
    }
}

impl ModSource for VortexDeployment {
    fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
        let folder = self.mods.get(*self.by_path.get(path)?)?;
        Some(self.mod_ref(folder))
    }

    fn staged_files(&self, mod_ref: &ModRef) -> Option<Vec<GamePath>> {
        if mod_ref.source != SOURCE_ID {
            return None;
        }
        self.staged.get(&mod_ref.id).cloned()
    }

    fn switched_off_with(&self, path: &GamePath) -> Vec<ModRef> {
        self.switched_off
            .get(path)
            .into_iter()
            .flatten()
            .map(|folder| self.mod_ref(folder))
            .collect()
    }

    fn switched_off_files(&self) -> Vec<(GamePath, ModRef)> {
        let mut files: Vec<(GamePath, ModRef)> = self
            .switched_off
            .iter()
            .flat_map(|(path, folders)| {
                folders
                    .iter()
                    .map(|folder| (path.clone(), self.mod_ref(folder)))
            })
            .collect();
        files.sort();
        files
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const GAME_DIR: &str = r"C:\Games\Skyrim Special Edition";

    const MANIFEST: &str = r#"{
        "instance": "00000000-0000-0000-0000-000000000000",
        "version": 1,
        "deploymentMethod": "hardlink_activator",
        "gameId": "skyrimse",
        "deploymentTime": 1794695581306,
        "stagingPath": "D:\\Vortex Mods\\skyrimse",
        "targetPath": "C:\\Games\\Skyrim Special Edition\\Data",
        "files": [
            { "relPath": "SKSE\\Plugins\\Old.dll", "source": "Old Mod 123 1.0", "target": "", "time": 1 },
            { "relPath": "SKSE\\Plugins\\Old.ini", "source": "Old Mod 123 1.0", "target": "", "time": 1 },
            { "relPath": "textures\\sky.dds", "source": "Sky Textures 456 2.0", "target": "", "time": 1 }
        ]
    }"#;

    #[test]
    fn a_manifest_of_a_moved_game_is_read_where_it_lies() {
        // The manifest names a drive the game is no longer on.
        assert!(VortexDeployment::parse(MANIFEST, r"E:\Elsewhere\Skyrim").is_err());
        let deployment = VortexDeployment::parse_at(MANIFEST, "data").unwrap();
        assert_eq!(
            deployment
                .provider_of(&GamePath::new("data/textures/sky.dds"))
                .unwrap()
                .id,
            "Sky Textures 456 2.0"
        );
    }

    #[test]
    fn oblivion_remastered_names_its_data_manifest_after_the_mod_type() {
        let game = Path::new("game");
        assert_eq!(
            data_manifest_path_for(game, "oblivionremastered"),
            game.join("Data")
                .join("vortex.deployment.oblivionremastered-data-folder.json")
        );
        assert_eq!(
            data_manifest_path_for(game, "skyrimse"),
            data_manifest_path(game)
        );
    }

    #[test]
    fn manifests_of_every_mod_type_are_found_in_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("vortex.deployment.json"), b"{}").unwrap();
        std::fs::write(
            dir.path()
                .join("vortex.deployment.example-pak-modtype.json"),
            b"{}",
        )
        .unwrap();
        std::fs::write(dir.path().join("Mod_P.pak"), b"").unwrap();
        assert_eq!(manifests_in(dir.path()).len(), 2);
        assert_eq!(
            manifests_in(&dir.path().join("missing")),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn maps_deployed_files_to_their_mod() {
        let deployment = VortexDeployment::parse(MANIFEST, GAME_DIR).unwrap();
        assert_eq!(deployment.file_count(), 3);
        assert_eq!(deployment.mod_count(), 2);

        let provider = deployment
            .provider_of(&GamePath::new("Data/SKSE/Plugins/old.dll"))
            .unwrap();
        assert_eq!(provider.source, "vortex");
        assert_eq!(provider.id, "Old Mod 123 1.0");
        assert_eq!(provider.name, "Old Mod 123 1.0");
    }

    #[test]
    fn mods_are_shown_under_the_name_vortex_gives_them() {
        let names = HashMap::from([("Old Mod 123 1.0".to_owned(), "Old Mod".to_owned())]);
        let deployment = VortexDeployment::parse(MANIFEST, GAME_DIR)
            .unwrap()
            .with_names(names);

        let known = deployment
            .provider_of(&GamePath::new("data/skse/plugins/old.dll"))
            .unwrap();
        assert_eq!(known.id, "Old Mod 123 1.0");
        assert_eq!(known.name, "Old Mod");

        // A mod the state does not know keeps its folder name.
        let unnamed = deployment
            .provider_of(&GamePath::new("data/textures/sky.dds"))
            .unwrap();
        assert_eq!(unnamed.name, "Sky Textures 456 2.0");
    }

    #[test]
    fn files_of_switched_off_mods_are_found_by_path() {
        let names = HashMap::from([("Town Textures 7 1.0".to_owned(), "Town Textures".to_owned())]);
        let folders = HashMap::from([(
            "Town Textures 7 1.0".to_owned(),
            vec![r"textures\town\wall.dds".to_owned()],
        )]);
        let deployment = VortexDeployment::parse(MANIFEST, GAME_DIR)
            .unwrap()
            .with_names(names)
            .with_switched_off_files(folders);

        let [found] = deployment
            .switched_off_with(&GamePath::new("Data/Textures/Town/Wall.dds"))
            .try_into()
            .unwrap();
        assert_eq!(found.id, "Town Textures 7 1.0");
        assert_eq!(found.name, "Town Textures");
        assert_eq!(
            deployment.switched_off_with(&GamePath::new("data/textures/sky.dds")),
            Vec::<ModRef>::new()
        );
        // A switched-off mod's file is not in the game.
        assert_eq!(
            deployment.provider_of(&GamePath::new("data/textures/town/wall.dds")),
            None
        );
        // Every file of the switched-off mods can be listed with its mod.
        let [(path, owner)] = deployment.switched_off_files().try_into().unwrap();
        assert_eq!(path, GamePath::new("data/textures/town/wall.dds"));
        assert_eq!(owner.name, "Town Textures");
    }

    #[test]
    fn files_vortex_did_not_deploy_have_no_provider() {
        let deployment = VortexDeployment::parse(MANIFEST, GAME_DIR).unwrap();
        assert_eq!(
            deployment.provider_of(&GamePath::new("data/skyrim.esm")),
            None
        );
        // Paths are relative to the game directory, not to `Data`.
        assert_eq!(
            deployment.provider_of(&GamePath::new("skse/plugins/old.dll")),
            None
        );
    }

    #[test]
    fn game_directory_spelling_does_not_matter() {
        let deployment =
            VortexDeployment::parse(MANIFEST, "c:/games/SKYRIM SPECIAL EDITION/").unwrap();
        assert!(
            deployment
                .provider_of(&GamePath::new("data/textures/sky.dds"))
                .is_some()
        );
    }

    #[test]
    fn a_deployment_into_the_game_directory_itself_is_accepted() {
        let manifest = r#"{ "targetPath": "C:\\Games\\Skyrim Special Edition",
            "files": [ { "relPath": "d3d11.dll", "source": "Shader Loader" } ] }"#;
        let deployment = VortexDeployment::parse(manifest, GAME_DIR).unwrap();
        assert!(
            deployment
                .provider_of(&GamePath::new("d3d11.dll"))
                .is_some()
        );
    }

    #[test]
    fn a_manifest_for_another_installation_is_rejected() {
        let error = VortexDeployment::parse(MANIFEST, r"D:\Other Game").unwrap_err();
        assert!(matches!(error, VortexError::TargetOutsideGame { .. }));
    }

    #[test]
    fn broken_json_is_an_error_not_a_crash() {
        let error = VortexDeployment::parse("{ not json", GAME_DIR).unwrap_err();
        assert!(matches!(error, VortexError::InvalidManifest(_)));
    }

    #[test]
    fn data_manifest_lies_in_the_data_directory() {
        let path = data_manifest_path(Path::new("game"));
        assert_eq!(
            path,
            Path::new("game")
                .join("Data")
                .join("vortex.deployment.json")
        );
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(manifest in ".*", game_dir in ".*") {
            let _ = VortexDeployment::parse(&manifest, &game_dir);
        }
    }
}
