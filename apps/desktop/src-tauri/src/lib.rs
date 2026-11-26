//! The Mod Mallet desktop app.
//!
//! The app is a thin shell, like the command line. The commands below call
//! `mallet-engine` and hand the result, put into words by `mallet-messages`,
//! to the web frontend. Anything that decides something about a setup
//! belongs in those crates, not here.
//!
//! The frontend's view of these commands is `apps/desktop/src/api.ts`. The
//! shape of what they return is generated from [`RenderedReport`] into
//! `apps/desktop/src/generated/rendered-report.ts` (ADR 0012).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mallet_core::{Action, ArgValue, GameChoice, LocationRole, ObjectRef, ScanReport};
use mallet_engine::{ManagerOptions, Settings};
use mallet_messages::{
    Locale, Messages, RenderedLink, RenderedOutcome, RenderedPlaces, RenderedReport,
    RenderedToolbox,
};
use tauri::{Emitter, Manager};

/// Examines the setup and returns what is wrong with it, in the language
/// `lang`. The result is remembered for [`last_report`].
// `async` here means "not on the main thread": a scan must not freeze the
// window. Tauri hands command arguments over by value.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn scan(app: tauri::AppHandle, lang: String) -> Result<RenderedReport, String> {
    let options = settings()?.scan_options();
    // The engine rations its reports; they go on to the window as events.
    let forward = |progress: mallet_core::ScanProgress| {
        // A window that is gone cannot be told; the scan goes on.
        let _ = app.emit("scan-progress", progress);
    };
    let progress = mallet_core::Progress::to(&forward);
    let report = match mallet_engine::default_cache_path() {
        Some(cache) => mallet_engine::scan_and_store_with_progress(&options, &cache, &progress),
        None => mallet_engine::scan_with_progress(&options, &progress),
    }
    .map_err(|error| error.to_string())?;
    Ok(render(&report, &lang))
}

/// Returns the result of the last scan in the language `lang`, or nothing
/// if no scan has been stored yet.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn last_report(lang: String) -> Result<Option<RenderedReport>, String> {
    let Some(cache) = mallet_engine::default_cache_path() else {
        return Ok(None);
    };
    let report = mallet_engine::last_report(&settings()?.scan_options(), &cache)
        .map_err(|error| error.to_string())?;
    Ok(report.map(|report| render(&report, &lang)))
}

/// Hands an action to the mod manager and says, in the language `lang`,
/// what became of it. Only ever called because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn run_action(action: Action, lang: String) -> RenderedOutcome {
    let outcome = mallet_engine::run_action(&action, &manager_options());
    messages(&lang).render_outcome(&outcome)
}

/// Says whether the link to the mod manager is installed and answering.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn manager_link(lang: String) -> RenderedLink {
    let link = mallet_engine::manager_link(&manager_options());
    messages(&lang).render_link(link.installed, link.running, link.restart_needed)
}

/// Installs the link into the mod manager and returns a sentence saying
/// where it went. Only ever called because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn install_manager_link(lang: String) -> Result<String, String> {
    let path = mallet_engine::install_manager_link(&manager_options())
        .map_err(|error| error.to_string())?;
    let path = ArgValue::from(path.display().to_string());
    Ok(messages(&lang).text("link-installed", &[("path", path)]))
}

/// Asks Vortex, through the link, to list Mod Mallet among its tools (or
/// to take it off the list) and says what became of it. Only ever called
/// because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn set_vortex_tool(add: bool, lang: String) -> RenderedOutcome {
    let outcome = mallet_engine::set_mallet_tool(&manager_options(), add);
    messages(&lang).render_outcome(&outcome)
}

/// The places a scan would read and how each was found, in the language
/// `lang`, and whether the user has agreed to them. Nothing is read to
/// find this out.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn places(lang: String) -> Result<RenderedPlaces, String> {
    Ok(render_places(&settings()?, &lang))
}

/// The places a scan would read with the mod manager `manager` (`vortex`,
/// `mo2`) and its instance at `instance`, in the language `lang`. Nothing
/// is kept and nothing is read: the user is still choosing (ADR 0064).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn places_for_manager(
    manager: String,
    instance: Option<String>,
    lang: String,
) -> Result<RenderedPlaces, String> {
    let instance = instance
        .map(|path| path.trim().to_owned())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    Ok(render_places(
        &settings()?.with_manager(&manager, instance),
        &lang,
    ))
}

