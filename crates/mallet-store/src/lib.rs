//! Mod Mallet's local cache.
//!
//! The store is a SQLite file that remembers the result of the last scan, so
//! it can be shown again without scanning. It is a cache and nothing more
//! (ADR 0006): it holds nothing that a new scan cannot recreate. That is why
//! there are no migrations. When the layout changes, [`SCHEMA_VERSION`] is
//! raised and an older file is simply emptied on the next open.
//!
//! Decisions people make, such as ignoring a finding, must never be stored
//! here. They belong in a separate, versioned file.

use std::path::Path;

use mallet_core::{Finding, ScanReport, TextCache};
use rusqlite::{Connection, OptionalExtension, params};

/// Version of the table layout. Raise it with every change to [`SCHEMA`].
pub const SCHEMA_VERSION: u32 = 3;

const SCHEMA: &str = "
    CREATE TABLE scan (
        id          INTEGER PRIMARY KEY CHECK (id = 1),
        -- The report without its findings, as JSON.
        report_json TEXT NOT NULL
    );
    CREATE TABLE finding (
        position INTEGER PRIMARY KEY,
        id       TEXT NOT NULL UNIQUE,
        kind     TEXT NOT NULL,
        json     TEXT NOT NULL
    );
    CREATE INDEX finding_kind ON finding (kind);
    -- Text that is expensive to obtain, kept until its source changes.
    CREATE TABLE text_cache (
        namespace   TEXT NOT NULL,
        key         TEXT NOT NULL,
        fingerprint TEXT NOT NULL,
        value       TEXT NOT NULL,
        PRIMARY KEY (namespace, key)
    );
";

