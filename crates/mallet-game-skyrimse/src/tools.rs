//! What Skyrim Special Edition needs besides the mods: the script
//! extender, the libraries most of its plugins need, and the tools that are
//! used to make or check mods.

use std::path::Path;

use mallet_core::{Tool, ToolGroup, ToolNeed, ToolState};

use crate::swap::SWAPPER_PLUGIN;

/// The script extender's page.
const SKSE_PAGE: &str = "https://skse.silverlock.org/";
/// Address Library for SKSE Plugins on Nexus Mods.
const ADDRESS_LIBRARY_PAGE: &str = "https://www.nexusmods.com/skyrimspecialedition/mods/32444";
/// Base Object Swapper on Nexus Mods.
const SWAPPER_PAGE: &str = "https://www.nexusmods.com/skyrimspecialedition/mods/60805";
/// Microsoft's page of the Visual C++ runtime.
const VC_RUNTIME_PAGE: &str =
    "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist";
/// The Creation Kit on Steam.
const CREATION_KIT_PAGE: &str = "https://store.steampowered.com/app/1946180/";
/// LOOT's page.
const LOOT_PAGE: &str = "https://loot.github.io/";

/// Looks at what the game in `game_dir` needs besides its mods.
/// `game_version` is the version the game last ran in, as the script
/// extender logged it (`1.7.104`); without it, versions are not compared.
pub fn requirements(game_dir: &Path, game_version: Option<&str>) -> Vec<Tool> {
    let plugins = game_dir.join("Data").join("SKSE").join("Plugins");
    vec![
        script_extender(game_dir, game_version),
        address_library(&plugins, game_version),
        vc_runtime(),
        swapper(game_dir),
        creation_kit(game_dir),
        loot(),
    ]
}

/// SKSE: its loader, and its library for the game's version.
fn script_extender(game_dir: &Path, game_version: Option<&str>) -> Tool {
    let loader = game_dir.join("skse64_loader.exe");
    let tool = |state| {
        Tool::new("skse", ToolNeed::Required, state)
            .with_page(SKSE_PAGE)
            .with_path(Some(loader.display().to_string()))
    };
    if !loader.is_file() {
        return tool(ToolState::Missing).with_path(None);
    }
    let Some(version) = game_version else {
        return tool(ToolState::Ready);
    };
    // SKSE ships one library per game version it supports, named after it.
    let library = game_dir.join(format!("skse64_{}.dll", version.replace('.', "_")));
    let state = if library.is_file() {
        ToolState::Ready
    } else {
        ToolState::Outdated
    };
    tool(state).with_version(Some(format!("→ {version}")))
}

/// Address Library: one database file per game version. Files for 1.5 are
/// called `version-…`, those for 1.6 and later `versionlib-…`.
fn address_library(plugins: &Path, game_version: Option<&str>) -> Tool {
    let names: Vec<String> = std::fs::read_dir(plugins).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| {
                    let path = Path::new(name);
                    name.to_ascii_lowercase().starts_with("version")
                        && path
                            .extension()
                            .is_some_and(|extension| extension.eq_ignore_ascii_case("bin"))
                })
                .collect()
        },
    );
    let tool = |state| {
        Tool::new("address-library", ToolNeed::Recommended, state).with_page(ADDRESS_LIBRARY_PAGE)
    };
    if names.is_empty() {
        return tool(ToolState::Missing);
    }
    let Some(version) = game_version else {
        return tool(ToolState::Ready);
    };
    let dashed = version.replace('.', "-");
    let wanted = [
        format!("versionlib-{dashed}-0.bin"),
        format!("version-{dashed}-0.bin"),
    ];
    let found = names
        .iter()
        .find(|name| wanted.iter().any(|want| name.eq_ignore_ascii_case(want)));
    match found {
        Some(name) => tool(ToolState::Ready)
            .with_version(Some(version.to_owned()))
            .with_path(Some(plugins.join(name).display().to_string())),
        None => tool(ToolState::Outdated),
    }
}