/// Remembers the places the user named, the mod manager they chose, and
/// that they agreed to the rest. `named` holds only what the user chose;
/// every other place is found by its rule. Only ever called because the
/// user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn confirm_places(
    named: BTreeMap<LocationRole, String>,
    manager: Option<String>,
    lang: String,
) -> Result<RenderedPlaces, String> {
    // What else the user decided stays as it was.
    let settings = change_settings(|settings| {
        settings.confirmed = true;
        if let Some(manager) = manager {
            settings.manager = Some(manager);
        }
        settings.places = named
            .into_iter()
            .map(|(role, path)| (role, path.trim().to_owned()))
            .filter(|(_, path)| !path.is_empty())
            .map(|(role, path)| (role, PathBuf::from(path)))
            .collect();
    })?;
    Ok(render_places(&settings, &lang))
}

/// The identities of the findings the user has set aside.
#[tauri::command(async)]
fn ignored() -> Result<Vec<String>, String> {
    Ok(settings()?.ignored.into_iter().collect())
}

/// Sets a finding aside, or brings it back, and returns the findings set
/// aside afterwards. Only ever called because the user pressed a button.
#[tauri::command(async)]
fn set_ignored(id: String, ignored: bool) -> Result<Vec<String>, String> {
    let settings = change_settings(|settings| {
        if ignored {
            settings.ignored.insert(id);
        } else {
            settings.ignored.remove(&id);
        }
    })?;
    Ok(settings.ignored.into_iter().collect())
}

/// The versions of things the user put into the game, by thing.
#[tauri::command(async)]
fn asset_choices() -> Result<BTreeMap<String, String>, String> {
    Ok(settings()?.asset_choices)
}

/// Remembers the version of a thing the user put into the game, or forgets
/// it with `None`, and returns all such choices afterwards. Only ever
/// called after the mod manager carried out the choice.
#[tauri::command(async)]
fn set_asset_choice(
    id: String,
    version: Option<String>,
) -> Result<BTreeMap<String, String>, String> {
    let settings = change_settings(|settings| {
        match version {
            Some(version) => settings.asset_choices.insert(id, version),
            None => settings.asset_choices.remove(&id),
        };
    })?;
    Ok(settings.asset_choices)
}

/// The sides the user picked in duels between mods, by duel, as kept
/// before the picks became decisions of the patches (ADR 0081): read once
/// to move them into the first patch.
#[tauri::command(async)]
fn decisions() -> Result<BTreeMap<String, u8>, String> {
    Ok(settings()?.decisions)
}

/// Remembers the side picked in a duel, or forgets it with `None`, and
/// returns all decisions afterwards. The window only forgets picks with it,
/// once they moved into a patch (ADR 0081).
#[tauri::command(async)]
fn set_decision(id: String, side: Option<u8>) -> Result<BTreeMap<String, u8>, String> {
    let settings = change_settings(|settings| {
        match side {
            Some(side) => settings.decisions.insert(id, side.min(1)),
            None => settings.decisions.remove(&id),
        };
    })?;
    Ok(settings.decisions)
}

/// The mods in the order the user likes them, the favourite first.
#[tauri::command(async)]
fn ranking() -> Result<Vec<String>, String> {
    Ok(settings()?.ranking)
}

/// Remembers the order the user likes the mods in. Only ever called
/// because the user changed it.
#[tauri::command(async)]
fn set_ranking(ranking: Vec<String>) -> Result<Vec<String>, String> {
    let settings = change_settings(|settings| settings.ranking = ranking)?;
    Ok(settings.ranking)
}

/// What the user chose for the patch that hides objects, by finding, spot
/// or object: the plugin to hide, or `None` to keep both. `None` as a
/// whole if the user has chosen nothing yet.
#[tauri::command(async)]
fn patch_choices() -> Result<Option<BTreeMap<String, Option<String>>>, String> {
    Ok(settings()?.patch_choices)
}

/// Remembers what the user chose for the patch that hides objects. Only
/// ever called because the user chose something.
#[tauri::command(async)]
fn set_patch_choices(choices: BTreeMap<String, Option<String>>) -> Result<(), String> {
    change_settings(|settings| settings.patch_choices = Some(choices))?;
    Ok(())
}

/// The user's patches for the game in use (ADR 0070); `None` until the
/// window made them.
#[tauri::command(async)]
fn patch_book() -> Result<Option<mallet_core::PatchBook>, String> {
    Ok(settings()?.patch_book)
}