/// Why the cache could not be used.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// SQLite reported an error.
    #[error("the cache database reported an error: {0}")]
    Database(#[from] rusqlite::Error),
    /// A stored value is not what this version of Mod Mallet wrote.
    #[error("the cache contains data that cannot be read: {0}")]
    Corrupt(#[from] serde_json::Error),
}

/// An open cache.
#[derive(Debug)]
pub struct Store {
    connection: Connection,
}

impl Store {
    /// Opens the cache file at `path`, creating it if it does not exist.
    ///
    /// The directory must already exist.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::prepare(Connection::open(path)?)
    }

    /// Opens a cache that lives in memory only, for tests.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(connection: Connection) -> Result<Self, StoreError> {
        let found: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if found != SCHEMA_VERSION {
            // A cache from another version, or a new file. Either way there
            // is nothing worth keeping: start over.
            connection.execute_batch(
                "DROP TABLE IF EXISTS finding;
                 DROP TABLE IF EXISTS scan;
                 DROP TABLE IF EXISTS text_cache;",
            )?;
            connection.execute_batch(SCHEMA)?;
            connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        // The file holds nothing that cannot be recreated, so there is no
        // reason to wait for the disk after every write.
        connection.pragma_update(None, "synchronous", "OFF")?;
        Ok(Self { connection })
    }

    /// Replaces the stored scan with `report`.
    pub fn save_scan(&mut self, report: &ScanReport) -> Result<(), StoreError> {
        // Findings get a row each so that they can be queried; everything
        // else in the report is kept as one value.
        let without_findings = ScanReport {
            findings: Vec::new(),
            ..report.clone()
        };

        let transaction = self.connection.transaction()?;
        transaction.execute("DELETE FROM finding", [])?;
        transaction.execute("DELETE FROM scan", [])?;
        transaction.execute(
            "INSERT INTO scan (id, report_json) VALUES (1, ?1)",
            params![serde_json::to_string(&without_findings)?],
        )?;
        {
            let mut insert =
                transaction.prepare("INSERT INTO finding (id, kind, json) VALUES (?1, ?2, ?3)")?;
            for finding in &report.findings {
                insert.execute(params![
                    finding.id.as_str(),
                    finding.kind,
                    serde_json::to_string(finding)?,
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Runs `work` with all its writes to the cache gathered into one
    /// step, which is much faster than thousands of single writes.
    pub fn batched<T>(&self, work: impl FnOnce() -> T) -> Result<T, StoreError> {
        self.connection.execute_batch("BEGIN")?;
        let result = work();
        self.connection.execute_batch("COMMIT")?;
        Ok(result)
    }

    /// The text kept under `namespace` and `key`, if it was stored with
    /// this `fingerprint`.
    ///
    /// The fingerprint stands for the state of whatever the text was
    /// derived from, for example the size and date of a file. A text stored
    /// under another fingerprint is out of date and is not returned.
    pub fn cached_text(
        &self,
        namespace: &str,
        key: &str,
        fingerprint: &str,
    ) -> Result<Option<String>, StoreError> {
        Ok(self
            .connection
            .query_row(
                "SELECT value FROM text_cache
                 WHERE namespace = ?1 AND key = ?2 AND fingerprint = ?3",
                params![namespace, key, fingerprint],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Keeps `value` under `namespace` and `key`, replacing what was there.
    pub fn cache_text(
        &self,
        namespace: &str,
        key: &str,
        fingerprint: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO text_cache (namespace, key, fingerprint, value)
             VALUES (?1, ?2, ?3, ?4)",
            params![namespace, key, fingerprint, value],
        )?;
        Ok(())
    }

    /// The stored scan, or `None` if nothing has been stored yet.
    pub fn load_scan(&self) -> Result<Option<ScanReport>, StoreError> {
        let report_json = self
            .connection
            .query_row("SELECT report_json FROM scan WHERE id = 1", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?;
        let Some(report_json) = report_json else {
            return Ok(None);
        };
        let mut report: ScanReport = serde_json::from_str(&report_json)?;

        let mut select = self
            .connection
            .prepare("SELECT json FROM finding ORDER BY position")?;
        let rows = select.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            report
                .findings
                .push(serde_json::from_str::<Finding>(&row?)?);
        }
        Ok(Some(report))
    }
}

impl TextCache for Store {
    fn get(&self, namespace: &str, key: &str, fingerprint: &str) -> Option<String> {
        self.cached_text(namespace, key, fingerprint).ok().flatten()
    }

    fn put(&self, namespace: &str, key: &str, fingerprint: &str, value: &str) {
        // A text that could not be kept is simply read again next time.
        let _ = self.cache_text(namespace, key, fingerprint, value);
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::{GameInfo, GamePath, Subject, kinds, report};

    use super::*;

    fn sample_report(file_names: &[&str]) -> ScanReport {
        ScanReport {
            schema_version: report::SCHEMA_VERSION,
            scanned_at: 1_794_695_581,
            game: GameInfo {
                id: "skyrimse".to_owned(),
                name: "Skyrim Special Edition".to_owned(),
                version: Some("1.7.104".to_owned()),
                extender_version: Some("2.3.1".to_owned()),
                unreal_visuals: false,
            },
            extender_plugins_checked: 46,
            plugins_checked: 300,
            archives_checked: 120,
            installers_checked: 40,
            meshes_checked: 900,
            locations: Vec::new(),
            mods: Vec::new(),
            map: mallet_core::WorldMap::default(),
            object_swapper: false,
            installed_patch: None,
            duels: Vec::new(),
            unreal_paks: Vec::new(),
            findings: file_names
                .iter()
                .map(|file_name| {
                    Finding::new(
                        &kinds::EXTENDER_PLUGIN_INCOMPATIBLE,
                        Subject::File {
                            path: GamePath::new("data/skse/plugins").join(file_name),
                        },
                    )
                    .with_arg("file", *file_name)
                    .with_arg("gameVersion", "1.7.104")
                })
                .collect(),
        }
    }

    #[test]
    fn a_new_cache_has_no_scan() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.load_scan().unwrap(), None);
    }

    #[test]
    fn a_saved_scan_is_loaded_unchanged_and_in_order() {
        let report = sample_report(&["Zeta.dll", "Alpha.dll", "Mid.dll"]);
        let mut store = Store::open_in_memory().unwrap();
        store.save_scan(&report).unwrap();
        assert_eq!(store.load_scan().unwrap(), Some(report));
    }

    #[test]
    fn saving_replaces_the_previous_scan() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .save_scan(&sample_report(&["Old.dll", "Older.dll"]))
            .unwrap();
        let newer = sample_report(&["New.dll"]);
        store.save_scan(&newer).unwrap();
        assert_eq!(store.load_scan().unwrap(), Some(newer));
    }

    #[test]
    fn the_cache_survives_being_closed() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cache.sqlite");
        let report = sample_report(&["Old.dll"]);

        Store::open(&path).unwrap().save_scan(&report).unwrap();
        assert_eq!(
            Store::open(&path).unwrap().load_scan().unwrap(),
            Some(report)
        );
    }

    #[test]
    fn a_cache_from_another_version_is_emptied() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cache.sqlite");
        Store::open(&path)
            .unwrap()
            .save_scan(&sample_report(&["Old.dll"]))
            .unwrap();

        // Pretend a different version of Mod Mallet wrote the file.
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();

        assert_eq!(Store::open(&path).unwrap().load_scan().unwrap(), None);
    }

    #[test]
    fn cached_text_is_returned_only_for_the_fingerprint_it_was_stored_with() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.cached_text("fomod", "a.7z", "10:1").unwrap(), None);

        store
            .cache_text("fomod", "a.7z", "10:1", "<config/>")
            .unwrap();
        assert_eq!(
            store
                .cached_text("fomod", "a.7z", "10:1")
                .unwrap()
                .as_deref(),
            Some("<config/>")
        );
        // The file changed, or another kind of text is asked for.
        assert_eq!(store.cached_text("fomod", "a.7z", "11:2").unwrap(), None);
        assert_eq!(store.cached_text("other", "a.7z", "10:1").unwrap(), None);

        // Storing again replaces the entry.
        store.cache_text("fomod", "a.7z", "11:2", "<new/>").unwrap();
        assert_eq!(store.cached_text("fomod", "a.7z", "10:1").unwrap(), None);
        assert_eq!(
            store
                .cached_text("fomod", "a.7z", "11:2")
                .unwrap()
                .as_deref(),
            Some("<new/>")
        );
    }

    #[test]
    fn batched_writes_are_kept() {
        let store = Store::open_in_memory().unwrap();
        let written = store
            .batched(|| {
                for index in 0..100 {
                    TextCache::put(&store, "nif", &format!("mesh{index}"), "1:1", "textures");
                }
                100
            })
            .unwrap();
        assert_eq!(written, 100);
        assert_eq!(
            TextCache::get(&store, "nif", "mesh99", "1:1").as_deref(),
            Some("textures")
        );
        assert_eq!(TextCache::get(&store, "nif", "mesh99", "2:2"), None);
    }

    #[test]
    fn saving_a_scan_keeps_the_text_cache() {
        let mut store = Store::open_in_memory().unwrap();
        store
            .cache_text("fomod", "a.7z", "10:1", "<config/>")
            .unwrap();
        store.save_scan(&sample_report(&["Old.dll"])).unwrap();
        assert!(
            store
                .cached_text("fomod", "a.7z", "10:1")
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn unreadable_content_is_an_error_not_a_crash() {
        let store = Store::open_in_memory().unwrap();
        store
            .connection
            .execute("INSERT INTO scan VALUES (1, 'not json')", [])
            .unwrap();
        assert!(matches!(store.load_scan(), Err(StoreError::Corrupt(_))));
    }
}
