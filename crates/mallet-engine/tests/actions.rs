//! Tests of handing an action to the mod manager, with a thread that plays
//! the Vortex extension.

// Helper functions of an integration test are not inside a `#[test]`
// function, so the test exemptions in clippy.toml do not reach them.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use mallet_core::{Action, ActionStatus, ModRef};
use mallet_engine::{
    EngineError, ManagerLink, ManagerOptions, install_manager_link, manager_link, run_action,
};

/// A made-up Vortex: its data directory and the folder shared with its
/// extension.
struct Vortex {
    root: tempfile::TempDir,
}

impl Vortex {
    fn new() -> Self {
        let vortex = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(vortex.data_dir().join("plugins")).unwrap();
        vortex
    }

    fn data_dir(&self) -> PathBuf {
        self.root.path().join("Vortex")
    }

    fn link_dir(&self) -> PathBuf {
        self.root.path().join("bridge")
    }

    fn options(&self) -> ManagerOptions {
        ManagerOptions {
            game: None,
            vortex_data_dir: Some(self.data_dir()),
            link_dir: Some(self.link_dir()),
            timeout: Some(Duration::from_secs(5)),
        }
    }

    /// Does what the extension does when Vortex starts.
    fn start_extension(&self) {
        let version = mallet_source_vortex::link::extension_version().unwrap();
        self.start_extension_of_version(&version);
    }

