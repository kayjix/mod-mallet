//! A made-up Skyrim updated by a made-up Steam, scanned before and after
//! (ADR 0079). Every file is synthetic (ADR 0017): the game's program, the
//! script extender and its plugins are tiny DLLs with only the version
//! information and the exports the scan reads.

// Helper functions of an integration test are not inside a `#[test]`
// function, so the test exemptions in clippy.toml do not reach them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::fs;
use std::path::PathBuf;

use mallet_core::{ArgValue, ScanReport, VersionStep};
use mallet_engine::{ScanOptions, scan_and_store};
use mallet_formats_bethesda::fixture;
use mallet_game_bethesda::pe::fixture::{Export, dll};
use mallet_game_bethesda::versions::fixture::version_data;

const V1_6_1170: u32 = 0x0106_4920;
const V1_7_104: u32 = 0x0107_0680;
const MASTER_FLAG: u32 = 1;

/// A game folder with SKSE for 1.6.1170 and 1.7.104, the Address Library
/// for both, a plugin that finds the game through it, and the library
/// of the body and face editor (`skee64.dll`) built for 1.6.1170 only.
struct Setup {
    root: tempfile::TempDir,
}

impl Setup {
    fn new() -> Self {
        let setup = Self {
            root: tempfile::tempdir().unwrap(),
        };
        let plugins = setup.plugins_dir();
        fs::create_dir_all(&plugins).unwrap();
        fs::create_dir_all(setup.root.path().join("Local")).unwrap();
        fs::write(setup.root.path().join("Local/plugins.txt"), "").unwrap();
        fs::write(
            setup.game_dir().join("Data/Skyrim.esm"),
            fixture::plugin(&[], MASTER_FLAG, &[]),
        )
        .unwrap();
        fs::write(
            setup.game_dir().join("skse64_loader.exe"),
            dll(&[], Some([0, 2, 3, 1]), "SKSE"),
        )
        .unwrap();
        for library in ["skse64_1_6_1170.dll", "skse64_1_7_104.dll"] {
            fs::write(setup.game_dir().join(library), b"").unwrap();
        }
        for database in ["versionlib-1-6-1170-0.bin", "versionlib-1-7-104-0.bin"] {
            fs::write(plugins.join(database), b"").unwrap();
        }
        setup.write_plugin(
            "Fixes.dll",
            &version_data("Fixes", 0b101, 0, &[], 0),
            [1, 0, 0, 0],
        );
        setup.write_plugin(
            "skee64.dll",
            &version_data("skee", 0, 0, &[V1_6_1170], 0),
            [0, 4, 19, 16],
        );
        setup.set_game([1, 6, 1170, 0]);
        setup
    }

    fn game_dir(&self) -> PathBuf {
        self.root.path().join("Game")
    }

    fn plugins_dir(&self) -> PathBuf {
        self.game_dir().join("Data/SKSE/Plugins")
    }

    fn cache_path(&self) -> PathBuf {
        self.root.path().join("cache/cache.sqlite")
    }

    /// What Steam does: puts a program of another version in place.
    fn set_game(&self, version: [u16; 4]) {
        fs::write(
            self.game_dir().join("SkyrimSE.exe"),
            dll(&[], Some(version), "Skyrim"),
        )
        .unwrap();
    }

    fn write_plugin(&self, name: &str, data: &[u8], version: [u16; 4]) {
        fs::write(
            self.plugins_dir().join(name),
            dll(
                &[Export {
                    name: "SKSEPlugin_Version",
                    data: data.to_vec(),
                }],
                Some(version),
                name,
            ),
        )
        .unwrap();
    }

    fn options(&self) -> ScanOptions {
        ScanOptions {
            game_dir: Some(self.game_dir()),
            local_dir: Some(self.root.path().join("Local")),
            // Tests never look at the real logs or Vortex of the machine.
            extender_log: Some(self.root.path().join("skse64.log")),
            manager_state: Some(self.root.path().join("vortex-state.json")),
            downloads_dir: Some(self.root.path().join("downloads")),
            staging_dir: Some(self.root.path().join("staging")),
            ..ScanOptions::default()
        }
    }

    fn scan(&self) -> ScanReport {
        scan_and_store(&self.options(), &self.cache_path()).unwrap()
    }
}

fn version_kinds(report: &ScanReport) -> Vec<&str> {
    report
        .findings
        .iter()
        .map(|finding| finding.kind.as_str())
        .filter(|kind| kind.starts_with("versions.") || kind.starts_with("extender."))
        .collect()
}

#[test]
fn a_steam_update_is_noticed_before_the_game_is_started_again() {
    let setup = Setup::new();

    let before = setup.scan();
    assert_eq!(before.game.version.as_deref(), Some("1.6.1170"));
    assert_eq!(before.game.extender_version.as_deref(), Some("2.3.1"));
    assert_eq!(version_kinds(&before), Vec::<&str>::new());
    assert_eq!(before.version_changes, None);
    assert_eq!(before.extender_plugins_checked, 2);

    setup.set_game([1, 7, 104, 0]);
    let after = setup.scan();
    assert_eq!(version_kinds(&after), ["versions.plugin-wrong-game"]);
    let finding = &after.findings[0];
    assert_eq!(finding.args["file"], ArgValue::from("skee64.dll"));
    assert_eq!(finding.args["builtFor"], ArgValue::from("1.6.1170"));
    assert_eq!(finding.args["confirmed"], ArgValue::from("no"));
    let changes = after.version_changes.as_ref().unwrap();
    assert_eq!(changes.since, before.scanned_at);
    assert_eq!(
        changes.game,
        Some(VersionStep {
            from: "1.6.1170".to_owned(),
            to: "1.7.104".to_owned()
        })
    );
    assert_eq!(changes.unfit, ["skee64.dll"]);

    // Scanning again keeps the change while the plugin does not fit.
    let again = setup.scan();
    assert_eq!(again.version_changes.as_ref().unwrap().game, changes.game);

    // RaceMenu's update arrives, built for 1.7.104.
    setup.write_plugin(
        "skee64.dll",
        &version_data("skee", 0, 0, &[V1_7_104], 0),
        [0, 4, 20, 0],
    );
    let updated = setup.scan();
    assert_eq!(version_kinds(&updated), Vec::<&str>::new());
    let changes = updated.version_changes.unwrap();
    assert_eq!(changes.game.unwrap().to, "1.7.104");
    assert_eq!(changes.unfit, Vec::<String>::new());
    assert_eq!(changes.plugins[0].file, "skee64.dll");
    assert_eq!(changes.plugins[0].from, "0.4.19.16");
    assert_eq!(changes.plugins[0].to, "0.4.20.0");
}

#[test]
fn a_script_extender_behind_the_game_is_one_finding() {
    let setup = Setup::new();
    fs::remove_file(setup.game_dir().join("skse64_1_7_104.dll")).unwrap();
    setup.set_game([1, 7, 104, 0]);
    let report = setup.scan();
    let kinds = version_kinds(&report);
    assert!(kinds.contains(&"versions.extender-lacks-game"), "{kinds:?}");
    assert!(
        report.version_changes.is_none(),
        "nothing to compare with yet"
    );
}

#[test]
fn scanning_the_versions_writes_nothing_into_the_game() {
    let setup = Setup::new();
    let listing = || {
        let mut files: Vec<_> = fs::read_dir(setup.plugins_dir())
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name(),
                    entry.metadata().unwrap().modified().unwrap(),
                )
            })
            .collect();
        files.sort();
        files
    };
    let before = listing();
    setup.scan();
    assert_eq!(listing(), before);
}
