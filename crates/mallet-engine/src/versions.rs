//! Every version of a thing of the catalogue, and what each looks like
//! (ADR 0053): the copies of its files in the game's archives, in the
//! mods' archives, in the data directory and in the mod manager's staging
//! folder, where the copies other mods replaced are kept.
//!
//! Only reads. Files in the staging folder are opened only for the thing
//! looked at.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mallet_core::catalogue::{AssetSource, AssetVersion, AssetVersions};
use mallet_core::mesh::MeshView;
use mallet_core::{Action, GamePath, ModRef, ModSource};
use mallet_game_bethesda::Game;
use mallet_game_bethesda::files;
use mallet_game_bethesda::lod_library::LodLibrary;
use mallet_source_vortex::VortexDeployment;

use crate::ScanOptions;

/// The folders below the data directory a thing's files lie in.
const PREFIXES: [&str; 3] = ["meshes/", "textures/", "materials/"];

/// How many files of one thing are looked at, models and textures
/// together; enough for any weapon, armour or face.
const MAX_FILES: usize = 40;

/// Files in the staging folder larger than this are not read.
const MAX_STAGED: u64 = 64 * 1024 * 1024;

/// Where every file of the setup can come from, found once per setup.
struct Sources {
    key: String,
    game: Game,
    library: LodLibrary,
    deployment: Option<VortexDeployment>,
    staging: Option<PathBuf>,
    /// The mods whose folders hold each file, by its path below the data
    /// directory, lower case.
    staged: HashMap<String, Vec<String>>,
}

/// The sources of the setup last asked about.
static KEPT: Mutex<Option<Arc<Sources>>> = Mutex::new(None);

fn sources(options: &ScanOptions) -> Option<Arc<Sources>> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let key = format!(
        "{}|{}|{}",
        game.id(),
        game_dir.display(),
        crate::setup_fingerprint(options)
    );
    if let Ok(kept) = KEPT.lock()
        && let Some(sources) = kept.as_ref()
        && sources.key == key
    {
        return Some(Arc::clone(sources));
    }
    let started = std::time::Instant::now();
    let prefixes: Vec<String> = PREFIXES.iter().map(|&prefix| prefix.to_owned()).collect();
    let library =
        LodLibrary::with_prefixes(game, &game_dir, options.local_dir.as_deref(), &prefixes);
    let deployment = deployment(options, &game_dir);
    let staging = deployment.as_ref().and_then(|deployment| {
        options
            .staging_dir
            .clone()
            .or_else(|| deployment.staging_path().map(PathBuf::from))
    });
    let mut staged: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(deployment) = &deployment {
        for folder in deployment.mod_folders() {
            for path in deployment
                .staged_files(&deployment.mod_ref(folder))
                .unwrap_or_default()
            {
                let path = files::normalize(path.as_str());
                if PREFIXES.iter().any(|prefix| path.starts_with(prefix)) {
                    staged.entry(path).or_default().push(folder.clone());
                }
            }
        }
    }
    tracing::info!(
        staged = staged.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "found where the files of things come from"
    );
    let sources = Arc::new(Sources {
        key,
        game,
        library,
        deployment,
        staging,
        staged,
    });
    if let Ok(mut kept) = KEPT.lock() {
        *kept = Some(Arc::clone(&sources));
    }
    Some(sources)
}

/// Vortex's deployment with the files of every mod's folder, or `None`
/// without Vortex.
fn deployment(options: &ScanOptions, game_dir: &std::path::Path) -> Option<VortexDeployment> {
    let manifest_path = options
        .deployment_manifest
        .clone()
        .unwrap_or_else(|| mallet_source_vortex::data_manifest_path(game_dir));
    let text = std::fs::read_to_string(manifest_path).ok()?;
    let deployment = VortexDeployment::parse(&text, &game_dir.to_string_lossy()).ok()?;
    // The listings are not kept in the cache here: they are read once per
    // setup and kept in memory with the rest.
    Some(crate::staging::read(
        options,
        deployment,
        None,
        &mut Vec::new(),
    ))
}

impl Sources {
    fn mod_of(&self, path: &str) -> Option<ModRef> {
        self.deployment
            .as_ref()?
            .provider_of(&GamePath::new(&format!("data/{path}")))
    }

