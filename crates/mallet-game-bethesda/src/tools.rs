//! What a game needs besides the mods: the script extender, the libraries
//! most of its plugins need, and the tools that are used to make or check
//! mods.

use std::path::Path;

use mallet_core::{Tool, ToolGroup, ToolNeed, ToolState};

use crate::game::{AddressLibrary, Editor, Game};
use crate::swap::SWAPPER_PLUGIN;

/// UE4SS's page.
const UE4SS_PAGE: &str = "https://github.com/UE4SS-RE/RE-UE4SS";

/// Base Object Swapper on Nexus Mods.
const SWAPPER_PAGE: &str = "https://www.nexusmods.com/skyrimspecialedition/mods/60805";
/// Microsoft's page of the Visual C++ runtime.
const VC_RUNTIME_PAGE: &str =
    "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist";
/// LOOT's page.
const LOOT_PAGE: &str = "https://loot.github.io/";

/// Looks at what `game`, installed in `game_dir`, needs besides its mods.
/// `game_version` is the version the game last ran in, as the script
/// extender logged it (`1.7.104`); without it, versions are not compared.
pub fn requirements(game: Game, game_dir: &Path, game_version: Option<&str>) -> Vec<Tool> {
    // The extender and the editor lie in the installation, which for
    // Oblivion Remastered is above the folder the checks read (ADR 0065).
    let install = crate::locate::install_dir(game, game_dir);
    let plugins = install.join(game.script_extender().plugin_dir);
    let mut tools = vec![script_extender(game, &install, game_version)];
    if let Some(library) = game.address_library() {
        tools.push(address_library(library, &plugins, game_version));
    }
    tools.push(vc_runtime(game.is_64_bit()));
    if game.has_object_swapper() {
        tools.push(swapper(game_dir));
    }
    if let Some(program) = game.ue4ss() {
        tools.push(ue4ss(&install.join(program)));
    }
    if let Some(editor) = game.editor() {
        tools.push(editor_tool(editor, &install));
    }
    tools.push(loot());
    tools
}

/// What a mod manager's list of tools makes of one program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListedTool {
    /// A modding tool Mod Mallet has words for, by the identifier of its
    /// texts: `xedit`, `xedit-clean`, `wrye-bash`, `fnis`, `bodyslide` or
    /// `creation-kit`.
    Known(&'static str),
    /// The game itself or its script extender, which the toolbox already
    /// shows on its own.
    Covered,
    /// A program Mod Mallet knows nothing about.
    Other,
}

/// Tells what the program `file_name` is that a mod manager lists among
/// the tools of a game, by the name of its file.
pub fn classify_listed_tool(file_name: &str) -> ListedTool {
    let name = file_name.to_ascii_lowercase();
    let stem = name.strip_suffix(".exe").unwrap_or(&name);
    let game_program = [
        Game::SkyrimSe,
        Game::SkyrimVr,
        Game::Skyrim,
        Game::Fallout4,
        Game::Fallout4Vr,
        Game::FalloutNv,
        Game::Fallout3,
        Game::Oblivion,
    ]
    .iter()
    .any(|game| game.executable().eq_ignore_ascii_case(&name));
    if game_program || stem.ends_with("_loader") || stem.ends_with("launcher") {
        return ListedTool::Covered;
    }
    // xEdit is called after the game it is for: SSEEdit, FO4Edit, TES5Edit.
    let xedit = stem.strip_suffix("64").unwrap_or(stem).ends_with("edit");
    if stem.ends_with("quickautoclean") {
        ListedTool::Known("xedit-clean")
    } else if xedit {
        ListedTool::Known("xedit")
    } else if stem.starts_with("wrye bash") || stem.starts_with("wrye_bash") {
        ListedTool::Known("wrye-bash")
    } else if stem.starts_with("generatefnis") {
        ListedTool::Known("fnis")
    } else if stem.starts_with("bodyslide") {
        ListedTool::Known("bodyslide")
    } else if stem == "creationkit" {
        ListedTool::Known("creation-kit")
    } else {
        ListedTool::Other
    }
}

