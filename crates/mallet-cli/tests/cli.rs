//! Tests of the command line from the outside: arguments in, text out.

// Helper functions of an integration test are not inside a `#[test]`
// function, so the test exemptions in clippy.toml do not reach them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::path::Path;

const SKSE_LOG: &str = "\
SKSE64 runtime: initialize (version = 2.3.1 01070680 01DD748122C36157, os = 6.2 (9200))
plugin Fixes.dll (00000001 Fixes 00000007) loaded correctly (handle 1)
plugin helper.dll (00000000  00000000) no version data 0 (handle 0)
plugin Old.dll (00000001 old 00000001) disabled, incompatible with current version of the game 0 (handle 0)
";

/// Creates a made-up game directory below `root` and returns the arguments
/// that point a scan at it.
fn setup(root: &Path) -> Vec<String> {
    let game_dir = root.join("Game");
    let data_dir = game_dir.join("Data");
    fs::create_dir_all(&data_dir).unwrap();

    let log = root.join("skse64.log");
    fs::write(&log, SKSE_LOG).unwrap();
    // The plugins the log names are still in the game; a plugin that is
    // gone since is not reported.
    let plugins = data_dir.join("SKSE").join("Plugins");
    fs::create_dir_all(&plugins).unwrap();
    for plugin in ["Fixes.dll", "helper.dll", "Old.dll"] {
        fs::write(plugins.join(plugin), b"not a real library").unwrap();
    }

    // An empty list of active plugins, so that the test never looks at the
    // real one of the machine it runs on.
    let local_dir = root.join("Local");
    fs::create_dir_all(&local_dir).unwrap();
    fs::write(local_dir.join("plugins.txt"), "").unwrap();

    let target = data_dir.to_string_lossy().replace('\\', "\\\\");
    fs::write(
        data_dir.join("vortex.deployment.json"),
        format!(
            r#"{{ "targetPath": "{target}", "files": [
                {{ "relPath": "SKSE\\Plugins\\Old.dll", "source": "Old Mod 123 1.0" }}
            ] }}"#
        ),
    )
    .unwrap();

    vec![
        "--game-dir".to_owned(),
        game_dir.to_string_lossy().into_owned(),
        "--local-dir".to_owned(),
        local_dir.to_string_lossy().into_owned(),
        "--manager-state".to_owned(),
        root.join("no-vortex-state.json")
            .to_string_lossy()
            .into_owned(),
        "--extender-log".to_owned(),
        log.to_string_lossy().into_owned(),
        "--cache".to_owned(),
        root.join("cache.sqlite").to_string_lossy().into_owned(),
    ]
}

fn run(args: &[&str]) -> anyhow::Result<String> {
    let mut out = Vec::new();
    let all = std::iter::once("mallet").chain(args.iter().copied());
    mallet_cli::run(all, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

fn run_scan(leading: &[&str], setup_args: &[String], trailing: &[&str]) -> String {
    let mut args: Vec<&str> = leading.to_vec();
    args.push("scan");
    args.extend(setup_args.iter().map(String::as_str));
    args.extend_from_slice(trailing);
    run(&args).unwrap()
}

#[test]
fn scan_prints_a_report_in_english() {
    let root = tempfile::tempdir().unwrap();
    let output = run_scan(&[], &setup(root.path()), &[]);
    insta::assert_snapshot!(output);
}

#[test]
fn scan_prints_a_report_in_german() {
    let root = tempfile::tempdir().unwrap();
    let output = run_scan(&["--lang", "de"], &setup(root.path()), &[]);
    insta::assert_snapshot!(output);
}

#[test]
fn notes_are_listed_on_request() {
    let root = tempfile::tempdir().unwrap();
    let setup_args = setup(root.path());

    let without = run_scan(&[], &setup_args, &[]);
    assert!(!without.contains("helper.dll"));
    assert!(without.contains("Add --notes"));

    let with = run_scan(&[], &setup_args, &["--notes"]);
    assert!(with.contains("helper.dll"));
    assert!(!with.contains("Add --notes"));
}

#[test]
fn report_repeats_the_last_scan() {
    let root = tempfile::tempdir().unwrap();
    let setup_args = setup(root.path());
    let scanned = run_scan(&[], &setup_args, &[]);

    let cache = setup_args.last().unwrap();
    let reported = run(&["report", "--cache", cache]).unwrap();
    assert_eq!(reported, scanned);
}

#[test]
fn report_without_a_scan_says_so() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("none.sqlite");
    let output = run(&["report", "--cache", &cache.to_string_lossy()]).unwrap();
    assert_eq!(
        output,
        "No scan has been stored yet. Run \"mallet scan\" first.\n"
    );
}

#[test]
fn json_output_is_the_scan_report() {
    let root = tempfile::tempdir().unwrap();
    let output = run_scan(&[], &setup(root.path()), &["--format", "json"]);

    let report: mallet_core::ScanReport = serde_json::from_str(&output).unwrap();
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].kind, "extender.plugin-incompatible");
}