    /// The version an archive's copies belong to.
    fn archive_version(&self, name: &str) -> String {
        if let Some(found) = self.mod_of(name) {
            return format!("mod:{}", found.id);
        }
        let lower = name.to_ascii_lowercase();
        let stem = lower
            .rsplit_once('.')
            .map_or(lower.as_str(), |(stem, _)| stem);
        let plugin = stem.split(" - ").next().unwrap_or(stem);
        let base = self
            .game
            .base_archive_prefixes()
            .iter()
            .any(|prefix| lower.starts_with(prefix))
            || self.game.base_archives_last().contains(&lower.as_str());
        if base
            || ["esm", "esp", "esl"]
                .iter()
                .any(|extension| self.game.is_official(&format!("{plugin}.{extension}")))
        {
            "game".to_owned()
        } else {
            format!("archive:{name}")
        }
    }

    /// Every version that has a copy of `path`.
    fn versions_of(&self, path: &str) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        let mut add = |version: String| {
            if !found.contains(&version) {
                found.push(version);
            }
        };
        for archive in self.library.archives_with(path) {
            add(self.archive_version(&archive));
        }
        for folder in self.staged.get(path).into_iter().flatten() {
            add(format!("mod:{folder}"));
        }
        if self.library.has_loose(path) {
            add(self.loose_version(path));
        }
        found
    }

    fn loose_version(&self, path: &str) -> String {
        self.mod_of(path)
            .map_or_else(|| "loose".to_owned(), |found| format!("mod:{}", found.id))
    }

    /// The version whose copy of `path` the game uses: a loose file wins
    /// over every archive, a later archive over an earlier one.
    fn winner(&self, path: &str) -> Option<String> {
        if self.library.has_loose(path) {
            return Some(self.loose_version(path));
        }
        let last = self.library.archives_with(path).pop()?;
        Some(self.archive_version(&last))
    }

    /// The copy of `path` that `version` brings, if it has one.
    fn read(&self, path: &str, version: &str) -> Option<Vec<u8>> {
        let path = files::normalize(path);
        if let Some(folder) = version.strip_prefix("mod:") {
            // The mod's own folder first: that is where a loose copy lies,
            // deployed or kept back.
            let staged = self
                .staged
                .get(&path)
                .is_some_and(|folders| folders.iter().any(|known| known == folder));
            if staged
                && let Some(staging) = &self.staging
                && let Some(bytes) = read_small(&staging.join(folder).join(&path))
            {
                return Some(bytes);
            }
        }
        if self.library.has_loose(&path)
            && self.loose_version(&path) == version
            && let Some(bytes) = self.library.read_loose(&path)
        {
            return Some(bytes);
        }
        self.library
            .archives_with(&path)
            .into_iter()
            .rev()
            .filter(|archive| self.archive_version(archive) == version)
            .find_map(|archive| self.library.read_from_archive(&path, &archive))
    }

    /// The copy of `path` the game uses.
    fn read_shown(&self, path: &str) -> Option<Vec<u8>> {
        self.read(path, &self.winner(path)?)
    }

    fn mod_ref(&self, folder: &str) -> ModRef {
        self.deployment.as_ref().map_or_else(
            || ModRef {
                source: "vortex".to_owned(),
                id: folder.to_owned(),
                name: folder.to_owned(),
            },
            |deployment| deployment.mod_ref(folder),
        )
    }

    /// The mods whose folders hold a loose copy of `path`.
    fn loose_mods(&self, path: &str) -> &[String] {
        self.staged.get(path).map_or(&[], Vec::as_slice)
    }

    /// What makes the game show `version` where file overrides can: the
    /// other mods' loose copies left out, the version's own put back.
    /// `None` if the game shows it already or overrides cannot help.
    fn choice(&self, version: &AssetVersion) -> Option<Action> {
        if version.shown.len() == version.files.len() {
            return None;
        }
        let own = version.id.strip_prefix("mod:");
        let mut files = Vec::new();
        let mut leave_out: Vec<String> = Vec::new();
        let mut provide: Vec<String> = Vec::new();
        for path in &version.files {
            let loose = self.loose_mods(path);
            let own_loose = own.is_some_and(|own| loose.iter().any(|folder| folder == own));
            // Loose files no mod manager knows cannot be left out.
            let unmanaged = self.library.has_loose(path) && self.mod_of(path).is_none();
            // Without a loose copy of its own, the version shows only if
            // its archive is the last to hold the file.
            let reachable = !unmanaged
                && (own_loose
                    || self
                        .library
                        .archives_with(path)
                        .last()
                        .is_some_and(|archive| self.archive_version(archive) == version.id));
            if !reachable {
                continue;
            }
            files.push(path.clone());
            for folder in loose {
                let list = if Some(folder.as_str()) == own {
                    &mut provide
                } else {
                    &mut leave_out
                };
                if !list.contains(folder) {
                    list.push(folder.clone());
                }
            }
        }
        if leave_out.is_empty() {
            return None;
        }
        Some(Action::FileOverrides {
            files,
            leave_out: leave_out
                .iter()
                .map(|folder| self.mod_ref(folder))
                .collect(),
            provide: provide.iter().map(|folder| self.mod_ref(folder)).collect(),
        })
    }

    /// What gives every mod's loose copies of `paths` back to the mod
    /// manager's order.
    fn reset(&self, paths: &[String]) -> Option<Action> {
        let mut provide: Vec<String> = Vec::new();
        let mut files = Vec::new();
        for path in paths {
            let loose = self.loose_mods(path);
            if loose.is_empty() {
                continue;
            }
            files.push(path.clone());
            for folder in loose {
                if !provide.contains(folder) {
                    provide.push(folder.clone());
                }
            }
        }
        (!files.is_empty()).then(|| Action::FileOverrides {
            files,
            leave_out: Vec::new(),
            provide: provide.iter().map(|folder| self.mod_ref(folder)).collect(),
        })
    }

    fn source(&self, version: &str) -> AssetSource {
        if version == "game" {
            AssetSource::Game
        } else if version == "loose" {
            AssetSource::Loose
        } else if let Some(name) = version.strip_prefix("archive:") {
            AssetSource::Archive {
                name: name.to_owned(),
            }
        } else {
            let folder = version.strip_prefix("mod:").unwrap_or(version);
            AssetSource::Mod {
                mod_ref: self.mod_ref(folder),
            }
        }
    }
}