/// Base Object Swapper, which Mod Mallet's patches use.
fn swapper(game_dir: &Path) -> Tool {
    let path = game_dir.join("Data").join(SWAPPER_PLUGIN);
    let state = if path.is_file() {
        ToolState::Ready
    } else {
        ToolState::Missing
    };
    Tool::new("base-object-swapper", ToolNeed::Optional, state)
        .with_page(SWAPPER_PAGE)
        .with_path((state == ToolState::Ready).then(|| path.display().to_string()))
}

/// Bethesda's editor, used to make and inspect mods.
fn creation_kit(game_dir: &Path) -> Tool {
    let path = game_dir.join("CreationKit.exe");
    let state = if path.is_file() {
        ToolState::Ready
    } else {
        ToolState::Missing
    };
    Tool::new("creation-kit", ToolNeed::Optional, state)
        .in_group(ToolGroup::Helper)
        .with_page(CREATION_KIT_PAGE)
        .with_path((state == ToolState::Ready).then(|| path.display().to_string()))
}

/// LOOT on its own. Vortex sorts plugins with LOOT's rules by itself; the
/// program adds a view of its messages about each plugin.
fn loot() -> Tool {
    let candidates = [
        dirs::data_local_dir().map(|dir| dir.join("LOOT")),
        std::env::var_os("ProgramFiles").map(|dir| Path::new(&dir).join("LOOT").join("LOOT.exe")),
    ];
    let found = candidates.into_iter().flatten().find(|path| path.exists());
    let state = if found.is_some() {
        ToolState::Ready
    } else {
        ToolState::Missing
    };
    Tool::new("loot", ToolNeed::Optional, state)
        .in_group(ToolGroup::Helper)
        .with_page(LOOT_PAGE)
        .with_path(found.map(|path| path.display().to_string()))
}

/// The Visual C++ runtime, which nearly every script extender plugin is
/// built against.
fn vc_runtime() -> Tool {
    let version = vc_runtime_version();
    let state = if version.is_some() {
        ToolState::Ready
    } else {
        ToolState::Missing
    };
    Tool::new("vc-runtime", ToolNeed::Required, state)
        .with_page(VC_RUNTIME_PAGE)
        .with_version(version)
}

#[cfg(windows)]
fn vc_runtime_version() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};

    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64",
            KEY_READ | KEY_WOW64_64KEY,
        )
        .ok()?;
    let installed: u32 = key.get_value("Installed").ok()?;
    if installed != 1 {
        return None;
    }
    let version: String = key.get_value("Version").ok()?;
    Some(version.trim_start_matches('v').to_owned())
}

#[cfg(not(windows))]
fn vc_runtime_version() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Data/SKSE/Plugins")).unwrap();
        dir
    }

    fn state_of(tools: &[Tool], id: &str) -> ToolState {
        tools.iter().find(|tool| tool.id == id).unwrap().state
    }

    #[test]
    fn an_empty_game_lacks_the_extender_and_its_library() {
        let dir = game();
        let tools = requirements(dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "skse"), ToolState::Missing);
        assert_eq!(state_of(&tools, "address-library"), ToolState::Missing);
        assert_eq!(state_of(&tools, "base-object-swapper"), ToolState::Missing);
    }

    #[test]
    fn the_extender_must_have_a_library_for_the_game_version() {
        let dir = game();
        std::fs::write(dir.path().join("skse64_loader.exe"), b"").unwrap();
        std::fs::write(dir.path().join("skse64_1_6_1170.dll"), b"").unwrap();
        let tools = requirements(dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "skse"), ToolState::Outdated);

        std::fs::write(dir.path().join("skse64_1_7_104.dll"), b"").unwrap();
        let tools = requirements(dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "skse"), ToolState::Ready);
    }

    #[test]
    fn the_address_library_must_cover_the_game_version() {
        let dir = game();
        let plugins = dir.path().join("Data/SKSE/Plugins");
        std::fs::write(plugins.join("versionlib-1-6-1170-0.bin"), b"").unwrap();
        let tools = requirements(dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Outdated);

        std::fs::write(plugins.join("versionlib-1-7-104-0.bin"), b"").unwrap();
        let tools = requirements(dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Ready);

        let tools = requirements(dir.path(), Some("1.5.97"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Outdated);
        std::fs::write(plugins.join("version-1-5-97-0.bin"), b"").unwrap();
        let tools = requirements(dir.path(), Some("1.5.97"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Ready);
    }
}