#[test]
fn explain_describes_a_kind() {
    let output = run(&["explain", "extender.plugin-incompatible"]).unwrap();
    insta::assert_snapshot!(output);
}

#[test]
fn explain_rejects_an_unknown_kind() {
    let error = run(&["explain", "no.such-kind"]).unwrap_err();
    assert!(error.to_string().contains("no.such-kind"));
}

#[test]
fn kinds_lists_every_kind() {
    let output = run(&["kinds"]).unwrap();
    for kind in mallet_core::kinds::ALL {
        assert!(
            output.contains(kind.id),
            "{} missing from:\n{output}",
            kind.id
        );
    }
}

/// The arguments that point the `vortex` commands at a made-up Vortex.
fn vortex_args(root: &Path) -> Vec<String> {
    fs::create_dir_all(root.join("Vortex").join("plugins")).unwrap();
    vec![
        "--vortex-data-dir".to_owned(),
        root.join("Vortex").to_string_lossy().into_owned(),
        "--link-dir".to_owned(),
        root.join("bridge").to_string_lossy().into_owned(),
    ]
}

fn run_vortex(command: &[&str], vortex_args: &[String]) -> anyhow::Result<String> {
    let mut args = vec!["vortex"];
    args.extend_from_slice(command);
    args.extend(vortex_args.iter().map(String::as_str));
    run(&args)
}

#[test]
fn the_vortex_extension_is_installed_on_request_only() {
    let root = tempfile::tempdir().unwrap();
    let args = vortex_args(root.path());
    let extension = root.path().join("Vortex/plugins/mod-mallet-link");

    let status = run_vortex(&["status"], &args).unwrap();
    assert!(status.starts_with("The Vortex extension is not installed."));
    assert!(!extension.exists());

    let installed = run_vortex(&["install-extension"], &args).unwrap();
    assert!(installed.contains("Restart Vortex"));
    assert!(extension.join("index.js").is_file());

    let status = run_vortex(&["status"], &args).unwrap();
    assert!(status.contains("Vortex is not answering"));
}

#[test]
fn a_reinstall_that_cannot_be_handed_over_is_an_error_that_says_why() {
    let root = tempfile::tempdir().unwrap();
    let args = vortex_args(root.path());

    let error = run_vortex(&["reinstall", "Patch Hub 1.0"], &args).unwrap_err();
    assert!(error.to_string().contains("not installed yet"));

    run_vortex(&["install-extension"], &args).unwrap();
    let error = run_vortex(&["reinstall", "Patch Hub 1.0"], &args).unwrap_err();
    assert!(error.to_string().contains("Vortex is not answering"));
}

#[test]
fn an_unsupported_language_is_rejected() {
    let error = run(&["--lang", "fr", "kinds"]).unwrap_err();
    assert!(error.to_string().contains("not a supported language"));
}

/// A made-up report of a made-up user, Alice, whose paths name her
/// everywhere.
fn report_of_alice() -> mallet_core::ScanReport {
    use mallet_core::{Finding, GamePath, LoadedPlugin, ModRef, Subject, WorldOrigin, kinds};
    let old_mod = ModRef {
        source: "vortex".to_owned(),
        id: "Old Mod-123-1-0".to_owned(),
        name: "Old Mod".to_owned(),
    };
    let mut report: mallet_core::ScanReport = serde_json::from_value(serde_json::json!({
        "schemaVersion": 1,
        "scannedAt": 0,
        "game": {
            "id": "skyrimse",
            "name": "Skyrim Special Edition",
            "version": "1.6.1170",
            "extenderVersion": "2.2.6"
        },
        "extenderPluginsChecked": 3,
        "pluginsChecked": 2,
        "archivesChecked": 1,
        "findings": [],
        "manager": "vortex",
    }))
    .unwrap();
    report.locations = alices_places();
    report.findings = vec![
        Finding::new(
            &kinds::EXTENDER_PLUGIN_INCOMPATIBLE,
            Subject::File {
                path: GamePath::new("data/skse/plugins/old.dll"),
            },
        )
        .with_arg("file", "Old.dll")
        .with_arg("gameVersion", "1.6.1170")
        .with_provider(Some(old_mod.clone())),
        Finding::new(&kinds::CRASHES_RECENT, Subject::Setup)
            .with_arg("count", 2_i64)
            .with_arg("days", 7_i64)
            .with_arg(
                "last",
                r"C:\Users\Alice\Documents\My Games\Skyrim Special Edition\SKSE\crash-2026-11-20.log",
            )
            .with_arg("examples", "SkyrimSE.exe+0x1234"),
    ];
    report.load_order = vec![
        LoadedPlugin {
            name: "Skyrim.esm".to_owned(),
            origin: WorldOrigin::Game,
            masters: Vec::new(),
            provider: None,
        },
        LoadedPlugin {
            name: "OldMod.esp".to_owned(),
            origin: WorldOrigin::Mod,
            masters: vec!["Skyrim.esm".to_owned()],
            provider: Some(old_mod),
        },
    ];
    report
}