fn read_small(file: &std::path::Path) -> Option<Vec<u8>> {
    let small =
        std::fs::metadata(file).is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_STAGED);
    small.then(|| std::fs::read(file).ok()).flatten()
}

/// Every version of the thing whose models are `models` (below the data
/// directory): the copies of its models and of the textures they name, by
/// where they come from. The first call for a setup finds every file of
/// the archives and mods, which takes a moment. `None` if the setup cannot
/// be read.
pub fn asset_versions(options: &ScanOptions, models: &[String]) -> Option<AssetVersions> {
    let sources = sources(options)?;
    let mut paths: Vec<String> = Vec::new();
    let add = |path: String, paths: &mut Vec<String>| {
        if paths.len() < MAX_FILES && !paths.contains(&path) {
            paths.push(path);
        }
    };
    for model in models {
        add(files::normalize(model), &mut paths);
    }
    // The textures every copy of every model names.
    for model in models {
        let model = files::normalize(model);
        for version in sources.versions_of(&model) {
            let Some(bytes) = sources.read(&model, &version) else {
                continue;
            };
            for texture in files::surface_textures(&bytes) {
                add(crate::mesh::texture_path(&texture), &mut paths);
            }
        }
        if let Some(tint) = crate::mesh::face_tint(&model) {
            add(tint, &mut paths);
        }
    }

    let mut by_version: BTreeMap<String, AssetVersion> = BTreeMap::new();
    for path in &paths {
        let winner = sources.winner(path);
        for version in sources.versions_of(path) {
            let entry = by_version
                .entry(version.clone())
                .or_insert_with(|| AssetVersion {
                    id: version.clone(),
                    source: sources.source(&version),
                    files: Vec::new(),
                    shown: Vec::new(),
                    choice: None,
                });
            entry.files.push(path.clone());
            if winner.as_deref() == Some(version.as_str()) {
                entry.shown.push(path.clone());
            }
        }
    }
    let mut versions: Vec<AssetVersion> = by_version
        .into_values()
        .map(|version| AssetVersion {
            choice: sources.choice(&version),
            ..version
        })
        .collect();
    versions.sort_by(|a, b| {
        b.shown
            .len()
            .cmp(&a.shown.len())
            .then_with(|| (b.source == AssetSource::Game).cmp(&(a.source == AssetSource::Game)))
            .then_with(|| a.id.cmp(&b.id))
    });
    let reset = sources.reset(&paths);
    Some(AssetVersions {
        files: paths,
        versions,
        reset,
    })
}

/// The mesh at `path` as the version `version` of a thing makes it look:
/// its copies of the mesh and its textures where it has them, the copies
/// the game uses for the rest. `None` if there is no mesh to draw.
pub fn asset_view(options: &ScanOptions, path: &str, version: &str) -> Option<MeshView> {
    let sources = sources(options)?;
    let read = |file: &str| {
        sources
            .read(file, version)
            .or_else(|| sources.read_shown(file))
    };
    let bytes = read(path)?;
    crate::mesh::render(path, &bytes, None, &read)
}
