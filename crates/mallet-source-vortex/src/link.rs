//! The link to a running Vortex: asking it to do something.
//!
//! Mod Mallet does not edit Vortex's data (ADR 0005). To have Vortex act,
//! it writes a request into a folder that a small Vortex extension watches
//! (`integrations/vortex-extension`). The extension carries the request out
//! with the calls Vortex's own menus make and writes a reply.
//!
//! The folder holds three things:
//!
//! ```text
//! status.json            written by the extension every few seconds
//! requests/<id>.json     written here
//! results/<id>.json      written by the extension
//! ```
//!
//! A folder was chosen over a network port or a pipe because it needs no
//! permission, cannot be reached from outside the machine, and can be
//! inspected with a file manager when something does not work. The format
//! is versioned by [`PROTOCOL`].
//!
//! [`install_extension`] puts the extension into Vortex's plugin folder.
//! The extension's files are compiled into Mod Mallet, so the two always
//! speak the same protocol.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use mallet_core::OptionToTick;
use serde::{Deserialize, Serialize};

/// Version of the request and reply format.
pub const PROTOCOL: u32 = 1;

/// The folder of the extension inside Vortex's plugin folder.
pub const EXTENSION_FOLDER: &str = "mod-mallet-link";

const EXTENSION_INDEX: &str = include_str!("../../../integrations/vortex-extension/index.js");
const EXTENSION_INFO: &str = include_str!("../../../integrations/vortex-extension/info.json");

/// The extension counts as running if it reported in within this time. It
/// reports every five seconds.
const ALIVE_WITHIN: Duration = Duration::from_secs(15);

/// How often the reply is looked for while waiting.
const POLL_EVERY: Duration = Duration::from_millis(100);

/// Why Vortex could not be asked.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    /// The extension has not reported in recently: Vortex is closed, or the
    /// extension is not loaded.
    #[error("Vortex is not running, or the Mod Mallet link in it is not loaded")]
    NotRunning,
    /// No reply arrived in time.
    #[error("Vortex did not answer in time")]
    NoAnswer,
    /// A file of the shared folder could not be read or written.
    #[error("the folder shared with Vortex could not be used: {0}")]
    Io(#[from] std::io::Error),
    /// The reply was not understood.
    #[error("Vortex's answer could not be read: {0}")]
    InvalidReply(#[from] serde_json::Error),
}

/// What the extension said about a request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Reply {
    /// Whether the request was carried out.
    #[serde(default)]
    pub ok: bool,
    /// What happened, as one of the extension's fixed words: `started`,
    /// `unknown-mod`, `archive-missing`, `wrong-game`, `unsupported`,
    /// `expired` or `failed`.
    #[serde(default)]
    pub code: String,
    /// Further detail, if any.
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    protocol: u32,
    id: &'a str,
    action: &'a str,
    game: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    mod_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "<[TickRequest<'a>]>::is_empty")]
    tick: Vec<TickRequest<'a>>,
    #[serde(skip_serializing_if = "<[&str]>::is_empty")]
    files: &'a [&'a str],
    #[serde(skip_serializing_if = "<[&str]>::is_empty")]
    leave_out: &'a [&'a str],
    #[serde(skip_serializing_if = "<[&str]>::is_empty")]
    provide: &'a [&'a str],
    created_at: u64,
}

/// An option of a mod's installer that the extension ticks in advance
/// when it reinstalls the mod.
#[derive(Serialize)]
struct TickRequest<'a> {
    step: &'a str,
    group: &'a str,
    option: &'a str,
    index: u32,
}

/// What a request names besides its action and game.
#[derive(Default)]
struct Subject<'a> {
    mod_id: Option<&'a str>,
    path: Option<&'a str>,
    plugin: Option<&'a str>,
    after: Option<&'a str>,
    enabled: Option<bool>,
    tick: &'a [OptionToTick],
    files: &'a [&'a str],
    leave_out: &'a [&'a str],
    provide: &'a [&'a str],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    #[serde(default)]
    extension_version: String,
    #[serde(default)]
    heartbeat_at: u64,
}

/// The folder shared with the Vortex extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    dir: PathBuf,
}