/// Remembers the user's patches. Only ever called because the user decided
/// something, or made, changed or loaded a patch.
#[tauri::command(async)]
fn set_patch_book(book: mallet_core::PatchBook) -> Result<(), String> {
    change_settings(|settings| settings.patch_book = Some(book))?;
    Ok(())
}

/// What the user said about preferences between their mods for the game
/// in use (ADR 0086); `None` until they said anything.
#[tauri::command(async)]
fn preferences() -> Result<Option<mallet_core::PatchPreferences>, String> {
    Ok(settings()?.preferences)
}

/// Remembers what the user said about preferences between their mods.
/// Only ever called because the user set a preference, answered one, or
/// switched learning on or off.
#[tauri::command(async)]
fn set_preferences(preferences: mallet_core::PatchPreferences) -> Result<(), String> {
    change_settings(|settings| settings.preferences = Some(preferences))?;
    Ok(())
}

/// Remembers how healthy the setup of the game in use is, as the window
/// weighed it from the last scan and the decisions, so that the list of
/// games shows it while another game is in use (ADR 0048). Called when the
/// score changes.
#[tauri::command(async)]
fn set_last_health(health: mallet_core::LastHealth) -> Result<(), String> {
    change_settings(|settings| settings.last_health = Some(health))?;
    Ok(())
}

/// Reads the patch in an archive the user picked, by its file name and its
/// bytes. Nothing is installed or written.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn load_patch_archive(name: String, bytes: Vec<u8>) -> Result<mallet_core::LoadedPatch, String> {
    mallet_engine::load_patch(&name, &bytes).map_err(|error| {
        tracing::warn!(%error, "patch not loaded");
        error.to_string()
    })
}

/// A short text that changes when a file the scan starts from changes.
/// The window asks for it every few seconds and scans again when it
/// changed.
#[tauri::command(async)]
fn setup_fingerprint() -> Result<String, String> {
    Ok(mallet_engine::setup_fingerprint(
        &settings()?.scan_options(),
    ))
}

/// The addresses Mod Mallet opens in the browser: mod pages, and the pages
/// of the tools in the toolbox.
const OPENABLE_PAGES: [&str; 14] = [
    "https://www.nexusmods.com/",
    "https://skse.silverlock.org/",
    "https://f4se.silverlock.org/",
    "https://fose.silverlock.org/",
    "https://github.com/xNVSE/NVSE",
    "https://github.com/llde/xOBSE",
    "https://store.steampowered.com/app/202480/",
    "https://store.steampowered.com/app/1946160/",
    "https://geckwiki.com/",
    "https://cs.uesp.net/",
    "https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist",
    "https://store.steampowered.com/app/1946180/",
    "https://loot.github.io/",
    "https://www.7-zip.org/",
];

/// Opens a mod's or a tool's page in the browser. Only the pages in
/// [`OPENABLE_PAGES`] are opened, and only addresses made of ordinary
/// characters, so that nothing in a report can make the app open something
/// else.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn open_page(url: String) -> Result<(), String> {
    let ordinary = url
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/:.-_?=&%#~".contains(character));
    let allowed = OPENABLE_PAGES.iter().any(|prefix| url.starts_with(prefix));
    if !allowed || !ordinary {
        return Err(format!("not a page Mod Mallet opens: {url}"));
    }
    // The program that hands an address to the default browser. The
    // address is passed as one argument, not through a shell.
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Writes the report of what the patch hides next to it, in Mod Mallet's
/// own folder, and returns where it is. Only ever called because the user
/// created the patch or pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn save_patch_report(text: String) -> Result<String, String> {
    mallet_engine::save_patch_report(&text, &manager_options())
        .map(|path| path.display().to_string())
        .map_err(|error| {
            tracing::warn!(%error, "patch report not written");
            error.to_string()
        })
}

/// Writes `patch`, with an installer for its parts, and returns where it
/// is. Nothing is installed. Only ever called because the user pressed a
/// button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn create_patch(patch: mallet_core::Patch) -> Result<String, String> {
    mallet_engine::create_patch(&patch, &manager_options())
        .map(|created| created.path.display().to_string())
        .map_err(|error| {
            tracing::warn!(%error, "patch not written");
            error.to_string()
        })
}