/// The places a scan read on Alice's made-up machine.
fn alices_places() -> Vec<mallet_core::ScanLocation> {
    use mallet_core::{LocationOrigin, LocationRole, ScanLocation};
    let location = |role, path: &str, origin, found| ScanLocation {
        role,
        path: path.to_owned(),
        origin,
        found,
    };
    vec![
        location(
            LocationRole::GameDir,
            r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition",
            LocationOrigin::Registry,
            true,
        ),
        location(
            LocationRole::PluginList,
            r"C:\Users\Alice\AppData\Local\Skyrim Special Edition\plugins.txt",
            LocationOrigin::Standard,
            true,
        ),
        location(
            LocationRole::ExtenderLog,
            r"C:\Users\Alice\Documents\My Games\Skyrim Special Edition\SKSE\skse64.log",
            LocationOrigin::Standard,
            true,
        ),
        location(
            LocationRole::ManagerState,
            r"C:\Users\Alice\AppData\Roaming\Vortex\temp\state_backups_full\startup.json",
            LocationOrigin::Standard,
            true,
        ),
        location(
            LocationRole::ModStaging,
            r"E:\Alice\Vortex Mods\skyrimse",
            LocationOrigin::Manager,
            true,
        ),
        location(
            LocationRole::CrashLogs,
            r"C:\Users\Alice\Documents\My Games\Skyrim Special Edition\SKSE",
            LocationOrigin::Standard,
            false,
        ),
    ]
}

/// The private folders and names of Alice's made-up machine.
fn alices_machine(report: &mallet_core::ScanReport) -> mallet_core::Shortener {
    use mallet_core::{LocationRole, Placeholder};
    let mut shortener = mallet_core::Shortener::new();
    for location in &report.locations {
        match location.role {
            LocationRole::GameDir => {
                shortener = shortener.with_root(&location.path, Placeholder::Game);
            }
            LocationRole::ModStaging => {
                shortener = shortener.with_root(&location.path, Placeholder::Staging);
            }
            _ => {}
        }
    }
    shortener
        .with_root(r"C:\Users\Alice\Documents", Placeholder::Documents)
        .with_root(r"C:\Users\Alice\AppData\Local", Placeholder::LocalAppData)
        .with_root(r"C:\Users\Alice\AppData\Roaming", Placeholder::AppData)
        .with_root(r"C:\Users\Alice", Placeholder::UserProfile)
        .with_name("Alice", Placeholder::User)
        .with_name("ALICE-DESKTOP", Placeholder::Machine)
}

fn shared_text(lang: &str) -> String {
    let report = report_of_alice();
    let shared = mallet_engine::share_report_with(&report, &alices_machine(&report)).unwrap();
    let locale = mallet_messages::Locale::from_tag(lang).unwrap();
    mallet_messages::Messages::new(locale)
        .render_share(&shared)
        .text
}

#[test]
fn a_shared_report_in_english() {
    let text = shared_text("en");
    assert!(!text.contains("Alice"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn a_shared_report_in_german() {
    let text = shared_text("de");
    assert!(!text.contains("Alice"), "{text}");
    insta::assert_snapshot!(text);
}

#[test]
fn a_shared_report_as_json() {
    let report = report_of_alice();
    let shared = mallet_engine::share_report_with(&report, &alices_machine(&report)).unwrap();
    let json = serde_json::to_string_pretty(&shared.report).unwrap();
    assert!(!json.contains("Alice"), "{json}");
    insta::assert_snapshot!(json);
}

#[test]
fn report_share_leaves_no_folder_of_the_machine() {
    let root = tempfile::tempdir().unwrap();
    let setup_args = setup(root.path());
    run_scan(&[], &setup_args, &[]);
    let cache = setup_args.last().unwrap();
    let game_dir = root.path().join("Game").to_string_lossy().into_owned();

    for format in ["text", "json"] {
        let shared = run(&["report", "--share", "--format", format, "--cache", cache]).unwrap();
        assert!(shared.contains("<game>"), "{shared}");
        let flat = shared.replace("\\\\", "\\").to_lowercase();
        assert!(!flat.contains(&game_dir.to_lowercase()), "{shared}");
        if let Some(home) = dirs_home() {
            assert!(!flat.contains(&home.to_lowercase()), "{shared}");
        }
    }
}

/// The home directory, where the test can tell it: a path of more than one
/// folder below the root.
fn dirs_home() -> Option<String> {
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })?;
    let home = std::path::PathBuf::from(home);
    (home.components().count() > 2).then(|| home.to_string_lossy().into_owned())
}