impl Link {
    /// Uses `dir` as the shared folder. The extension looks in
    /// `%LOCALAPPDATA%\mod-mallet\vortex-bridge`.
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The version of the extension if it is running in Vortex right now.
    pub fn running_extension(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.dir.join("status.json")).ok()?;
        let status: Status = serde_json::from_str(&text).ok()?;
        let age = now_ms().saturating_sub(status.heartbeat_at);
        (u128::from(age) <= ALIVE_WITHIN.as_millis()).then_some(status.extension_version)
    }

    /// Asks Vortex to reinstall the mod `mod_id` of `game`, and waits up to
    /// `timeout` for the reply.
    ///
    /// `mod_id` is the mod's folder in Vortex's staging directory. The
    /// installer options in `tick` are ticked in advance, on top of the
    /// choices made last time. A reply of `started` means Vortex has opened
    /// the mod's installer; what the user does there is not waited for.
    pub fn reinstall_mod(
        &self,
        game: &str,
        mod_id: &str,
        tick: &[OptionToTick],
        timeout: Duration,
    ) -> Result<Reply, LinkError> {
        let subject = Subject {
            mod_id: Some(mod_id),
            tick,
            ..Subject::default()
        };
        self.ask("reinstall-mod", game, &subject, timeout)
    }

    /// The folder in which Mod Mallet puts the archives it asks Vortex to
    /// install. The extension installs nothing from anywhere else.
    pub fn patches_dir(&self) -> PathBuf {
        self.dir
            .parent()
            .map_or_else(|| self.dir.join("patches"), |parent| parent.join("patches"))
    }

    /// Asks Vortex to install the archive at `path` as a mod, as if it had
    /// been dropped onto Vortex, and waits up to `timeout` for the reply.
    /// The archive has to lie in [`Link::patches_dir`].
    pub fn install_archive(
        &self,
        game: &str,
        path: &Path,
        timeout: Duration,
    ) -> Result<Reply, LinkError> {
        let path = path.to_string_lossy();
        let subject = Subject {
            path: Some(&path),
            ..Subject::default()
        };
        self.ask("install-archive", game, &subject, timeout)
    }

    /// Asks Vortex to load `plugin` after `after`, through a rule in its
    /// plugin list, and to sort the plugins again. The rule stays visible
    /// and removable in Vortex.
    pub fn load_after(
        &self,
        game: &str,
        plugin: &str,
        after: &str,
        timeout: Duration,
    ) -> Result<Reply, LinkError> {
        let subject = Subject {
            plugin: Some(plugin),
            after: Some(after),
            ..Subject::default()
        };
        self.ask("load-after", game, &subject, timeout)
    }

    /// Asks Vortex to switch the mod `mod_id` on or off in the profile in
    /// use.
    pub fn set_mod_enabled(
        &self,
        game: &str,
        mod_id: &str,
        enabled: bool,
        timeout: Duration,
    ) -> Result<Reply, LinkError> {
        let subject = Subject {
            mod_id: Some(mod_id),
            enabled: Some(enabled),
            ..Subject::default()
        };
        self.ask("set-mod-enabled", game, &subject, timeout)
    }

    /// Asks Vortex to leave `files` (below the data directory) out of the
    /// mods `leave_out` and to deploy them again from the mods `provide`,
    /// through its file overrides, and then to deploy. Mods are named by
    /// their folder in the staging directory.
    pub fn file_overrides(
        &self,
        game: &str,
        files: &[&str],
        leave_out: &[&str],
        provide: &[&str],
        timeout: Duration,
    ) -> Result<Reply, LinkError> {
        let subject = Subject {
            files,
            leave_out,
            provide,
            ..Subject::default()
        };
        self.ask("file-overrides", game, &subject, timeout)
    }

    fn ask(
        &self,
        action: &str,
        game: &str,
        subject: &Subject<'_>,
        timeout: Duration,
    ) -> Result<Reply, LinkError> {
        if self.running_extension().is_none() {
            return Err(LinkError::NotRunning);
        }

        let id = request_id();
        let file_name = format!("{id}.json");
        let request_file = self.dir.join("requests").join(&file_name);
        let reply_file = self.dir.join("results").join(&file_name);
        let request = Request {
            protocol: PROTOCOL,
            id: &id,
            action,
            game,
            mod_id: subject.mod_id,
            path: subject.path,
            plugin: subject.plugin,
            after: subject.after,
            enabled: subject.enabled,
            tick: subject
                .tick
                .iter()
                .map(|option| TickRequest {
                    step: &option.step,
                    group: &option.group,
                    option: &option.option,
                    index: option.index,
                })
                .collect(),
            files: subject.files,
            leave_out: subject.leave_out,
            provide: subject.provide,
            created_at: now_ms(),
        };

        // Written under another name first, so that the extension never
        // reads half a request.
        let temporary = request_file.with_extension("tmp");
        std::fs::write(&temporary, serde_json::to_vec(&request)?)?;
        std::fs::rename(&temporary, &request_file)?;

        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(text) = std::fs::read_to_string(&reply_file) {
                // The reply has been read; it is of no further use.
                let _ = std::fs::remove_file(&reply_file);
                return Ok(serde_json::from_str(&text)?);
            }
            if Instant::now() >= deadline {
                // Withdrawn, so that it is not carried out later when
                // nobody expects it any more.
                let _ = std::fs::remove_file(&request_file);
                return Err(LinkError::NoAnswer);
            }
            std::thread::sleep(POLL_EVERY);
        }
    }
}

