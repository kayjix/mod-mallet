//! Carries out actions by handing them to the tool that owns the change.
//!
//! Today there is one such tool, Vortex, reached through the link in
//! `mallet-source-vortex`. An action never fails with an error: whatever
//! happens is an [`ActionOutcome`] that can be put into words for the
//! user.

use std::path::PathBuf;
use std::time::Duration;

use mallet_core::action::{Action, ActionOutcome, ActionStatus, OptionToTick};
use mallet_core::{DEFAULT_PATCH_NAME, Patch};
use mallet_source_vortex::SOURCE_ID;
use mallet_source_vortex::link::{self, Link, LinkError};

use crate::EngineError;

/// How long Vortex is given to answer. The extension looks for requests
/// once a second.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Where the mod manager is. Every field that is `None` is the standard
/// place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManagerOptions {
    /// The game whose mods are meant, by its identifier (`skyrimse`).
    /// Skyrim Special Edition if `None`.
    pub game: Option<String>,
    /// The directory in which Vortex keeps its data.
    pub vortex_data_dir: Option<PathBuf>,
    /// The folder shared with the Vortex extension.
    pub link_dir: Option<PathBuf>,
    /// How long to wait for the mod manager's answer.
    pub timeout: Option<Duration>,
}

impl ManagerOptions {
    /// The game whose mods are meant. An unknown identifier is taken for
    /// what it says: Vortex knows more games than Mod Mallet.
    pub(crate) fn vortex_game(&self) -> &str {
        self.game
            .as_deref()
            .unwrap_or_else(|| mallet_game_bethesda::Game::default().id())
    }

    pub(crate) fn vortex_data_dir(&self) -> Option<PathBuf> {
        self.vortex_data_dir
            .clone()
            .or_else(|| Some(dirs::config_dir()?.join("Vortex")))
    }

    pub(crate) fn link_dir(&self) -> Option<PathBuf> {
        self.link_dir.clone().or_else(default_link_dir)
    }
}

/// The folder shared with the Vortex extension, in its standard place.
pub(crate) fn default_link_dir() -> Option<PathBuf> {
    Some(
        dirs::data_local_dir()?
            .join("mod-mallet")
            .join("vortex-bridge"),
    )
}

/// How things stand between Mod Mallet and the mod manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagerLink {
    /// Whether the link is installed in the mod manager in the version
    /// this Mod Mallet needs.
    pub installed: bool,
    /// Whether the link is answering right now.
    pub running: bool,
    /// Whether the mod manager is still running an older version of the
    /// link than the one installed, and has to be restarted to load it.
    pub restart_needed: bool,
}

/// Looks at whether actions can be handed to the mod manager.
pub fn manager_link(options: &ManagerOptions) -> ManagerLink {
    let installed = options
        .vortex_data_dir()
        .is_some_and(|dir| link::extension_is_current(&dir));
    let running = options
        .link_dir()
        .and_then(|dir| Link::new(dir).running_extension());
    ManagerLink {
        installed,
        restart_needed: installed
            && running
                .as_ref()
                .is_some_and(|version| Some(version) != link::extension_version().as_ref()),
        running: running.is_some(),
    }
}

/// Installs the link into the mod manager, replacing an older version, and
/// returns the folder it was put in. The mod manager loads it the next time
/// it starts.
///
/// This writes two files into Vortex's plugin folder. It must only be
/// called when the user asked for it.
pub fn install_manager_link(options: &ManagerOptions) -> Result<PathBuf, EngineError> {
    let vortex_data_dir = options
        .vortex_data_dir()
        .ok_or(EngineError::ManagerNotFound)?;
    if !vortex_data_dir.is_dir() {
        return Err(EngineError::ManagerNotFound);
    }
    link::install_extension(&vortex_data_dir).map_err(|source| EngineError::Read {
        path: link::extension_dir(&vortex_data_dir),
        source,
    })
}

