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
use tauri::Manager;

/// Examines the setup and returns what is wrong with it, in the language
/// `lang`. The result is remembered for [`last_report`].
// `async` here means "not on the main thread": a scan must not freeze the
// window. Tauri hands command arguments over by value.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn scan(lang: String) -> Result<RenderedReport, String> {
    let options = settings()?.scan_options();
    let report = match mallet_engine::default_cache_path() {
        Some(cache) => mallet_engine::scan_and_store(&options, &cache),
        None => mallet_engine::scan(&options),
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

/// The places a scan would read and how each was found, in the language
/// `lang`, and whether the user has agreed to them. Nothing is read to
/// find this out.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn places(lang: String) -> Result<RenderedPlaces, String> {
    Ok(render_places(&settings()?, &lang))
}

/// Remembers the places the user named and that they agreed to the rest.
/// `named` holds only what the user chose; every other place is found by
/// its rule. Only ever called because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn confirm_places(
    named: BTreeMap<LocationRole, String>,
    lang: String,
) -> Result<RenderedPlaces, String> {
    // What else the user decided stays as it was.
    let settings = change_settings(|settings| {
        settings.confirmed = true;
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

/// The sides the user picked in duels between mods, by duel.
#[tauri::command(async)]
fn decisions() -> Result<BTreeMap<String, u8>, String> {
    Ok(settings()?.decisions)
}

/// Remembers the side picked in a duel, or forgets it with `None`, and
/// returns all decisions afterwards. Only ever called because the user
/// pressed a button.
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

/// Writes the patch that hides `objects` and returns where it is. Nothing
/// is installed. Only ever called because the user pressed a button.
#[tauri::command(async)]
#[allow(clippy::needless_pass_by_value)]
fn create_patch(objects: Vec<ObjectRef>) -> Result<String, String> {
    mallet_engine::create_patch(&objects, &manager_options())
        .map(|created| created.path.display().to_string())
        .map_err(|error| {
            tracing::warn!(%error, "patch not written");
            error.to_string()
        })
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
    messages(lang).render_places(settings.confirmed, &planned)
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
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                fit_window(&window);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan,
            last_report,
            run_action,
            manager_link,
            install_manager_link,
            places,
            confirm_places,
            ignored,
            set_ignored,
            setup_fingerprint,
            open_page,
            create_patch,
            open_folder,
            log_file,
            toolbox,
            restart_manager,
            game_art,
            map_image,
            file_preview,
            decisions,
            set_decision,
            mesh_view,
            lod_tiles,
            lod_mesh,
            lod_texture,
            area_objects,
            area_land,
            area_mesh,
            area_texture,
            ranking,
            set_ranking,
            patch_choices,
            set_patch_choices,
            games,
            select_game
        ])
        .run(tauri::generate_context!())
}