/// The folder of the extension, given the directory in which Vortex keeps
/// its data (`%APPDATA%\Vortex` on Windows).
pub fn extension_dir(vortex_data_dir: &Path) -> PathBuf {
    vortex_data_dir.join("plugins").join(EXTENSION_FOLDER)
}

/// The version of the extension this Mod Mallet contains.
pub fn extension_version() -> Option<String> {
    #[derive(Deserialize)]
    struct Info {
        version: String,
    }
    serde_json::from_str::<Info>(EXTENSION_INFO)
        .ok()
        .map(|info| info.version)
}

/// Whether the extension is installed in Vortex in exactly the version
/// this Mod Mallet contains.
pub fn extension_is_current(vortex_data_dir: &Path) -> bool {
    let dir = extension_dir(vortex_data_dir);
    let same = |file: &str, expected: &str| {
        std::fs::read_to_string(dir.join(file)).is_ok_and(|found| found == expected)
    };
    same("index.js", EXTENSION_INDEX) && same("info.json", EXTENSION_INFO)
}

/// Puts the extension into Vortex's plugin folder, replacing an older
/// version. Vortex loads it the next time it starts.
///
/// This is the one place where Mod Mallet writes into Vortex's data
/// directory, and it only ever writes its own two files into its own
/// folder there. It must be started by the user.
pub fn install_extension(vortex_data_dir: &Path) -> std::io::Result<PathBuf> {
    let dir = extension_dir(vortex_data_dir);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("index.js"), EXTENSION_INDEX)?;
    std::fs::write(dir.join("info.json"), EXTENSION_INFO)?;
    Ok(dir)
}

