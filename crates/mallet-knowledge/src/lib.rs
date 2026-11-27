//! Community-maintained rules about plugins, from the LOOT masterlist.
//!
//! LOOT's masterlist says, for thousands of plugins, which files they need,
//! which they cannot be used with, and which released versions contain
//! records that should be cleaned out (ADR 0008). Mod managers download it
//! and keep it on the machine, so reading it needs no network (ADR 0014).
//!
//! The rules carry conditions, such as "only if this other mod is active"
//! or "only for this version of the file". They are evaluated with LOOT's
//! own library, `libloot`, against the game as it is, so the facts handed
//! on hold only the rules that apply to this setup. Evaluating them only
//! reads: files are looked up and checksums computed, nothing is written.

use std::path::{Path, PathBuf};

use libloot::metadata::{File, Message, MessageType, PluginCleaningData, select_message_content};
use libloot::{EvalMode, Game, GameType, MergeMode};
use mallet_core::facts::knowledge::{
    Cleaning, GeneralMessage, KnownFile, KnownMessage, MessageLevel, PluginKnowledge,
};
use mallet_core::facts::{KnowledgeFacts, PluginFacts};

/// Why the rules could not be read.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    /// The game could not be opened for evaluating the rules.
    #[error("the game could not be opened for checking LOOT's rules: {0}")]
    Game(String),
    /// The masterlist could not be read.
    #[error("the LOOT masterlist `{}` could not be read: {reason}", .path.display())]
    Masterlist {
        /// The masterlist.
        path: PathBuf,
        /// What went wrong, in the words of the reader.
        reason: String,
    },
}

/// LOOT's kind of game for the game with the mod manager's identifier
/// `game` (`skyrimse`), and the folder LOOT keeps that game's data in.
fn loot_game(game: &str) -> Option<(GameType, &'static str)> {
    Some(match game {
        "skyrimse" => (GameType::SkyrimSE, "Skyrim Special Edition"),
        "skyrimvr" => (GameType::SkyrimVR, "Skyrim VR"),
        "skyrim" => (GameType::Skyrim, "Skyrim"),
        "fallout4" => (GameType::Fallout4, "Fallout4"),
        "fallout4vr" => (GameType::Fallout4VR, "Fallout4VR"),
        "falloutnv" => (GameType::FalloutNV, "FalloutNV"),
        "fallout3" => (GameType::Fallout3, "Fallout3"),
        "oblivion" => (GameType::Oblivion, "Oblivion"),
        // LOOT reads Oblivion Remastered from its installation's top folder
        // and finds the old game's data below it (ADR 0065, 0092).
        "oblivionremastered" => (GameType::OblivionRemastered, "Oblivion Remastered"),
        // LOOT reads Enderal as Skyrim Special Edition, from a folder of its
        // own (ADR 0078).
        "enderalspecialedition" => (GameType::SkyrimSE, "Enderal Special Edition"),
        _ => return None,
    })
}

/// The places a mod manager or LOOT itself keeps the masterlist for the
/// game with the mod manager's identifier `game` (`skyrimse`) on this
/// machine, whether or not a file is there.
///
/// Vortex keeps its copy in its own data folder and updates it while it
/// runs; LOOT keeps one per game in its local data folder.
pub fn standard_places(game: &str) -> Vec<PathBuf> {
    let mut places = Vec::new();
    let Some((_, loot_folder)) = loot_game(game) else {
        return places;
    };
    if let Some(roaming) = dirs::config_dir() {
        places.push(
            roaming
                .join("Vortex")
                .join(game)
                .join("masterlist")
                .join("masterlist.yaml"),
        );
    }
    if let Some(local) = dirs::data_local_dir() {
        places.push(
            local
                .join("LOOT")
                .join("games")
                .join(loot_folder)
                .join("masterlist.yaml"),
        );
    }
    places
}

