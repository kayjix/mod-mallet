//! Reports meant for sharing (ADR 0014, ADR 0091).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mallet_core::{LocationRole, Placeholder, ScanReport, SharedReport, Shortener};
use serde_json::Value;

use crate::EngineError;

/// Fields whose text is data rather than words or paths, such as the
/// heights of the land in Base64. Shortening them could only break them.
const DATA_FIELDS: [&str; 3] = ["heights", "water", "image"];

/// The private folders and names of this machine and of `report`'s
/// setup: the game directory, the mod manager's folders, the user's
/// folders, the user name and the computer's name.
pub fn machine_shortener(report: &ScanReport) -> Shortener {
    let mut shortener = Shortener::new();
    for location in &report.locations {
        let placeholder = match location.role {
            LocationRole::GameDir => Placeholder::Game,
            LocationRole::ModStaging | LocationRole::ManagerMods => Placeholder::Staging,
            LocationRole::ManagerInstance => Placeholder::Instance,
            LocationRole::Downloads => Placeholder::Downloads,
            _ => continue,
        };
        shortener = shortener.with_root(&location.path, placeholder);
    }
    let folders = [
        (dirs::document_dir(), Placeholder::Documents),
        (dirs::data_local_dir(), Placeholder::LocalAppData),
        (dirs::config_dir(), Placeholder::AppData),
        (dirs::home_dir(), Placeholder::UserProfile),
    ];
    for (folder, placeholder) in folders {
        if let Some(folder) = folder {
            shortener = shortener.with_root(&folder.to_string_lossy(), placeholder);
        }
    }
    let home_name = dirs::home_dir().and_then(|home| {
        home.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    });
    let names = [
        (std::env::var("USERNAME").ok(), Placeholder::User),
        (std::env::var("USER").ok(), Placeholder::User),
        (home_name, Placeholder::User),
        (std::env::var("COMPUTERNAME").ok(), Placeholder::Machine),
        (std::env::var("HOSTNAME").ok(), Placeholder::Machine),
    ];
    for (name, placeholder) in names {
        if let Some(name) = name {
            shortener = shortener.with_name(&name, placeholder);
        }
    }
    shortener
}

/// `report` made fit for sharing with the private folders and names of
/// this machine replaced (ADR 0091).
pub fn share_report(report: &ScanReport) -> Result<SharedReport, EngineError> {
    share_report_with(report, &machine_shortener(report))
}

/// `report` made fit for sharing by `shortener`: every text in it, at any
/// depth, shortened. Mod names, plugin names and findings stay.
pub fn share_report_with(
    report: &ScanReport,
    shortener: &Shortener,
) -> Result<SharedReport, EngineError> {
    let mut value =
        serde_json::to_value(report).map_err(|error| EngineError::Share(error.to_string()))?;
    let mut used = BTreeSet::new();
    shorten_value(&mut value, shortener, &mut used);
    let report =
        serde_json::from_value(value).map_err(|error| EngineError::Share(error.to_string()))?;
    Ok(SharedReport {
        report,
        placeholders: Placeholder::ALL
            .into_iter()
            .filter(|placeholder| used.contains(placeholder))
            .collect(),
    })
}