/// Writes the plugin that moves objects (ADR 0060) into Mod Mallet's own
/// folder and says what it moves and what it could not. Nothing is
/// installed. Only ever called because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn create_moves(
    moves: mallet_core::moves::Moves,
) -> Result<mallet_core::moves::CreatedMoves, String> {
    mallet_engine::create_moves(&settings()?.scan_options(), &moves, &manager_options()).map_err(
        |error| {
            tracing::warn!(%error, "moves not written");
            error.to_string()
        },
    )
}

/// Writes the plugin of repaired paths (ADR 0058) into Mod Mallet's own
/// folder and says what it repairs and what it could not. Nothing is
/// installed. Only ever called because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn create_paths(
    repairs: mallet_core::walkway_repair::PathRepairs,
) -> Result<mallet_core::walkway_repair::CreatedPaths, String> {
    mallet_engine::create_paths(&settings()?.scan_options(), &repairs, &manager_options()).map_err(
        |error| {
            tracing::warn!(%error, "repaired paths not written");
            error.to_string()
        },
    )
}

/// Whether characters reach where they go once the paths over objects
/// about to be hidden are repaired (ADR 0058, 0059), asked as
/// `walkway_repair`.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn walkway_repair_reach(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    objects: Vec<ObjectRef>,
) -> Result<Option<mallet_core::reach::WalkwayReach>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::walkway_repair_reach(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        &objects,
    ))
}

/// Shows one of Mod Mallet's own folders in the file explorer: `patches`
/// or `logs`. Nothing else can be opened this way.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn open_folder(which: String) -> Result<(), String> {
    let folder = match which.as_str() {
        "patches" => mallet_engine::patch_file(&manager_options())
            .and_then(|file| file.parent().map(std::path::Path::to_path_buf)),
        "logs" => mallet_engine::default_log_dir(),
        _ => None,
    }
    .ok_or_else(|| format!("not a folder Mod Mallet opens: {which}"))?;
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(&folder)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Shows a plugin or an archive in the game's Data folder in the file
/// explorer, selected, so that the user can move it out (ADR 0073). Only
/// such a file, by its bare name, and only one that is there; nothing is
/// moved or deleted.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn show_data_file(name: String) -> Result<(), String> {
    let file = mallet_engine::data_file(&settings()?.scan_options(), &name)
        .ok_or_else(|| format!("not a file in the Data folder: {name}"))?;
    tracing::info!(file = %file.display(), "showing a file in the Data folder");
    reveal(&file).map_err(|error| error.to_string())
}

/// Opens the folder of `file` in the file explorer with the file selected,
/// where the system can; elsewhere the folder.
fn reveal(file: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Explorer reads `/select,` and the path as one argument of its
        // own making; a path never contains a quotation mark on Windows.
        std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", file.display()))
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let folder = file.parent().unwrap_or(file);
        std::process::Command::new(opener)
            .arg(folder)
            .spawn()
            .map(|_| ())
    }
}

/// What the setup needs besides its mods, and how each of those stands.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn toolbox(lang: String) -> Result<RenderedToolbox, String> {
    let tools = mallet_engine::toolbox(&settings()?.scan_options(), &manager_options());
    Ok(messages(&lang).render_toolbox(&tools))
}

/// A picture of a worldspace from above, from the game's terrain textures.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn map_image(world: String) -> Result<Option<mallet_core::MapImage>, String> {
    Ok(mallet_engine::map_image(
        &settings()?.scan_options(),
        &world,
    ))
}

/// Everything placed around a point of a worldspace (ADR 0038).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn area_objects(
    world: String,
    x: f32,
    y: f32,
    radius: f32,
) -> Result<Option<mallet_core::mesh::AreaObjects>, String> {
    Ok(mallet_engine::area_objects(
        &settings()?.scan_options(),
        &world,
        [x, y],
        radius,
    ))
}

/// Everything placed in an interior cell, by its editor ID (ADR 0042).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn interior_objects(cell: String) -> Result<Option<mallet_core::mesh::AreaObjects>, String> {
    Ok(mallet_engine::interior_objects(
        &settings()?.scan_options(),
        &cell,
    ))
}

/// The game's ground of every cell of a small worldspace, for a world that
/// has no level of detail.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn world_land(
    world: String,
    west: i32,
    south: i32,
    east: i32,
    north: i32,
) -> Result<Option<mallet_core::mesh::AreaLand>, String> {
    Ok(mallet_engine::world_land(
        &settings()?.scan_options(),
        &world,
        [west, south, east, north],
    ))
}