/// The masterlist of `game` to read: the most recently updated of those
/// in [`standard_places`], or `None` if there is none.
pub fn find_masterlist(game: &str) -> Option<PathBuf> {
    standard_places(game)
        .into_iter()
        .filter_map(|path| {
            let modified = std::fs::metadata(&path).ok()?.modified().ok()?;
            Some((modified, path))
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

/// Reads what the masterlist at `masterlist` says about the active
/// `plugins` of the game with the mod manager's identifier `game`
/// (`skyrimse`), installed in `game_dir`.
///
/// `local_dir` is the directory of the list of active plugins, if it is not
/// the standard one. LOOT's library reads the list as `Plugins.txt`; on a
/// file system that tells case apart, a list spelt otherwise (as Mod
/// Organizer's `plugins.txt`) has to be handed over under that name. `shipped` names, in lower case, the plugins that come
/// with the game. A plugin whose rules cannot be evaluated is left out and
/// logged; the others are still reported.
pub fn read(
    game: &str,
    game_dir: &Path,
    local_dir: Option<&Path>,
    masterlist: &Path,
    plugins: &[PluginFacts],
    shipped: &[String],
) -> Result<KnowledgeFacts, KnowledgeError> {
    let (game_type, _) =
        loot_game(game).ok_or_else(|| KnowledgeError::Game(format!("unknown game `{game}`")))?;
    let mut game = match local_dir {
        Some(local_dir) => Game::with_local_path(game_type, game_dir, local_dir),
        None => Game::new(game_type, game_dir),
    }
    .map_err(|error| KnowledgeError::Game(error.to_string()))?;
    game.load_current_load_order_state()
        .map_err(|error| KnowledgeError::Game(error.to_string()))?;

    let database = game.database();
    let mut database = database
        .write()
        .map_err(|error| KnowledgeError::Masterlist {
            path: masterlist.to_path_buf(),
            reason: error.to_string(),
        })?;
    database
        .load_masterlist(masterlist)
        .map_err(|error| KnowledgeError::Masterlist {
            path: masterlist.to_path_buf(),
            reason: error.to_string(),
        })?;

    let present = |file: &File| -> Option<bool> {
        let exists = evaluate(&database, &file_condition(file.name().as_str()))?;
        match file.constraint() {
            Some(constraint) if exists => evaluate(&database, constraint),
            _ => Some(exists),
        }
    };
    let known = |files: &[File]| -> Vec<KnownFile> {
        files
            .iter()
            .filter_map(|file| {
                let (display, page) = file.display_name().map_or((None, None), split_link);
                Some(KnownFile {
                    name: file.name().as_str().to_owned(),
                    display,
                    page,
                    present: present(file)?,
                })
            })
            .collect()
    };

    let mut knowledge = KnowledgeFacts::default();
    for plugin in plugins {
        let metadata = match database.plugin_metadata(
            &plugin.name,
            MergeMode::WithoutUserMetadata,
            EvalMode::Evaluate,
        ) {
            Ok(Some(metadata)) => metadata,
            Ok(None) => continue,
            Err(error) => {
                tracing::warn!(plugin = %plugin.name, %error, "LOOT's rules for this plugin could not be evaluated");
                continue;
            }
        };
        let entry = PluginKnowledge {
            plugin: plugin.name.clone(),
            path: plugin.path.clone(),
            shipped_with_game: shipped.contains(&plugin.name.to_lowercase()),
            requirements: known(metadata.requirements()),
            incompatibilities: known(metadata.incompatibilities()),
            dirty: metadata.dirty_info().iter().map(cleaning).collect(),
            messages: metadata.messages().iter().filter_map(message).collect(),
        };
        if !(entry.requirements.is_empty()
            && entry.incompatibilities.is_empty()
            && entry.dirty.is_empty()
            && entry.messages.is_empty())
        {
            knowledge.plugins.push(entry);
        }
    }
    match database.general_messages(MergeMode::WithoutUserMetadata, EvalMode::Evaluate) {
        Ok(general) => {
            knowledge.general = general
                .iter()
                .filter_map(|general| {
                    let named = plugins_named(general.condition()?, plugins);
                    let message = message(general)?;
                    (!named.is_empty()).then_some(GeneralMessage {
                        message,
                        plugins: named,
                    })
                })
                .collect();
            knowledge.version_messages = general
                .iter()
                .filter(|general| general.condition().is_some_and(is_about_versions))
                .filter_map(message)
                .filter(|message| message.level >= MessageLevel::Warning)
                .collect();
        }
        Err(error) => tracing::warn!(%error, "LOOT's general messages could not be evaluated"),
    }
    Ok(knowledge)
}

/// The active `plugins` a LOOT condition names in its `active("…")` calls,
/// by file name. Each name is matched as LOOT matches it: as a regular
/// expression for the whole file name, ignoring case, which a plain name
/// matches by itself.
fn plugins_named(condition: &str, plugins: &[PluginFacts]) -> Vec<String> {
    let mut named = Vec::new();
    let mut rest = condition;
    while let Some(start) = rest.find("active(\"") {
        let after = rest.get(start + "active(\"".len()..).unwrap_or_default();
        let Some(end) = after.find("\")") else {
            break;
        };
        let pattern = after.get(..end).unwrap_or_default();
        rest = after.get(end..).unwrap_or_default();
        let Ok(regex) = regex::RegexBuilder::new(&format!("^(?:{pattern})$"))
            .case_insensitive(true)
            .build()
        else {
            continue;
        };
        for plugin in plugins {
            if regex.is_match(&plugin.name) && !named.contains(&plugin.name) {
                named.push(plugin.name.clone());
            }
        }
    }
    named
}

/// Whether a LOOT condition is about the version of a file, such as the
/// game's program or the script extender's loader, and names no plugin:
/// a message about the setup's versions (ADR 0079).
fn is_about_versions(condition: &str) -> bool {
    condition.contains("version(") && !condition.contains("active(")
}

/// A message of LOOT as plain English text, or `None` if it has none.
fn message(message: &Message) -> Option<KnownMessage> {
    let level = match message.message_type() {
        MessageType::Error => MessageLevel::Error,
        MessageType::Warn => MessageLevel::Warning,
        MessageType::Say => MessageLevel::Note,
    };
    let text = plain_text(select_message_content(message.content(), "en")?.text());
    (!text.is_empty()).then_some(KnownMessage { level, text })
}

/// Markdown as LOOT's messages use it, as plain text: emphasis marks are
/// dropped, and a link `[text](address)` becomes `text (address)`.
fn plain_text(markdown: &str) -> String {
    let mut text = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(start) = rest.find('[') {
        let (before, from_bracket) = rest.split_at(start);
        text.push_str(before);
        let link = from_bracket
            .get(1..)
            .and_then(|inner| inner.split_once("]("))
            .and_then(|(label, after)| {
                let (address, after) = after.split_once(')')?;
                Some((label, address, after))
            });
        match link {
            Some((label, address, after)) if !label.contains('[') => {
                text.push_str(label);
                text.push_str(" (");
                text.push_str(address.trim());
                text.push(')');
                rest = after;
            }
            _ => {
                text.push('[');
                rest = from_bracket.get(1..).unwrap_or_default();
            }
        }
    }
    text.push_str(rest);
    text.replace("**", "").replace("__", "").trim().to_owned()
}

/// Evaluates a LOOT condition, or `None` if it cannot be: a rule that
/// cannot be judged is not reported.
fn evaluate(database: &libloot::Database, condition: &str) -> Option<bool> {
    database
        .evaluate(condition)
        .map_err(|error| tracing::debug!(condition, %error, "LOOT condition not evaluated"))
        .ok()
}

/// The LOOT condition that holds when the file named by a rule is there,
/// with LOOT's own rules for plugins that are switched off by renaming.
fn file_condition(name: &str) -> String {
    format!("file(\"{}\")", name.replace('"', ""))
}

/// The counts of a cleaning report.
fn cleaning(data: &PluginCleaningData) -> Cleaning {
    Cleaning {
        identical_records: data.itm_count(),
        deleted_references: data.deleted_reference_count(),
        deleted_navmeshes: data.deleted_navmesh_count(),
        utility: split_link(data.cleaning_utility()).0.unwrap_or_default(),
    }
}

/// The text and the address of a name the masterlist writes as a Markdown
/// link, `[text](address)`. A name without a link is all text.
fn split_link(display: &str) -> (Option<String>, Option<String>) {
    let display = display.trim();
    let link = display
        .strip_prefix('[')
        .and_then(|rest| rest.split_once("]("))
        .and_then(|(text, rest)| Some((text, rest.strip_suffix(')')?)));
    match link {
        Some((text, address)) => (
            Some(text.trim().to_owned()).filter(|text| !text.is_empty()),
            Some(address.trim().to_owned()).filter(|address| address.starts_with("https://")),
        ),
        None => (
            Some(display.to_owned()).filter(|text| !text.is_empty()),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mallet_core::GamePath;
    use proptest::prelude::*;

    use super::*;

    /// A game folder with the given plugins in `Data` and a plugin list
    /// that activates them, and a local folder holding that list.
    struct Setup {
        root: tempfile::TempDir,
    }

    impl Setup {
        fn new(plugins: &[&str], other_files: &[&str]) -> Self {
            let root = tempfile::tempdir().unwrap();
            let data = root.path().join("game").join("Data");
            fs::create_dir_all(&data).unwrap();
            fs::create_dir_all(root.path().join("local")).unwrap();
            for name in plugins {
                fs::write(data.join(name), plugin_bytes()).unwrap();
            }
            for name in other_files {
                let path = data.join(name);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, b"x").unwrap();
            }
            let list = plugins
                .iter()
                .map(|name| format!("*{name}"))
                .collect::<Vec<_>>()
                .join(
                    "
",
                );
            // Named as the game names it; see `read` on other spellings.
            fs::write(root.path().join("local").join("Plugins.txt"), list).unwrap();
            Self { root }
        }

        fn masterlist(&self, yaml: &str) -> PathBuf {
            let path = self.root.path().join("masterlist.yaml");
            fs::write(&path, yaml).unwrap();
            path
        }

        fn read(&self, masterlist: &Path, active: &[&str]) -> KnowledgeFacts {
            let plugins: Vec<PluginFacts> = active
                .iter()
                .map(|name| PluginFacts {
                    name: (*name).to_owned(),
                    path: GamePath::new("data").join(name),
                    masters: Vec::new(),
                    npcs: Vec::new(),
                })
                .collect();
            read(
                "skyrimse",
                &self.root.path().join("game"),
                Some(&self.root.path().join("local")),
                masterlist,
                &plugins,
                &["skyrim.esm".to_owned()],
            )
            .unwrap()
        }
    }

    /// The smallest valid Skyrim SE plugin: a header record with nothing
    /// in it.
    fn plugin_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"TES4");
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // data size
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // flags
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // form id
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // version control
        bytes.extend_from_slice(&44_u16.to_le_bytes()); // form version
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes
    }

    #[test]
    fn requirements_and_incompatibilities_say_whether_their_file_is_there() {
        let setup = Setup::new(
            &["City.esp", "Other City.esp"],
            &["SKSE/Plugins/Helper.dll"],
        );
        let masterlist = setup.masterlist(
            r"
plugins:
  - name: 'City.esp'
    req:
      - name: 'SKSE/Plugins/Helper.dll'
        display: '[Helper](https://www.nexusmods.com/skyrimspecialedition/mods/1/)'
      - 'Missing.esp'
    inc: [ 'Other City.esp', 'Gone.esp' ]
",
        );
        let knowledge = setup.read(&masterlist, &["City.esp", "Other City.esp"]);

        let [city] = knowledge.plugins.as_slice() else {
            panic!("expected rules for one plugin, got {knowledge:?}");
        };
        assert_eq!(city.plugin, "City.esp");
        let requirements: Vec<(&str, bool)> = city
            .requirements
            .iter()
            .map(|file| (file.label(), file.present))
            .collect();
        assert_eq!(requirements, [("Helper", true), ("Missing.esp", false)]);
        assert_eq!(
            city.requirements[0].page.as_deref(),
            Some("https://www.nexusmods.com/skyrimspecialedition/mods/1/")
        );
        let incompatible: Vec<(&str, bool)> = city
            .incompatibilities
            .iter()
            .map(|file| (file.label(), file.present))
            .collect();
        assert_eq!(
            incompatible,
            [("Other City.esp", true), ("Gone.esp", false)]
        );
    }

    #[test]
    fn oblivion_remastered_is_read_from_its_installation() {
        // Oblivion Remastered keeps the old game's data four folders below
        // its installation, with the list of active plugins in `Data`.
        let root = tempfile::tempdir().unwrap();
        let data = root
            .path()
            .join("OblivionRemastered/Content/Dev/ObvData/Data");
        fs::create_dir_all(data.join("OBSE/Plugins")).unwrap();
        for name in ["Oblivion.esm", "Arena.esp"] {
            fs::write(data.join(name), oblivion_plugin_bytes()).unwrap();
        }
        fs::write(data.join("OBSE/Plugins/Helper.dll"), b"x").unwrap();
        fs::write(data.join("Plugins.txt"), "Oblivion.esm\r\nArena.esp\r\n").unwrap();
        let masterlist = root.path().join("masterlist.yaml");
        fs::write(
            &masterlist,
            r#"
plugins:
  - name: 'Arena.esp'
    req: [ 'OBSE/Plugins/Helper.dll', 'Missing.esp' ]
    msg:
      - type: warn
        content: 'Only when it is active.'
        condition: 'active("Arena.esp")'
"#,
        )
        .unwrap();
        let plugins: Vec<PluginFacts> = ["Oblivion.esm", "Arena.esp"]
            .iter()
            .map(|name| PluginFacts {
                name: (*name).to_owned(),
                path: GamePath::new("data").join(name),
                masters: Vec::new(),
                npcs: Vec::new(),
            })
            .collect();

        let knowledge = read(
            "oblivionremastered",
            root.path(),
            None,
            &masterlist,
            &plugins,
            &["oblivion.esm".to_owned()],
        )
        .unwrap();

        let [arena] = knowledge.plugins.as_slice() else {
            panic!("expected rules for one plugin, got {knowledge:?}");
        };
        let requirements: Vec<(&str, bool)> = arena
            .requirements
            .iter()
            .map(|file| (file.label(), file.present))
            .collect();
        assert_eq!(
            requirements,
            [("OBSE/Plugins/Helper.dll", true), ("Missing.esp", false)]
        );
        assert_eq!(arena.messages.len(), 1, "{arena:?}");
    }

    #[test]
    fn oblivion_remastered_has_its_own_places() {
        let places = standard_places("oblivionremastered");
        assert!(
            places.iter().any(|place| place.ends_with(Path::new(
                "Vortex/oblivionremastered/masterlist/masterlist.yaml"
            )) || place
                .ends_with(Path::new("LOOT/games/Oblivion Remastered/masterlist.yaml"))),
            "{places:?}"
        );
    }

    /// The smallest valid Oblivion plugin: a header record of Oblivion's
    /// 20 bytes, holding a header field.
    fn oblivion_plugin_bytes() -> Vec<u8> {
        let mut hedr = Vec::new();
        hedr.extend_from_slice(b"HEDR");
        hedr.extend_from_slice(&12_u16.to_le_bytes());
        hedr.extend_from_slice(&1.0_f32.to_le_bytes()); // version
        hedr.extend_from_slice(&0_u32.to_le_bytes()); // records
        hedr.extend_from_slice(&0x800_u32.to_le_bytes()); // next object id
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"TES4");
        bytes.extend_from_slice(&u32::try_from(hedr.len()).unwrap().to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // flags
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // form id
        bytes.extend_from_slice(&0_u32.to_le_bytes()); // version control
        bytes.extend_from_slice(&hedr);
        bytes
    }

    #[test]
    fn rules_whose_condition_fails_are_left_out() {
        let setup = Setup::new(&["City.esp"], &[]);
        let masterlist = setup.masterlist(
            r#"
plugins:
  - name: 'City.esp'
    req:
      - name: 'Patch.esp'
        condition: 'active("Dungeons.esp")'
"#,
        );
        let knowledge = setup.read(&masterlist, &["City.esp"]);
        assert_eq!(knowledge.plugins, []);
    }

    #[test]
    fn a_dirty_version_counts_only_if_the_checksum_matches() {
        let setup = Setup::new(&["City.esp", "Town.esp"], &[]);
        let crc = crc32(&plugin_bytes());
        let masterlist = setup.masterlist(&format!(
            r"
plugins:
  - name: 'City.esp'
    dirty:
      - crc: 0x{crc:08X}
        util: '[SSEEdit v4.1.5](https://www.nexusmods.com/skyrimspecialedition/mods/164/)'
        itm: 3
        udr: 2
        nav: 1
  - name: 'Town.esp'
    dirty:
      - crc: 0x{other:08X}
        util: 'SSEEdit'
        itm: 9
",
            other = crc ^ 1
        ));
        let knowledge = setup.read(&masterlist, &["City.esp", "Town.esp"]);

        let [city] = knowledge.plugins.as_slice() else {
            panic!("expected rules for one plugin, got {knowledge:?}");
        };
        assert_eq!(
            city.dirty,
            [Cleaning {
                identical_records: 3,
                deleted_references: 2,
                deleted_navmeshes: 1,
                utility: "SSEEdit v4.1.5".to_owned(),
            }]
        );
    }

    #[test]
    fn a_masterlist_that_is_not_yaml_is_an_error() {
        let setup = Setup::new(&["City.esp"], &[]);
        let masterlist = setup.masterlist("plugins: [ unclosed");
        let plugins = Vec::new();
        let result = read(
            "skyrimse",
            &setup.root.path().join("game"),
            Some(&setup.root.path().join("local")),
            &masterlist,
            &plugins,
            &[],
        );
        assert!(matches!(result, Err(KnowledgeError::Masterlist { .. })));
    }

    #[test]
    fn general_messages_are_read_with_the_plugins_their_condition_names() {
        let setup = Setup::new(&["Hall.esp", "Inn.esp", "Road.esp"], &[]);
        let masterlist = setup.masterlist(
            r#"
globals:
  - type: say
    content: 'Use the [Series Patch Collection](https://example.org/series).'
    condition: 'active("(hall|Inn|Tower)\.esp") and not active("Other.esp")'
  - type: say
    content: 'Only with a mod that is not there.'
    condition: 'active("Missing.esp")'
"#,
        );
        let knowledge = setup.read(&masterlist, &["Hall.esp", "Inn.esp", "Road.esp"]);

        let [general] = knowledge.general.as_slice() else {
            panic!("expected one general message, got {knowledge:?}");
        };
        assert_eq!(
            general.message.text,
            "Use the Series Patch Collection (https://example.org/series)."
        );
        assert_eq!(general.plugins, ["Hall.esp", "Inn.esp"]);
    }

    #[test]
    fn warnings_about_versions_are_read_apart() {
        let setup = Setup::new(&["City.esp"], &["SKSE/Plugins/marker.txt"]);
        let masterlist = setup.masterlist(
            r#"
globals:
  - type: warn
    content: 'An older version of **the script extender** is installed.'
    condition: 'file("SKSE/Plugins/marker.txt") or version("../loader.exe", <, "0.2.3.1")'
  - type: say
    content: 'The script extender is up to date.'
    condition: 'file("SKSE/Plugins/marker.txt") or version("../loader.exe", >=, "0.2.3.1")'
  - type: warn
    content: 'Not about versions.'
    condition: 'file("SKSE/Plugins/marker.txt")'
"#,
        );
        let knowledge = setup.read(&masterlist, &["City.esp"]);
        assert_eq!(
            knowledge.version_messages,
            [KnownMessage {
                level: MessageLevel::Warning,
                text: "An older version of the script extender is installed.".to_owned(),
            }]
        );
        assert!(is_about_versions(
            r#"product_version("../SkyrimSE.exe", ==, "1.7.104.0")"#
        ));
        assert!(!is_about_versions(
            r#"active("x.esp") and version("x.esp", <, "1.0")"#
        ));
    }

    #[test]
    fn messages_that_apply_are_read_as_plain_text_with_their_level() {
        let setup = Setup::new(&["City.esp"], &[]);
        let masterlist = setup.masterlist(
            r#"
plugins:
  - name: 'City.esp'
    msg:
      - type: error
        content: 'You use **Town**, but not the [City - Town Patch](https://example.org/patch).'
      - type: say
        content:
          - lang: en
            text: 'Requires a new save.'
          - lang: de
            text: 'Braucht einen neuen Spielstand.'
      - type: warn
        content: 'Only with Dungeons.'
        condition: 'active("Dungeons.esp")'
"#,
        );
        let knowledge = setup.read(&masterlist, &["City.esp"]);

        let [city] = knowledge.plugins.as_slice() else {
            panic!("expected rules for one plugin, got {knowledge:?}");
        };
        assert_eq!(
            city.messages,
            [
                KnownMessage {
                    level: MessageLevel::Error,
                    text:
                        "You use Town, but not the City - Town Patch (https://example.org/patch)."
                            .to_owned(),
                },
                KnownMessage {
                    level: MessageLevel::Note,
                    text: "Requires a new save.".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn markdown_becomes_plain_text() {
        assert_eq!(
            plain_text("**bold** and [a](b) [c](d)"),
            "bold and a (b) c (d)"
        );
        assert_eq!(plain_text("no [link here"), "no [link here");
        assert_eq!(plain_text("[[nested](x)"), "[nested (x)");
        assert_eq!(plain_text("  "), "");
    }

    #[test]
    fn links_are_split_into_text_and_address() {
        assert_eq!(
            split_link("[Helper](https://example.org/a)"),
            (
                Some("Helper".to_owned()),
                Some("https://example.org/a".to_owned())
            )
        );
        assert_eq!(split_link("Helper"), (Some("Helper".to_owned()), None));
        assert_eq!(
            split_link("[Helper](javascript:x)"),
            (Some("Helper".to_owned()), None)
        );
        assert_eq!(split_link(""), (None, None));
    }

    fn crc32(bytes: &[u8]) -> u32 {
        // The CRC-32 LOOT uses (IEEE 802.3), computed bit by bit.
        let mut crc = !0_u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    proptest! {
        #[test]
        fn splitting_a_link_never_panics(display in ".*") {
            let _ = split_link(&display);
        }

        #[test]
        fn plain_text_never_panics(markdown in ".*") {
            let _ = plain_text(&markdown);
        }
    }
}