/// A name for a request that no other request has: the time and the
/// process. Only hexadecimal digits and `-`, which is all the extension
/// accepts.
fn request_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("{nanos:x}-{:x}", std::process::id())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A shared folder with the directories the extension creates.
    fn shared_folder() -> (tempfile::TempDir, Link) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("requests")).unwrap();
        fs::create_dir_all(dir.path().join("results")).unwrap();
        let link = Link::new(dir.path().to_path_buf());
        (dir, link)
    }

    fn write_status(dir: &Path, heartbeat_at: u64) {
        fs::write(
            dir.join("status.json"),
            format!(
                r#"{{"protocol":1,"extensionVersion":"0.1.0","startedAt":1,"heartbeatAt":{heartbeat_at}}}"#
            ),
        )
        .unwrap();
    }

    /// Plays the extension: answers the first request that appears with
    /// `reply` and returns the request it saw.
    fn answer_next_request(dir: PathBuf, reply: &'static str) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            loop {
                let request = fs::read_dir(dir.join("requests"))
                    .unwrap()
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .find(|path| path.extension().is_some_and(|ext| ext == "json"));
                if let Some(request) = request {
                    let text = fs::read_to_string(&request).unwrap();
                    fs::remove_file(&request).unwrap();
                    let name = request.file_name().unwrap();
                    fs::write(dir.join("results").join(name), reply).unwrap();
                    return text;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })
    }

    #[test]
    fn the_extension_counts_as_running_while_it_reports_in() {
        let (dir, link) = shared_folder();
        assert_eq!(link.running_extension(), None);

        write_status(dir.path(), now_ms());
        assert_eq!(link.running_extension().as_deref(), Some("0.1.0"));

        // A status from a minute ago is from a Vortex that has been closed.
        write_status(dir.path(), now_ms() - 60_000);
        assert_eq!(link.running_extension(), None);

        fs::write(dir.path().join("status.json"), "not json").unwrap();
        assert_eq!(link.running_extension(), None);
    }

    #[test]
    fn a_request_is_written_and_the_reply_returned() {
        let (dir, link) = shared_folder();
        write_status(dir.path(), now_ms());
        let extension = answer_next_request(
            dir.path().to_path_buf(),
            r#"{"id":"x","ok":true,"code":"started"}"#,
        );

        let tick = [OptionToTick {
            page: 2,
            pages: 3,
            step: "Patches".to_owned(),
            group: "Cities".to_owned(),
            option: "Vigil".to_owned(),
            index: 4,
            description: "Words of the author".to_owned(),
        }];
        let reply = link
            .reinstall_mod("skyrimse", "Patch Hub 1.0", &tick, Duration::from_secs(5))
            .unwrap();
        assert_eq!(
            reply,
            Reply {
                ok: true,
                code: "started".to_owned(),
                detail: None
            }
        );

        let request: serde_json::Value = serde_json::from_str(&extension.join().unwrap()).unwrap();
        assert_eq!(request["protocol"], 1);
        assert_eq!(request["action"], "reinstall-mod");
        assert_eq!(request["game"], "skyrimse");
        assert_eq!(request["modId"], "Patch Hub 1.0");
        assert_eq!(
            request["tick"],
            serde_json::json!([{"step": "Patches", "group": "Cities", "option": "Vigil", "index": 4}])
        );
        assert!(request["createdAt"].as_u64().unwrap() > 0);
        let id = request["id"].as_str().unwrap();
        assert!(id.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));

        // Nothing is left behind.
        assert_eq!(
            fs::read_dir(dir.path().join("requests")).unwrap().count(),
            0
        );
        assert_eq!(fs::read_dir(dir.path().join("results")).unwrap().count(), 0);
    }

    #[test]
    fn a_refusal_is_passed_on() {
        let (dir, link) = shared_folder();
        write_status(dir.path(), now_ms());
        let extension = answer_next_request(
            dir.path().to_path_buf(),
            r#"{"id":"x","ok":false,"code":"unknown-mod"}"#,
        );
        let reply = link
            .reinstall_mod("skyrimse", "Gone", &[], Duration::from_secs(5))
            .unwrap();
        extension.join().unwrap();
        assert!(!reply.ok);
        assert_eq!(reply.code, "unknown-mod");
    }

    #[test]
    fn nothing_is_asked_of_a_vortex_that_is_not_running() {
        let (dir, link) = shared_folder();
        let result = link.reinstall_mod("skyrimse", "Patch Hub 1.0", &[], Duration::from_secs(5));
        assert!(matches!(result, Err(LinkError::NotRunning)));
        assert_eq!(
            fs::read_dir(dir.path().join("requests")).unwrap().count(),
            0
        );
    }

    #[test]
    fn a_request_nobody_answers_is_withdrawn() {
        let (dir, link) = shared_folder();
        write_status(dir.path(), now_ms());
        let result =
            link.reinstall_mod("skyrimse", "Patch Hub 1.0", &[], Duration::from_millis(300));
        assert!(matches!(result, Err(LinkError::NoAnswer)));
        assert_eq!(
            fs::read_dir(dir.path().join("requests")).unwrap().count(),
            0
        );
    }

    #[test]
    fn the_extension_is_installed_into_its_own_folder_only() {
        let vortex = tempfile::tempdir().unwrap();
        fs::create_dir_all(vortex.path().join("plugins").join("Another Extension")).unwrap();
        assert!(!extension_is_current(vortex.path()));

        let dir = install_extension(vortex.path()).unwrap();
        assert_eq!(dir, vortex.path().join("plugins").join("mod-mallet-link"));
        assert!(extension_is_current(vortex.path()));

        let mut files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        files.sort();
        assert_eq!(files, ["index.js", "info.json"]);
        // Other extensions are left alone.
        assert_eq!(
            fs::read_dir(vortex.path().join("plugins")).unwrap().count(),
            2
        );

        // An extension that was changed, or is older, is not current.
        fs::write(dir.join("index.js"), "// older").unwrap();
        assert!(!extension_is_current(vortex.path()));
        install_extension(vortex.path()).unwrap();
        assert!(extension_is_current(vortex.path()));
    }

    #[test]
    fn the_protocol_number_matches_the_extension() {
        assert!(EXTENSION_INDEX.contains(&format!("const PROTOCOL = {PROTOCOL};")));
    }
}