    fn start_extension_of_version(&self, version: &str) {
        fs::create_dir_all(self.link_dir().join("requests")).unwrap();
        fs::create_dir_all(self.link_dir().join("results")).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        fs::write(
            self.link_dir().join("status.json"),
            format!(r#"{{"protocol":1,"extensionVersion":"{version}","heartbeatAt":{now}}}"#),
        )
        .unwrap();
    }

    /// Answers the next request with `reply`, as the extension would, and
    /// returns the mod that was asked for.
    fn answer_next_request(&self, reply: &'static str) -> std::thread::JoinHandle<String> {
        let dir = self.link_dir();
        std::thread::spawn(move || {
            loop {
                if let Some(request) = first_json(&dir.join("requests")) {
                    let text = fs::read_to_string(&request).unwrap();
                    fs::remove_file(&request).unwrap();
                    fs::write(
                        dir.join("results").join(request.file_name().unwrap()),
                        reply,
                    )
                    .unwrap();
                    let request: serde_json::Value = serde_json::from_str(&text).unwrap();
                    // What was asked for: a mod, or the archive to install.
                    let asked = request["modId"].as_str().or(request["path"].as_str());
                    return asked.unwrap().to_owned();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
    }
}

fn first_json(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
}

fn reinstall(source: &str) -> Action {
    Action::ReinstallMod {
        mod_ref: ModRef {
            source: source.to_owned(),
            id: "Patch Hub 1.0".to_owned(),
            name: "Patch Hub".to_owned(),
        },
        tick: Vec::new(),
    }
}

#[test]
fn without_the_link_nothing_is_asked_of_vortex() {
    let vortex = Vortex::new();
    assert_eq!(
        manager_link(&vortex.options()),
        ManagerLink {
            installed: false,
            running: false,
            restart_needed: false,
        }
    );
    let outcome = run_action(&reinstall("vortex"), &vortex.options());
    assert_eq!(outcome.status, ActionStatus::LinkMissing);
}

#[test]
fn installing_the_link_puts_it_into_vortex_plugin_folder() {
    let vortex = Vortex::new();
    let installed_to = install_manager_link(&vortex.options()).unwrap();
    assert_eq!(
        installed_to,
        vortex.data_dir().join("plugins").join("mod-mallet-link")
    );
    assert!(installed_to.join("index.js").is_file());
    assert!(installed_to.join("info.json").is_file());

    // Installed, but Vortex has not been restarted to load it.
    assert_eq!(
        manager_link(&vortex.options()),
        ManagerLink {
            installed: true,
            running: false,
            restart_needed: false,
        }
    );
    let outcome = run_action(&reinstall("vortex"), &vortex.options());
    assert_eq!(outcome.status, ActionStatus::ManagerNotRunning);
}

#[test]
fn the_link_is_not_installed_where_there_is_no_vortex() {
    let vortex = Vortex::new();
    let options = ManagerOptions {
        vortex_data_dir: Some(vortex.root.path().join("No Vortex Here")),
        ..vortex.options()
    };
    assert!(matches!(
        install_manager_link(&options),
        Err(EngineError::ManagerNotFound)
    ));
    assert!(!vortex.root.path().join("No Vortex Here").exists());
}

#[test]
fn a_reinstall_is_handed_to_the_running_vortex() {
    let vortex = Vortex::new();
    install_manager_link(&vortex.options()).unwrap();
    vortex.start_extension();
    assert!(manager_link(&vortex.options()).running);

    let extension = vortex.answer_next_request(r#"{"ok":true,"code":"started"}"#);
    let outcome = run_action(&reinstall("vortex"), &vortex.options());
    assert_eq!(outcome.status, ActionStatus::Started);
    assert_eq!(extension.join().unwrap(), "Patch Hub 1.0");
}

#[test]
fn a_vortex_that_still_runs_an_older_link_has_to_be_restarted() {
    let vortex = Vortex::new();
    install_manager_link(&vortex.options()).unwrap();
    vortex.start_extension_of_version("0.0.1");
    assert_eq!(
        manager_link(&vortex.options()),
        ManagerLink {
            installed: true,
            running: true,
            restart_needed: true,
        }
    );

    vortex.start_extension();
    assert!(!manager_link(&vortex.options()).restart_needed);
}

#[test]
fn hiding_objects_writes_a_patch_and_hands_it_to_vortex() {
    let vortex = Vortex::new();
    install_manager_link(&vortex.options()).unwrap();
    vortex.start_extension();

    let extension = vortex.answer_next_request(r#"{"ok":true,"code":"installing"}"#);
    let patch = mallet_core::Patch {
        name: mallet_core::DEFAULT_PATCH_NAME.to_owned(),
        author: None,
        version: None,
        parts: vec![mallet_core::PatchPart {
            plugin: "Village.esp".to_owned(),
            against: Some("Town.esp".to_owned()),
            objects: vec![mallet_core::ObjectRef {
                plugin: "Village.esp".to_owned(),
                local_id: 0x812,
            }],
        }],
    };
    let outcome = run_action(&Action::HideObjects { patch }, &vortex.options());

    assert_eq!(outcome.status, ActionStatus::Installing);
    let archive = PathBuf::from(extension.join().unwrap());
    // The patch lies next to the shared folder, where the extension looks.
    assert_eq!(
        archive.parent(),
        Some(vortex.root.path().join("patches").as_path())
    );
    let bytes = fs::read(&archive).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert_eq!(
        archive.file_name().and_then(|name| name.to_str()),
        Some("Mod Mallet Patch.zip")
    );
    assert!(text.contains("Mod Mallet Patch - Village - Town_SWAP.ini"));
    assert!(text.contains("fomod/ModuleConfig.xml"));
    assert!(text.contains("0x812~Village.esp|0x3B~Skyrim.esm"));
}

#[test]
fn nothing_to_hide_asks_nothing_of_vortex() {
    let vortex = Vortex::new();
    let outcome = run_action(
        &Action::HideObjects {
            patch: mallet_core::Patch {
                name: mallet_core::DEFAULT_PATCH_NAME.to_owned(),
                author: None,
                version: None,
                parts: Vec::new(),
            },
        },
        &vortex.options(),
    );
    assert_eq!(outcome.status, ActionStatus::Failed);
    assert!(!vortex.root.path().join("patches").exists());
}

#[test]
fn what_vortex_refuses_is_passed_on() {
    let vortex = Vortex::new();
    install_manager_link(&vortex.options()).unwrap();
    vortex.start_extension();

    for (reply, expected) in [
        (
            r#"{"ok":false,"code":"unknown-mod"}"#,
            ActionStatus::UnknownMod,
        ),
        (
            r#"{"ok":false,"code":"archive-missing"}"#,
            ActionStatus::ArchiveMissing,
        ),
        (
            r#"{"ok":false,"code":"wrong-game"}"#,
            ActionStatus::WrongGame,
        ),
        (
            r#"{"ok":false,"code":"unsupported"}"#,
            ActionStatus::Unsupported,
        ),
        (
            r#"{"ok":false,"code":"failed","detail":"disk full"}"#,
            ActionStatus::Failed,
        ),
        (
            r#"{"ok":false,"code":"something new"}"#,
            ActionStatus::Failed,
        ),
    ] {
        let extension = vortex.answer_next_request(reply);
        let outcome = run_action(&reinstall("vortex"), &vortex.options());
        extension.join().unwrap();
        assert_eq!(outcome.status, expected, "{reply}");
    }
}

#[test]
fn a_vortex_that_does_not_answer_is_reported_as_such() {
    let vortex = Vortex::new();
    install_manager_link(&vortex.options()).unwrap();
    vortex.start_extension();
    let options = ManagerOptions {
        timeout: Some(Duration::from_millis(300)),
        ..vortex.options()
    };
    let outcome = run_action(&reinstall("vortex"), &options);
    assert_eq!(outcome.status, ActionStatus::NoAnswer);
    // The request was withdrawn.
    assert_eq!(first_json(&vortex.link_dir().join("requests")), None);
}

#[test]
fn a_mod_of_another_manager_cannot_be_reinstalled_through_vortex() {
    let vortex = Vortex::new();
    let outcome = run_action(&reinstall("mo2"), &vortex.options());
    assert_eq!(outcome.status, ActionStatus::Unsupported);
}
