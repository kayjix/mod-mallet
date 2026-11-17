//! What the setup needs besides its mods, and restarting the mod manager.

use std::time::Duration;

use mallet_core::{Tool, ToolGroup, ToolNeed, ToolState};
use mallet_game_bethesda::{locate, skse_log, tools};
use mallet_source_vortex::app;
use mallet_source_vortex::link::{self, Link};
use mallet_unpack::Unpacker;

use crate::{EngineError, ManagerOptions, ScanOptions};

/// Vortex's page.
const VORTEX_PAGE: &str = "https://www.nexusmods.com/about/vortex/";
/// 7-Zip's page.
const SEVEN_ZIP_PAGE: &str = "https://www.7-zip.org/";

/// How long Vortex is given to close before a restart gives up.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Looks at the programs and files the setup needs besides its mods: the
/// mod manager and Mod Mallet's link to it, the game's script extender and
/// its libraries, and the tools Mod Mallet uses. Only reads.
pub fn toolbox(scan: &ScanOptions, manager: &ManagerOptions) -> Vec<Tool> {
    let mut found = vortex(manager);
    let game_dir = scan.game_dir.clone().or_else(locate::game_dir);
    if let Some(game_dir) = game_dir {
        let game_version = scan
            .extender_log
            .clone()
            .or_else(locate::extender_log)
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| skse_log::parse(&text).game_version);
        found.extend(tools::requirements(&game_dir, game_version.as_deref()));
    }
    let unpacker = scan
        .unpacker
        .clone()
        .map(Unpacker::at)
        .or_else(Unpacker::locate)
        .filter(|unpacker| unpacker.program().is_file());
    found.push(
        Tool::new(
            "seven-zip",
            ToolNeed::Recommended,
            if unpacker.is_some() {
                ToolState::Ready
            } else {
                ToolState::Missing
            },
        )
        .in_group(ToolGroup::Helper)
        .with_page(SEVEN_ZIP_PAGE)
        .with_path(unpacker.map(|unpacker| unpacker.program().display().to_string())),
    );
    found
}

/// Vortex itself and the link to it.
fn vortex(options: &ManagerOptions) -> Vec<Tool> {
    let program = app::executable();
    let running = program.is_some() && app::is_running();
    let vortex = Tool::new(
        "vortex",
        ToolNeed::Recommended,
        match (&program, running) {
            (None, _) => ToolState::Missing,
            (Some(_), false) => ToolState::Stopped,
            (Some(_), true) => ToolState::Ready,
        },
    )
    .in_group(ToolGroup::ModManager)
    .with_page(VORTEX_PAGE)
    .with_path(program.as_ref().map(|path| path.display().to_string()));

    let data_dir = options.vortex_data_dir();
    let present = data_dir
        .as_ref()
        .is_some_and(|dir| link::extension_dir(dir).join("info.json").is_file());
    let state = crate::manager_link(options);
    let answering = options
        .link_dir()
        .and_then(|dir| Link::new(dir).running_extension());
    let link_state = if !present {
        ToolState::Missing
    } else if !state.installed || state.restart_needed {
        ToolState::Outdated
    } else if !state.running {
        ToolState::Stopped
    } else {
        ToolState::Ready
    };
    let link = Tool::new("mallet-link", ToolNeed::Recommended, link_state)
        .in_group(ToolGroup::ManagerLink)
        .with_version(answering.or_else(link::extension_version))
        .with_path(data_dir.map(|dir| link::extension_dir(&dir).display().to_string()));
    vec![vortex, link]
}

/// The picture of a game as a `data:` address for the window, taken from
/// the mod manager's installation, or `None` if it has none.
pub fn game_art(game_id: &str) -> Option<String> {
    let bytes = app::game_art(game_id)?;
    Some(format!(
        "data:image/jpeg;base64,{}",
        mallet_core::encoding::base64(&bytes)
    ))
}

/// Closes Vortex and starts it again, for example so that it loads a new
/// version of the link. Only to be called because the user asked for it.
///
/// # Errors
///
/// If Vortex is not installed, does not close within half a minute, or
/// cannot be started.
pub fn restart_manager() -> Result<(), EngineError> {
    tracing::info!("restarting Vortex");
    app::restart(CLOSE_TIMEOUT).map_err(|error| {
        tracing::warn!(%error, "Vortex was not restarted");
        EngineError::Restart(error.to_string())
    })
}
