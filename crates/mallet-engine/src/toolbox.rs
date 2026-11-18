//! What the setup needs besides its mods, and restarting the mod manager.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    if let (Ok(game), Some((game_dir, _))) = (scan.game(), scan.located_game_dir()) {
        let game_version = scan
            .extender_log
            .clone()
            .or_else(|| locate::extender_log(game, Some(&game_dir)))
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| skse_log::parse(game, &text).game_version);
        found.extend(tools::requirements(
            game,
            &game_dir,
            game_version.as_deref(),
        ));
        // Vortex has LOOT built in; then LOOT on its own is not needed.
        if let Some(built_in) = loot_in_vortex(manager, scan.game_id())
            && let Some(loot) = found.iter_mut().find(|tool| tool.id == "loot")
        {
            *loot = built_in;
        }
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

/// LOOT as Vortex has it built in: Vortex sorts plugins with LOOT's
/// library and keeps LOOT's masterlist for each game, which Mod Mallet
/// reads for its rules (ADR 0030). Ready when that masterlist is there,
/// with its date; `None` otherwise.
fn loot_in_vortex(manager: &ManagerOptions, game_id: &str) -> Option<Tool> {
    let masterlist = manager
        .vortex_data_dir()?
        .join(game_id)
        .join("masterlist")
        .join("masterlist.yaml");
    let modified = std::fs::metadata(&masterlist).ok()?.modified().ok()?;
    Some(
        Tool::new("loot", ToolNeed::Recommended, ToolState::Ready)
            .in_group(ToolGroup::Helper)
            .provided_by("vortex")
            .with_version(date(modified))
            .with_path(Some(masterlist.display().to_string())),
    )
}

/// A point in time as its date, `YYYY-MM-DD`, in universal time.
fn date(time: SystemTime) -> Option<String> {
    let days = i64::try_from(time.duration_since(UNIX_EPOCH).ok()?.as_secs() / 86_400).ok()?;
    // Days since 1970-01-01 to a date of the Gregorian calendar, after
    // Howard Hinnant's `civil_from_days`.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    Some(format!("{year:04}-{month:02}-{day:02}"))
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn dates_are_written_as_the_calendar_has_them() {
        let at = |seconds: u64| date(UNIX_EPOCH + Duration::from_secs(seconds));
        assert_eq!(at(0).as_deref(), Some("1970-01-01"));
        // 2000-02-29, a leap day.
        assert_eq!(at(951_782_400).as_deref(), Some("2000-02-29"));
        // 2026-11-17 18:00 UTC.
        assert_eq!(at(1_794_938_400).as_deref(), Some("2026-11-17"));
    }

    #[test]
    fn loot_counts_as_built_into_vortex_when_vortex_keeps_its_masterlist() {
        let vortex = tempfile::tempdir().unwrap();
        let manager = ManagerOptions {
            vortex_data_dir: Some(vortex.path().to_path_buf()),
            ..ManagerOptions::default()
        };
        assert_eq!(loot_in_vortex(&manager, "skyrimse"), None);

        let folder = vortex.path().join("skyrimse").join("masterlist");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("masterlist.yaml"), "plugins: []\n").unwrap();
        let loot = loot_in_vortex(&manager, "skyrimse").unwrap();
        assert_eq!(loot.state, ToolState::Ready);
        assert_eq!(loot.need, ToolNeed::Recommended);
        assert_eq!(loot.provided_by.as_deref(), Some("vortex"));
        assert!(loot.version.is_some_and(|date| date.len() == 10));
    }
}