/// The game's ground of the cells around a point of a worldspace (ADR 0041).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn area_land(
    world: String,
    x: f32,
    y: f32,
    radius: f32,
) -> Result<Option<mallet_core::mesh::AreaLand>, String> {
    Ok(mallet_engine::area_land(
        &settings()?.scan_options(),
        &world,
        [x, y],
        radius,
    ))
}

/// Why the game relies on each placed object, and whether it can be hidden
/// (ADR 0046).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn object_anchors(
    world_or_cell: String,
    objects: Vec<ObjectRef>,
) -> Result<Option<Vec<mallet_core::anchors::ObjectAnchors>>, String> {
    Ok(mallet_engine::object_anchors(
        &settings()?.scan_options(),
        &world_or_cell,
        &objects,
    ))
}

/// What repairing the paths over objects about to be hidden does (ADR
/// 0058): the triangles taken out and those laid instead, as for
/// `hidden_walkways`.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn walkway_repair(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    objects: Vec<ObjectRef>,
) -> Result<Option<mallet_core::walkway_repair::WalkwayRepair>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::walkway_repair(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        &objects,
    ))
}

/// The paths characters walk on that run over objects about to be hidden
/// (ADR 0051): in a worldspace near the point `x`, `y`, or, without a
/// point, in an interior cell by its editor ID.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn hidden_walkways(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    objects: Vec<ObjectRef>,
) -> Result<Option<mallet_core::mesh::HiddenWalkways>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::hidden_walkways(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        &objects,
    ))
}

/// Where characters walk in the worldspace `world_or_cell` within `radius`
/// game units of the point `x`, `y`, or, without a point, in that interior
/// cell (ADR 0071): who made each piece of the paths, where they run over
/// `hidden`, and where characters sit, lean, sleep or work.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn area_paths(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    radius: f32,
    hidden: Vec<ObjectRef>,
) -> Result<Option<mallet_core::area_paths::AreaPaths>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::area_paths(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        radius,
        &hidden,
    ))
}

/// Whether characters still reach where they go once `objects` are hidden
/// and the paths over them cut out (ADR 0059): in a worldspace near the
/// point `x`, `y`, or, without a point, in an interior cell by its editor
/// ID.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn walkway_reach(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    objects: Vec<ObjectRef>,
) -> Result<Option<mallet_core::reach::WalkwayReach>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::walkway_reach(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        &objects,
    ))
}

/// Whether characters reach the door `keep`, kept where stacked doors are
/// merged, and step through it once the patch hides `hidden` and makes the
/// plan's `changes` (ADR 0088): in a worldspace near the point `x`, `y`,
/// or, without a point, in an interior cell by its editor ID.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn door_reach(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    hidden: Vec<ObjectRef>,
    keep: ObjectRef,
    changes: Vec<mallet_core::doors::RecordChange>,
) -> Result<Option<mallet_core::reach::DoorReach>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::door_reach(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        &hidden,
        &keep,
        &changes,
    ))
}

/// Whether characters can reach places where they do something, where
/// they stand or would stand after a move (ADR 0069): in a worldspace near
/// the point `x`, `y`, or, without a point, in an interior cell by its
/// editor ID. With `hidden`, as if the patch hid those objects.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn spots_reach(
    world_or_cell: String,
    x: Option<f32>,
    y: Option<f32>,
    spots: Vec<mallet_core::action_places::SpotAsked>,
    hidden: Vec<ObjectRef>,
) -> Result<Option<mallet_core::action_places::SpotsReach>, String> {
    let centre = x.zip(y).map(|(x, y)| [x, y]);
    Ok(mallet_engine::spots_reach(
        &settings()?.scan_options(),
        &world_or_cell,
        centre,
        &spots,
        &hidden,
    ))
}

/// Every thing of the setup worth looking at, for the assets page. The
/// first call for a setup reads every plugin and takes a few seconds.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn catalogue(lang: String) -> Result<Option<mallet_core::catalogue::Catalogue>, String> {
    Ok(mallet_engine::catalogue(&settings()?.scan_options(), &lang)
        .map(|catalogue| (*catalogue).clone()))
}

/// Every version of a thing of the catalogue whose models are `models`:
/// what the game and each mod make it look like.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn asset_versions(
    models: Vec<String>,
) -> Result<Option<mallet_core::catalogue::AssetVersions>, String> {
    Ok(mallet_engine::asset_versions(
        &settings()?.scan_options(),
        &models,
    ))
}

