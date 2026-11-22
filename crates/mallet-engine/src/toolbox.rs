//! What the setup needs besides its mods, and restarting the mod manager.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mallet_core::{Tool, ToolGroup, ToolNeed, ToolState};
use std::path::Path;

use mallet_game_bethesda::tools::ListedTool;
use mallet_game_bethesda::{Game, locate, skse_log, tools, versions};
use mallet_source_vortex::VortexTool;
use mallet_source_vortex::app;
use mallet_source_vortex::link::{self, Link};
use mallet_unpack::Unpacker;

use crate::{EngineError, ManagerOptions, ScanOptions};

/// Vortex's page.
const VORTEX_PAGE: &str = "https://www.nexusmods.com/about/vortex/";
/// 7-Zip's page.
const SEVEN_ZIP_PAGE: &str = "https://www.7-zip.org/";

/// The identifier under which the Vortex link lists Mod Mallet among
/// Vortex's tools.
const MALLET_TOOL_ID: &str = "mod-mallet";

/// How long Vortex is given to close before a restart gives up.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);

/// Looks at the programs and files the setup needs besides its mods: the
/// mod manager and Mod Mallet's link to it, the game's script extender and
/// its libraries, and the tools Mod Mallet uses. Only reads.
pub fn toolbox(scan: &ScanOptions, manager: &ManagerOptions) -> Vec<Tool> {
    // Mod Organizer has no link to install or restart (ADR 0064).
    let mut found = if manager.by_hand {
        Vec::new()
    } else {
        vortex(manager)
    };
    if let (Ok(game), Some((game_dir, _))) = (scan.game(), scan.located_game_dir()) {
        // The version the game's program has now, which a Steam update
        // changes before the game is started again; else the one the
        // extender logged at the last start (ADR 0079).
        let install = locate::install_dir(game, &game_dir);
        let game_version = versions::game_version(game, &install)
            .map(|version| version.to_string())
            .or_else(|| {
                scan.extender_log
                    .clone()
                    .or_else(|| locate::extender_log(game, Some(&game_dir)))
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .and_then(|text| skse_log::parse(game, &text).game_version)
            });
        found.extend(tools::requirements(
            game,
            &game_dir,
            game_version.as_deref(),
        ));
        found.extend(version_tools(game, &install));
        // Vortex has LOOT built in; then LOOT on its own is not needed.
        if let Some(built_in) = loot_in_vortex(manager, scan.game_id()).filter(|_| !manager.by_hand)
            && let Some(loot) = found.iter_mut().find(|tool| tool.id == "loot")
        {
            *loot = built_in;
        }
    }
    // Vortex's list of tools, and Mod Mallet's place on it, do not apply to
    // Mod Organizer (ADR 0064).
    let entries = if manager.by_hand {
        Vec::new()
    } else {
        crate::installers::manager_tools(scan)
    };
    let listed = listed_tools(&entries, &found, |path| Path::new(path).is_file());
    // Whether Mod Mallet is among Vortex's tools, which the user can change
    // here; shown once the link can carry the request out or when it is
    // already there.
    let link_ready = found
        .iter()
        .any(|tool| tool.id == "mallet-link" && tool.state == ToolState::Ready);
    let own = entries.iter().find(|entry| entry.id == MALLET_TOOL_ID);
    if own.is_some() || link_ready {
        found.push(
            Tool::new(
                "mod-mallet-tool",
                ToolNeed::Optional,
                if own.is_some() {
                    ToolState::Ready
                } else {
                    ToolState::Missing
                },
            )
            .in_group(ToolGroup::ModTool)
            .with_path(own.map(|entry| entry.path.clone())),
        );
    }
    found.extend(listed);
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

/// Libraries many script extender plugins and mods need, by the file of
/// their plugin, with their identifier in the toolbox and their page. Only
/// those that are installed are shown (ADR 0079).
const LIBRARIES: [(&str, &str, &str); 3] = [
    (
        "PapyrusUtil.dll",
        "papyrus-util",
        "https://www.nexusmods.com/skyrimspecialedition/mods/13048",
    ),
    (
        "po3_PapyrusExtender.dll",
        "papyrus-extender",
        "https://www.nexusmods.com/skyrimspecialedition/mods/22854",
    ),
    (
        "JContainers64.dll",
        "jcontainers",
        "https://www.nexusmods.com/skyrimspecialedition/mods/16495",
    ),
];

/// The lines of the toolbox about versions (ADR 0079): how many of the
/// script extender's plugins fit the game version now, and the libraries
/// most plugins need, each with whether it fits.
fn version_tools(game: Game, install: &Path) -> Vec<Tool> {
    use mallet_analysis::versions::{Verdict, verdict};
    use mallet_core::facts::versions::PluginDeclaration;

    let dirs = [install.join(game.script_extender().plugin_dir)];
    let facts = versions::read(game, install, &dirs);
    let Some(game_version) = facts.game else {
        return Vec::new();
    };
    let extender = facts
        .extender
        .as_ref()
        .and_then(|extender| extender.version);
    let fits = |declaration: &PluginDeclaration| {
        verdict(declaration, game_version, extender) == Verdict::Fits
    };
    let plugins: Vec<_> = facts
        .native_plugins
        .iter()
        .filter(|plugin| {
            matches!(
                plugin.declaration,
                PluginDeclaration::VersionData(_) | PluginDeclaration::QueryOnly
            )
        })
        .collect();
    let mut tools = Vec::new();
    if !plugins.is_empty() {
        let fitting = plugins
            .iter()
            .filter(|plugin| fits(&plugin.declaration))
            .count();
        let state = if fitting == plugins.len() {
            ToolState::Ready
        } else {
            ToolState::Outdated
        };
        tools.push(
            Tool::new("extender-plugins", ToolNeed::Required, state).with_version(Some(format!(
                "{fitting}/{} → {game_version}",
                plugins.len()
            ))),
        );
    }
    if game != Game::SkyrimSe {
        return tools;
    }
    for (file, id, page) in LIBRARIES {
        let Some(plugin) = facts
            .native_plugins
            .iter()
            .find(|plugin| plugin.file_name.eq_ignore_ascii_case(file))
        else {
            continue;
        };
        let state = if fits(&plugin.declaration) {
            ToolState::Ready
        } else {
            ToolState::Outdated
        };
        tools.push(
            Tool::new(id, ToolNeed::Recommended, state)
                .with_page(page)
                .with_version(plugin.file_version.clone())
                .with_path(Some(dirs[0].join(&plugin.file_name).display().to_string())),
        );
    }
    tools
}

/// The modding tools Vortex lists for the game on its dashboard, as the
/// toolbox shows them (ADR 0063). A tool is ready when the program Vortex
/// noted is still there, missing when it is gone. Hidden tools, the game
/// and its script extender, and tools the toolbox already has (`have`) are
/// left out; of two entries for the same kind of program the first wins.
fn listed_tools(listed: &[VortexTool], have: &[Tool], exists: impl Fn(&str) -> bool) -> Vec<Tool> {
    let mut found: Vec<Tool> = Vec::new();
    // Mod Mallet's own entry, which the extension adds, is not a tool to
    // tell the user about.
    for entry in listed
        .iter()
        .filter(|entry| !entry.hidden && entry.id != MALLET_TOOL_ID)
    {
        let file_name = entry.path.rsplit(['\\', '/']).next().unwrap_or_default();
        let id = match tools::classify_listed_tool(file_name) {
            ListedTool::Covered => continue,
            ListedTool::Known(kind) => kind.to_owned(),
            ListedTool::Other => format!("vortex-tool-{}", plain(&entry.id)),
        };
        if have.iter().chain(&found).any(|tool| tool.id == id) {
            continue;
        }
        let state = if exists(&entry.path) {
            ToolState::Ready
        } else {
            ToolState::Missing
        };
        found.push(
            Tool::new(&id, ToolNeed::Optional, state)
                .in_group(ToolGroup::ModTool)
                .with_label(&entry.name)
                .with_path(Some(entry.path.clone())),
        );
    }
    found
}

/// A tool's identifier in Vortex reduced to what an identifier of ours may
/// hold.
fn plain(id: &str) -> String {
    id.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
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
/// the mod manager's installation or the game's extension it downloaded,
/// or `None` if it has none. A game Vortex brings no picture of may show
/// that of a game it is made from (Enderal Special Edition, ADR 0078).
pub fn game_art(game_id: &str) -> Option<String> {
    let vortex_data_dir = dirs::config_dir().map(|dir| dir.join("Vortex"));
    let related = mallet_game_bethesda::Game::from_id(game_id)
        .map_or(&[][..], mallet_game_bethesda::Game::related_art);
    let bytes = std::iter::once(game_id)
        .chain(related.iter().copied())
        .find_map(|id| app::game_art(id, vortex_data_dir.as_deref()))?;
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

    fn entry(id: &str, name: &str, path: &str, hidden: bool) -> VortexTool {
        VortexTool {
            id: id.to_owned(),
            name: name.to_owned(),
            path: path.to_owned(),
            custom: true,
            hidden,
        }
    }

    #[test]
    fn vortexs_tools_become_mod_tools_with_what_is_on_disk() {
        let listed = [
            entry(
                "a1",
                "SSEEdit (Create or Patch)",
                r"C:\Made Up\SSEEdit.exe",
                false,
            ),
            entry("a2", "SSEEdit again", r"C:\Made Up\SSEEdit64.exe", false),
            entry(
                "b1",
                "Quick Clean",
                r"C:\Made Up\SSEEditQuickAutoClean.exe",
                false,
            ),
            entry(
                "c1",
                "Script Extender",
                r"C:\Made Up\skse64_loader.exe",
                false,
            ),
            entry("d1", "Hidden Editor", r"C:\Made Up\Wrye Bash.exe", true),
            entry(
                "mod-mallet",
                "Mod Mallet",
                r"C:\Made Up\mod-mallet.exe",
                false,
            ),
            entry("e1", "Some Tool", r"C:\Made Up\Some Tool.exe", false),
            entry("f1", "Kit", r"C:\Made Up\CreationKit.exe", false),
        ];
        let have = [Tool::new(
            "creation-kit",
            ToolNeed::Optional,
            ToolState::Missing,
        )];
        let tools = listed_tools(&listed, &have, |path| !path.ends_with("Clean.exe"));
        let seen: Vec<(&str, ToolState)> = tools
            .iter()
            .map(|tool| (tool.id.as_str(), tool.state))
            .collect();
        assert_eq!(
            seen,
            [
                ("xedit", ToolState::Ready),
                ("xedit-clean", ToolState::Missing),
                ("vortex-tool-e1", ToolState::Ready),
            ]
        );
        assert_eq!(tools[0].label.as_deref(), Some("SSEEdit (Create or Patch)"));
        assert!(tools.iter().all(|tool| tool.group == ToolGroup::ModTool));
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