/// Hands `action` to the tool that carries it out and says what became of
/// it.
pub fn run_action(action: &Action, options: &ManagerOptions) -> ActionOutcome {
    match action {
        Action::ReinstallMod { mod_ref, tick } if mod_ref.source == SOURCE_ID => {
            reinstall_in_vortex(&mod_ref.id, tick, options)
        }
        Action::HideObjects { patch } => hide_objects(patch, options),
        Action::LoadAfter { plugin, after } => in_vortex(options, |link, timeout| {
            tracing::info!(plugin, after, "asking Vortex for a plugin rule");
            link.load_after(options.vortex_game(), plugin, after, timeout)
        }),
        Action::DisableMod { mod_ref } if mod_ref.source == SOURCE_ID => {
            in_vortex(options, |link, timeout| {
                tracing::info!(mod_id = mod_ref.id, "asking Vortex to disable a mod");
                link.set_mod_enabled(options.vortex_game(), &mod_ref.id, false, timeout)
            })
        }
        Action::EnableMod { mod_ref } if mod_ref.source == SOURCE_ID => {
            in_vortex(options, |link, timeout| {
                tracing::info!(mod_id = mod_ref.id, "asking Vortex to enable a mod");
                link.set_mod_enabled(options.vortex_game(), &mod_ref.id, true, timeout)
            })
        }
        Action::FileOverrides {
            files,
            leave_out,
            provide,
        } => {
            fn ids(mods: &[mallet_core::ModRef]) -> Option<Vec<&str>> {
                mods.iter()
                    .map(|mod_ref| (mod_ref.source == SOURCE_ID).then_some(mod_ref.id.as_str()))
                    .collect()
            }
            let (Some(leave_out), Some(provide)) = (ids(leave_out), ids(provide)) else {
                return ActionOutcome {
                    status: ActionStatus::Unsupported,
                    detail: Some("file overrides only through Vortex".to_owned()),
                };
            };
            let files: Vec<&str> = files.iter().map(String::as_str).collect();
            in_vortex(options, |link, timeout| {
                tracing::info!(
                    files = files.len(),
                    leave_out = leave_out.len(),
                    provide = provide.len(),
                    "asking Vortex for file overrides"
                );
                link.file_overrides(options.vortex_game(), &files, &leave_out, &provide, timeout)
            })
        }
        Action::ReinstallMod { mod_ref, .. }
        | Action::DisableMod { mod_ref }
        | Action::EnableMod { mod_ref } => ActionOutcome {
            status: ActionStatus::Unsupported,
            detail: Some(format!("no link to the mod manager `{}`", mod_ref.source)),
        },
    }
}

/// Asks Vortex to list Mod Mallet among its tools for the game in use, or
/// to take it off the list (ADR 0063). Only to be called because the user
/// pressed the button.
pub fn set_mallet_tool(options: &ManagerOptions, add: bool) -> ActionOutcome {
    in_vortex(options, |link, timeout| {
        tracing::info!(
            add,
            "asking Vortex to change Mod Mallet's place among its tools"
        );
        link.mallet_tool(options.vortex_game(), add, timeout)
    })
}

/// Hands a request to Vortex through the link, if the link is there.
fn in_vortex(
    options: &ManagerOptions,
    request: impl FnOnce(&Link, Duration) -> Result<link::Reply, LinkError>,
) -> ActionOutcome {
    if !manager_link(options).installed {
        return ActionOutcome::new(ActionStatus::LinkMissing);
    }
    let Some(link_dir) = options.link_dir() else {
        return ActionOutcome::new(ActionStatus::ManagerNotRunning);
    };
    let timeout = options.timeout.unwrap_or(DEFAULT_TIMEOUT);
    outcome_of(request(&Link::new(link_dir), timeout))
}

/// Where the patch with the standard name is written. The folder holds
/// every patch Mod Mallet writes.
pub fn patch_file(options: &ManagerOptions) -> Option<PathBuf> {
    patch_file_named(DEFAULT_PATCH_NAME, options)
}

/// Where the patch named `name` is written. Vortex names the mod after the
/// archive, so patches with different names become different mods.
fn patch_file_named(name: &str, options: &ManagerOptions) -> Option<PathBuf> {
    Some(
        Link::new(options.link_dir()?)
            .patches_dir()
            .join(mallet_game_bethesda::swap::archive_name(name)),
    )
}

/// A patch that has been written and can now be installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedPatch {
    /// The archive.
    pub path: PathBuf,
    /// How many objects it hides.
    pub objects: usize,
}