/// The mesh at `path` as the version `version` of a thing makes it look.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn asset_view(
    path: String,
    version: String,
) -> Result<Option<mallet_core::mesh::MeshView>, String> {
    Ok(mallet_engine::asset_view(
        &settings()?.scan_options(),
        &path,
        &version,
    ))
}

/// A mesh of the setup in its own units, with its textures named by path.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn area_mesh(path: String) -> Result<Option<mallet_core::mesh::LodMesh>, String> {
    Ok(mallet_engine::area_mesh(&settings()?.scan_options(), &path))
}

/// A texture of the setup as a PNG `data:` address.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn area_texture(path: String, max_side: u32) -> Result<Option<String>, String> {
    Ok(mallet_engine::area_texture(
        &settings()?.scan_options(),
        &path,
        max_side,
    ))
}

/// The blocks of the game's level of detail a worldspace has (ADR 0037).
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn lod_tiles(world: String) -> Result<mallet_core::mesh::LodTiles, String> {
    Ok(mallet_engine::lod_tiles(
        &settings()?.scan_options(),
        &world,
    ))
}

/// The land of one block of level of detail, or its objects.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn lod_mesh(
    world: String,
    level: u32,
    x: i32,
    y: i32,
    objects: bool,
) -> Result<Option<mallet_core::mesh::LodMesh>, String> {
    Ok(mallet_engine::lod_mesh(
        &settings()?.scan_options(),
        &world,
        level,
        x,
        y,
        objects,
    ))
}

/// A texture the level of detail uses, as a PNG `data:` address.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn lod_texture(world: String, path: String, max_side: u32) -> Result<Option<String>, String> {
    Ok(mallet_engine::lod_texture(
        &settings()?.scan_options(),
        mallet_engine::default_cache_path().as_deref(),
        &world,
        &path,
        max_side,
    ))
}

/// Every version of a file of the setup, as pictures where it is a texture.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn file_preview(path: String) -> Result<Option<mallet_core::FilePreview>, String> {
    Ok(mallet_engine::file_preview(
        &settings()?.scan_options(),
        &path,
    ))
}

/// Every version of files other than textures and meshes, compared: text,
/// size, date, whether the bytes are the same, a script's functions.
/// `mods` names the mod manager's folders of the mods concerned. Only
/// reads.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn compare_files(
    paths: Vec<String>,
    mods: Vec<String>,
) -> Result<Option<Vec<mallet_core::compare::FileComparison>>, String> {
    Ok(mallet_engine::compare_files(
        &settings()?.scan_options(),
        &paths,
        &mods,
    ))
}

/// A version of a mesh of the setup, ready to draw: the one the game uses
/// unless `version` names another.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn mesh_view(
    path: String,
    version: Option<usize>,
) -> Result<Option<mallet_core::mesh::MeshView>, String> {
    Ok(mallet_engine::mesh_view(
        &settings()?.scan_options(),
        &path,
        version,
    ))
}

/// Small pictures of mesh versions for lists (ADR 0085), from the cache
/// where they were drawn before. Only reads the setup.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn mesh_thumbnails(
    requests: Vec<mallet_core::mesh::MeshThumbnailRequest>,
    side: u32,
) -> Result<Vec<mallet_core::mesh::MeshThumbnail>, String> {
    Ok(mallet_engine::mesh_thumbnails(
        &settings()?.scan_options(),
        mallet_engine::default_cache_path().as_deref(),
        &requests,
        side,
    ))
}

/// The picture of a game, from the mod manager's installation.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn game_art(game: String) -> Option<String> {
    mallet_engine::game_art(&game)
}

/// Closes Vortex and starts it again. Only ever called because the user
/// pressed a button.
#[tauri::command(async)]
fn restart_manager() -> Result<(), String> {
    mallet_engine::restart_manager().map_err(|error| error.to_string())
}

/// Where the app writes its log, for showing it to the user.
#[tauri::command]
fn log_file() -> Option<String> {
    mallet_engine::default_log_dir()
        .map(|dir| dir.join(mallet_engine::LOG_FILE).display().to_string())
}

/// The games Mod Mallet knows, where each is, and which one the user
/// looks at.
#[tauri::command(async)]
fn games() -> Result<GameChoice, String> {
    let path = settings_path()?;
    mallet_engine::known_games(&path).map_err(|error| error.to_string())
}