fn shorten_value(value: &mut Value, shortener: &Shortener, used: &mut BTreeSet<Placeholder>) {
    match value {
        Value::String(text) => *text = shortener.shorten(text, used),
        Value::Array(items) => {
            for item in items {
                shorten_value(item, shortener, used);
            }
        }
        Value::Object(fields) => {
            let old = std::mem::take(fields);
            for (key, mut field) in old {
                if !DATA_FIELDS.contains(&key.as_str()) {
                    shorten_value(&mut field, shortener, used);
                }
                fields.insert(shortener.shorten(&key, used), field);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// Where shared reports are saved: a folder of Mod Mallet's own.
pub fn shared_report_dir() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join("mod-mallet").join("shared"))
}

/// What a shared report is saved as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedFile<'a> {
    /// The text of the report, Markdown, as `mallet-messages` puts it.
    Markdown(&'a str),
    /// The report itself, as JSON of the scan report's schema.
    Json,
}

/// Saves `shared` as `mod-mallet-report-<game>.md` or `.json` into
/// [`shared_report_dir`] and returns where it is. An earlier file of the
/// same game is replaced. Only ever called because the user asked for it.
pub fn save_shared_report(
    shared: &SharedReport,
    file: SharedFile<'_>,
) -> Result<PathBuf, EngineError> {
    let (contents, extension) = match file {
        SharedFile::Markdown(text) => (text.to_owned(), "md"),
        SharedFile::Json => (
            serde_json::to_string_pretty(&shared.report)
                .map_err(|error| EngineError::Share(error.to_string()))?,
            "json",
        ),
    };
    let game: String = shared
        .report
        .game
        .id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let folder = shared_report_dir().ok_or_else(|| {
        EngineError::Share("no folder for Mod Mallet's own files is known".to_owned())
    })?;
    let path = folder.join(format!("mod-mallet-report-{game}.{extension}"));
    write_own_file(&path, &contents)?;
    tracing::info!(path = %path.display(), "shared report written");
    Ok(path)
}

fn write_own_file(path: &Path, contents: &str) -> Result<(), EngineError> {
    crate::create_folder_of(path)?;
    std::fs::write(path, contents).map_err(EngineError::not_written(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mallet_core::{
        ArgValue, Finding, GameInfo, LocationOrigin, MapTerrain, MapWorld, ScanLocation, Subject,
        kinds,
    };

    fn report() -> ScanReport {
        let location = |role, path: &str| ScanLocation {
            role,
            path: path.to_owned(),
            origin: LocationOrigin::Standard,
            found: true,
        };
        let mut report: ScanReport = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "scannedAt": 0,
            "game": { "id": "skyrimse", "name": "Skyrim Special Edition" },
            "extenderPluginsChecked": 0,
            "findings": [],
        }))
        .unwrap();
        report.game = GameInfo {
            version: Some("1.6.1170".to_owned()),
            ..report.game
        };
        report.locations = vec![
            location(LocationRole::GameDir, r"E:\Games\Skyrim Special Edition"),
            location(LocationRole::ModStaging, r"E:\Vortex Mods\skyrimse"),
            location(
                LocationRole::ExtenderLog,
                r"C:\Users\Alice\Documents\My Games\Skyrim Special Edition\SKSE\skse64.log",
            ),
        ];
        report.findings.push(
            Finding::new(&kinds::CRASHES_RECENT, Subject::Setup)
                .with_arg("last", r"C:\Users\Alice\Documents\crash.log"),
        );
        report.map.worlds.push(MapWorld {
            terrain: Some(MapTerrain {
                west: 0,
                south: 0,
                columns: 1,
                rows: 1,
                samples: 1,
                heights: "/Alice/x".to_owned(),
                water: String::new(),
                lowest: 0,
                highest: 0,
            }),
            ..MapWorld::default()
        });
        report
    }

    fn alice() -> Shortener {
        Shortener::new()
            .with_root(r"E:\Games\Skyrim Special Edition", Placeholder::Game)
            .with_root(r"E:\Vortex Mods\skyrimse", Placeholder::Staging)
            .with_root(r"C:\Users\Alice", Placeholder::UserProfile)
            .with_root(r"C:\Users\Alice\Documents", Placeholder::Documents)
            .with_name("Alice", Placeholder::User)
    }

    #[test]
    fn every_text_of_the_report_is_shortened() {
        let shared = share_report_with(&report(), &alice()).unwrap();
        let paths: Vec<_> = shared
            .report
            .locations
            .iter()
            .map(|location| location.path.as_str())
            .collect();
        assert_eq!(
            paths,
            [
                "<game>",
                "<staging>",
                r"<documents>\My Games\Skyrim Special Edition\SKSE\skse64.log"
            ]
        );
        assert_eq!(
            shared.report.findings[0].args["last"],
            ArgValue::from(r"<documents>\crash.log")
        );
        assert_eq!(
            shared.placeholders,
            [
                Placeholder::Game,
                Placeholder::Staging,
                Placeholder::Documents
            ]
        );
    }

    #[test]
    fn no_original_folder_survives() {
        let shared = share_report_with(&report(), &alice()).unwrap();
        let mut json = serde_json::to_value(&shared.report).unwrap();
        // The heights are data, kept as they are.
        json["map"]["worlds"][0]["terrain"]["heights"] = Value::Null;
        let text = json.to_string();
        for private in ["Alice", r"E:\\Games", "Vortex Mods"] {
            assert!(!text.contains(private), "{private} in {text}");
        }
    }

    #[test]
    fn data_is_left_alone() {
        let shared = share_report_with(&report(), &alice()).unwrap();
        let terrain = shared.report.map.worlds[0].terrain.as_ref().unwrap();
        assert_eq!(terrain.heights, "/Alice/x");
    }

    #[test]
    fn the_machine_shortener_knows_the_setups_folders() {
        let shortener = machine_shortener(&report());
        let mut used = BTreeSet::new();
        assert_eq!(
            shortener.shorten(r"E:\Games\Skyrim Special Edition\Data", &mut used),
            r"<game>\Data"
        );
        if let Some(home) = dirs::home_dir().filter(|home| home.components().count() > 2) {
            let home = home.to_string_lossy();
            assert!(!shortener.shorten(&home, &mut used).contains(&*home));
        }
    }
}