/// Writes `patch` into Mod Mallet's own folder, with an installer that
/// offers its parts (ADR 0053), replacing the patch of the same name
/// written before. Parts that hide nothing are left out. Nothing is
/// installed: that is [`Action::HideObjects`].
///
/// # Errors
///
/// If there is nothing to hide, or the archive cannot be written.
pub fn create_patch(patch: &Patch, options: &ManagerOptions) -> Result<CreatedPatch, EngineError> {
    let mut patch = patch.clone();
    patch.parts.retain(|part| !part.objects.is_empty());
    if patch.name.trim().is_empty() {
        DEFAULT_PATCH_NAME.clone_into(&mut patch.name);
    }
    let objects = patch.objects();
    if objects.is_empty() {
        return Err(EngineError::NothingToHide);
    }
    let archive = patch_file_named(&patch.name, options).ok_or(EngineError::ManagerNotFound)?;
    if let Some(dir) = archive.parent() {
        std::fs::create_dir_all(dir).map_err(|source| EngineError::Write {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    crate::patch::write_zip(&archive, &crate::patch::patch_files(&patch)).map_err(|source| {
        EngineError::Write {
            path: archive.clone(),
            source,
        }
    })?;
    tracing::info!(
        path = %archive.display(),
        parts = patch.parts.len(),
        objects = objects.len(),
        "patch written"
    );
    Ok(CreatedPatch {
        path: archive,
        objects: objects.len(),
    })
}

/// The file name of the report written next to the patch.
const PATCH_REPORT: &str = "Mod Mallet Patch - report.txt";

/// Writes `text`, the report of what the patch hides and why, into Mod
/// Mallet's own folder next to the patch, replacing the one written before
/// (ADR 0050). Nothing else is touched.
///
/// # Errors
///
/// If Mod Mallet's folder is not known, or the file cannot be written.
pub fn save_patch_report(text: &str, options: &ManagerOptions) -> Result<PathBuf, EngineError> {
    let path = patch_file(options)
        .ok_or(EngineError::ManagerNotFound)?
        .with_file_name(PATCH_REPORT);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| EngineError::Write {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(&path, text).map_err(|source| EngineError::Write {
        path: path.clone(),
        source,
    })?;
    tracing::info!(path = %path.display(), "patch report written");
    Ok(path)
}

/// Writes `patch` and asks Vortex to install it.
fn hide_objects(patch: &Patch, options: &ManagerOptions) -> ActionOutcome {
    let swaps = mallet_game_bethesda::Game::from_id(options.vortex_game())
        .is_some_and(mallet_game_bethesda::Game::has_object_swapper);
    if !swaps {
        return ActionOutcome {
            status: ActionStatus::Failed,
            detail: Some(EngineError::NoObjectSwapper.to_string()),
        };
    }
    if patch.parts.iter().all(|part| part.objects.is_empty()) {
        return ActionOutcome {
            status: ActionStatus::Failed,
            detail: Some(EngineError::NothingToHide.to_string()),
        };
    }
    let state = manager_link(options);
    if !state.installed {
        return ActionOutcome::new(ActionStatus::LinkMissing);
    }
    let Some(link_dir) = options.link_dir() else {
        return ActionOutcome::new(ActionStatus::ManagerNotRunning);
    };
    let created = match create_patch(patch, options) {
        Ok(created) => created,
        Err(error) => {
            tracing::warn!(%error, "patch not written");
            return ActionOutcome {
                status: ActionStatus::Failed,
                detail: Some(error.to_string()),
            };
        }
    };
    let timeout = options.timeout.unwrap_or(DEFAULT_TIMEOUT);
    tracing::info!(archive = %created.path.display(), "asking Vortex to install the patch");
    outcome_of(Link::new(link_dir).install_archive(options.vortex_game(), &created.path, timeout))
}

fn reinstall_in_vortex(
    mod_id: &str,
    tick: &[OptionToTick],
    options: &ManagerOptions,
) -> ActionOutcome {
    let state = manager_link(options);
    if !state.installed {
        return ActionOutcome::new(ActionStatus::LinkMissing);
    }
    let Some(link_dir) = options.link_dir() else {
        return ActionOutcome::new(ActionStatus::ManagerNotRunning);
    };
    let timeout = options.timeout.unwrap_or(DEFAULT_TIMEOUT);
    tracing::info!(
        mod_id,
        options = tick.len(),
        "asking Vortex to reinstall a mod"
    );
    outcome_of(Link::new(link_dir).reinstall_mod(options.vortex_game(), mod_id, tick, timeout))
}

/// What a reply of the extension means.
fn outcome_of(reply: Result<link::Reply, LinkError>) -> ActionOutcome {
    match &reply {
        Ok(reply) => tracing::info!(
            code = %reply.code,
            detail = reply.detail.as_deref().unwrap_or(""),
            "Vortex answered"
        ),
        Err(error) => tracing::warn!(%error, "Vortex did not answer"),
    }
    match reply {
        Ok(reply) => ActionOutcome {
            status: match reply.code.as_str() {
                "started" => ActionStatus::Started,
                "installing" => ActionStatus::Installing,
                "applied" => ActionStatus::Applied,
                "unknown-mod" => ActionStatus::UnknownMod,
                "archive-missing" => ActionStatus::ArchiveMissing,
                "wrong-game" => ActionStatus::WrongGame,
                "unsupported" => ActionStatus::Unsupported,
                // The extension only calls a request expired that this
                // side has already given up on.
                "expired" => ActionStatus::NoAnswer,
                _ => ActionStatus::Failed,
            },
            detail: reply.detail,
        },
        Err(LinkError::NotRunning) => ActionOutcome::new(ActionStatus::ManagerNotRunning),
        Err(LinkError::NoAnswer) => ActionOutcome::new(ActionStatus::NoAnswer),
        Err(error) => ActionOutcome {
            status: ActionStatus::Failed,
            detail: Some(error.to_string()),
        },
    }
}