/// Makes `game` the game to look at and returns the games afterwards. Each
/// game keeps its own places, findings set aside and ranking. Only ever
/// called because the user chose a game.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn select_game(game: String) -> Result<GameChoice, String> {
    let path = settings_path()?;
    mallet_engine::select_game(&path, &game).map_err(|error| error.to_string())?;
    mallet_engine::known_games(&path).map_err(|error| error.to_string())
}

fn settings_path() -> Result<PathBuf, String> {
    mallet_engine::default_settings_path()
        .ok_or_else(|| "the system does not say where settings are kept".to_owned())
}

/// Where the mod manager is, for the game the user looks at.
fn manager_options() -> ManagerOptions {
    settings().map_or_default(|settings| settings.manager_options())
}

fn settings() -> Result<Settings, String> {
    match mallet_engine::default_settings_path() {
        Some(path) => mallet_engine::load_settings(&path).map_err(|error| error.to_string()),
        None => Ok(Settings::default()),
    }
}

/// Changes the settings of the game the user looks at, as one step with
/// reading them, and returns them as written.
fn change_settings(change: impl FnOnce(&mut Settings)) -> Result<Settings, String> {
    let path = mallet_engine::default_settings_path()
        .ok_or_else(|| "the system does not say where settings are kept".to_owned())?;
    mallet_engine::update_settings(&path, change).map_err(|error| error.to_string())
}

fn render_places(settings: &Settings, lang: &str) -> RenderedPlaces {
    let planned = mallet_engine::planned_locations(&settings.scan_options());
    RenderedPlaces {
        managers: settings.managers(),
        ..messages(lang).render_places(settings.confirmed, &planned)
    }
}

fn messages(lang: &str) -> Messages {
    Messages::new(Locale::from_tag(lang).unwrap_or_default())
}

fn render(report: &ScanReport, lang: &str) -> RenderedReport {
    messages(lang).render_report(report)
}

/// Opens the window at Full HD, or smaller where the screen is: never
/// more than nine tenths of it, and in the middle. Not maximized.
fn fit_window(window: &tauri::WebviewWindow) {
    // Before the window is shown it may not know its screen yet.
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        tracing::info!("no screen known; the window keeps its size");
        return;
    };
    let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
    let size = tauri::LogicalSize::new(
        1920f64.min(screen.width * 0.9),
        1080f64.min(screen.height * 0.9),
    );
    tracing::info!(width = size.width, height = size.height, "window size");
    let _ = window.set_size(size);
    let _ = window.center();
}

/// Opens the app's window and runs until it is closed.
pub fn run() -> tauri::Result<()> {
    // Without a log the app still works; there is just nothing to read
    // afterwards.
    if let Some(dir) = mallet_engine::default_log_dir() {
        let _ = mallet_engine::start_logging(&dir);
    }
    tauri::Builder::default()
        // Only the open dialog is granted (capabilities/default.json): it
        // names a place to read, it reads and writes nothing itself.
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                fit_window(&window);
            }
            // So that the Vortex link can list this app among Vortex's tools.
            if let Err(error) = mallet_engine::announce_app(&manager_options()) {
                tracing::warn!(%error, "could not tell the Vortex link where Mod Mallet is");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan,
            last_report,
            run_action,
            manager_link,
            install_manager_link,
            set_vortex_tool,
            places,
            places_for_manager,
            confirm_places,
            ignored,
            set_ignored,
            setup_fingerprint,
            open_page,
            create_patch,
            create_moves,
            create_paths,
            save_patch_report,
            open_folder,
            show_data_file,
            log_file,
            toolbox,
            restart_manager,
            game_art,
            map_image,
            file_preview,
            compare_files,
            decisions,
            set_decision,
            mesh_view,
            mesh_thumbnails,
            catalogue,
            asset_versions,
            asset_choices,
            set_asset_choice,
            asset_view,
            lod_tiles,
            lod_mesh,
            lod_texture,
            area_objects,
            area_land,
            world_land,
            interior_objects,
            object_anchors,
            hidden_walkways,
            walkway_reach,
            door_reach,
            spots_reach,
            walkway_repair,
            walkway_repair_reach,
            area_paths,
            area_mesh,
            area_texture,
            ranking,
            set_ranking,
            patch_choices,
            set_patch_choices,
            patch_book,
            set_patch_book,
            preferences,
            set_preferences,
            load_patch_archive,
            games,
            select_game,
            set_last_health
        ])
        .run(tauri::generate_context!())
}