/// The script extender: its loader, and its library for the game's
/// version where it ships one per version.
fn script_extender(game: Game, game_dir: &Path, game_version: Option<&str>) -> Tool {
    let extender = game.script_extender();
    let loader = game_dir.join(extender.loader);
    let tool = |state| {
        Tool::new(extender.tool, ToolNeed::Required, state)
            .with_page(extender.page)
            .with_path(Some(loader.display().to_string()))
    };
    if !loader.is_file() {
        return tool(ToolState::Missing).with_path(None);
    }
    let (Some(version), Some(prefix)) = (game_version, extender.versioned_library) else {
        return tool(ToolState::Ready);
    };
    // The extender ships one library per game version it supports, named
    // after it.
    // The libraries lie beside the loader: in the installation, or for
    // Oblivion Remastered beside its Unreal program.
    let library = loader
        .parent()
        .unwrap_or(game_dir)
        .join(format!("{prefix}_{}.dll", version.replace('.', "_")));
    let state = if library.is_file() {
        ToolState::Ready
    } else {
        ToolState::Outdated
    };
    tool(state).with_version(Some(format!("→ {version}")))
}

/// Address Library: one database file per game version. Files for
/// Skyrim 1.5, Skyrim VR and Fallout 4 are called `version-…`, those for
/// Skyrim 1.6 and later `versionlib-…`.
fn address_library(library: AddressLibrary, plugins: &Path, game_version: Option<&str>) -> Tool {
    let names: Vec<String> = std::fs::read_dir(plugins).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| {
                    let path = Path::new(name);
                    name.to_ascii_lowercase().starts_with("version")
                        && path.extension().is_some_and(|extension| {
                            extension.eq_ignore_ascii_case(library.extension)
                        })
                })
                .collect()
        },
    );
    let tool =
        |state| Tool::new(library.tool, ToolNeed::Recommended, state).with_page(library.page);
    if names.is_empty() {
        return tool(ToolState::Missing);
    }
    let Some(version) = game_version else {
        return tool(ToolState::Ready);
    };
    let dashed = version.replace('.', "-");
    let wanted = [
        format!("versionlib-{dashed}-0.{}", library.extension),
        format!("version-{dashed}-0.{}", library.extension),
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

/// UE4SS, which loads the Unreal Engine side's logic mods (`LogicMods`)
/// and scripts, at `library`.
fn ue4ss(library: &Path) -> Tool {
    let state = if library.is_file() {
        ToolState::Ready
    } else {
        ToolState::Missing
    };
    Tool::new("ue4ss", ToolNeed::Optional, state)
        .with_page(UE4SS_PAGE)
        .with_path((state == ToolState::Ready).then(|| library.display().to_string()))
}

/// Bethesda's editor for the game, used to make and inspect mods.
fn editor_tool(editor: Editor, game_dir: &Path) -> Tool {
    let path = game_dir.join(editor.program);
    let state = if path.is_file() {
        ToolState::Ready
    } else {
        ToolState::Missing
    };
    Tool::new(editor.tool, ToolNeed::Optional, state)
        .in_group(ToolGroup::Helper)
        .with_page(editor.page)
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
/// built against: the 64-bit one for the newer games, the 32-bit one for
/// the older.
fn vc_runtime(wide: bool) -> Tool {
    let version = vc_runtime_version(wide);
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
fn vc_runtime_version(wide: bool) -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};

    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            if wide {
                r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64"
            } else {
                r"SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x86"
            },
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
fn vc_runtime_version(_wide: bool) -> Option<String> {
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
    fn listed_tools_are_told_apart_by_their_file_names() {
        use ListedTool::{Covered, Known, Other};
        let cases = [
            ("SSEEdit.exe", Known("xedit")),
            ("FO4Edit64.exe", Known("xedit")),
            ("TES5Edit.exe", Known("xedit")),
            ("SSEEditQuickAutoClean.exe", Known("xedit-clean")),
            ("Wrye Bash.exe", Known("wrye-bash")),
            ("GenerateFNISForUsers.exe", Known("fnis")),
            ("BodySlide x64.exe", Known("bodyslide")),
            ("CreationKit.exe", Known("creation-kit")),
            ("skse64_loader.exe", Covered),
            ("SkyrimSE.exe", Covered),
            ("SkyrimSELauncher.exe", Covered),
            ("Synthesis.exe", Other),
            ("", Other),
        ];
        for (name, expected) in cases {
            assert_eq!(classify_listed_tool(name), expected, "{name}");
        }
    }

    #[test]
    fn an_empty_game_lacks_the_extender_and_its_library() {
        let dir = game();
        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "skse"), ToolState::Missing);
        assert_eq!(state_of(&tools, "address-library"), ToolState::Missing);
        assert_eq!(state_of(&tools, "base-object-swapper"), ToolState::Missing);
    }

    #[test]
    fn the_extender_must_have_a_library_for_the_game_version() {
        let dir = game();
        std::fs::write(dir.path().join("skse64_loader.exe"), b"").unwrap();
        std::fs::write(dir.path().join("skse64_1_6_1170.dll"), b"").unwrap();
        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "skse"), ToolState::Outdated);

        std::fs::write(dir.path().join("skse64_1_7_104.dll"), b"").unwrap();
        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "skse"), ToolState::Ready);
    }

    #[test]
    fn new_vegas_needs_its_own_extender_and_editor_and_no_address_library() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("nvse_loader.exe"), b"").unwrap();
        let tools = requirements(Game::FalloutNv, dir.path(), None);
        let ids: Vec<&str> = tools.iter().map(|tool| tool.id.as_str()).collect();
        assert_eq!(ids, ["nvse", "vc-runtime", "geck", "loot"]);
        assert_eq!(state_of(&tools, "nvse"), ToolState::Ready);
    }

    #[test]
    fn oblivion_remastered_finds_obse64_and_ue4ss_beside_its_unreal_program() {
        let install = tempfile::tempdir().unwrap();
        let root = install
            .path()
            .join("OblivionRemastered/Content/Dev/ObvData");
        std::fs::create_dir_all(root.join("Data")).unwrap();
        let tools = requirements(Game::OblivionRemastered, &root, None);
        let ids: Vec<&str> = tools.iter().map(|tool| tool.id.as_str()).collect();
        assert_eq!(ids, ["obse64", "vc-runtime", "ue4ss", "loot"]);
        assert_eq!(state_of(&tools, "obse64"), ToolState::Missing);
        assert_eq!(state_of(&tools, "ue4ss"), ToolState::Missing);

        let binaries = install.path().join("OblivionRemastered/Binaries/Win64");
        std::fs::create_dir_all(binaries.join("ue4ss")).unwrap();
        std::fs::write(binaries.join("obse64_loader.exe"), b"").unwrap();
        std::fs::write(binaries.join("ue4ss/UE4SS.dll"), b"").unwrap();
        let tools = requirements(Game::OblivionRemastered, &root, None);
        assert_eq!(state_of(&tools, "obse64"), ToolState::Ready);
        assert_eq!(state_of(&tools, "ue4ss"), ToolState::Ready);
    }

    #[test]
    fn the_address_library_must_cover_the_game_version() {
        let dir = game();
        let plugins = dir.path().join("Data/SKSE/Plugins");
        std::fs::write(plugins.join("versionlib-1-6-1170-0.bin"), b"").unwrap();
        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Outdated);

        std::fs::write(plugins.join("versionlib-1-7-104-0.bin"), b"").unwrap();
        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.7.104"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Ready);

        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.5.97"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Outdated);
        std::fs::write(plugins.join("version-1-5-97-0.bin"), b"").unwrap();
        let tools = requirements(Game::SkyrimSe, dir.path(), Some("1.5.97"));
        assert_eq!(state_of(&tools, "address-library"), ToolState::Ready);
    }
}
