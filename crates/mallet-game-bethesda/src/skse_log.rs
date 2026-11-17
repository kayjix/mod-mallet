//! Reads `skse64.log`, the log SKSE writes every time the game starts.
//!
//! The log is the game's own account of which native plugins it accepted,
//! so everything derived from it is a fact, not a guess.
//!
//! The line shapes below were taken from a log written by SKSE 2.3.1:
//!
//! ```text
//! SKSE64 runtime: initialize (version = 2.3.1 01070680 01DD748122C36157, os = 6.2 (9200))
//! plugin Example.dll (00000001 Example 00000007) loaded correctly (handle 3)
//! plugin Old.dll (00000001 old 00000001) disabled, incompatible with current version of the game 0 (handle 0)
//! plugin helper.dll (00000000  00000000) no version data 0 (handle 0)
//! ```
//!
//! A log is untrusted input like any other file (ADR 0013): [`parse`]
//! accepts any text, skips what it does not recognise and never panics.

use mallet_core::GamePath;
use mallet_core::facts::{ExtenderPlugin, ExtenderPluginStatus, ExtenderReport};

use crate::EXTENDER_PLUGIN_DIR;

const HEADER_PREFIX: &str = "SKSE64 runtime: initialize (version = ";
const PLUGIN_PREFIX: &str = "plugin ";
const HANDLE_MARKER: &str = " (handle ";

const STATUS_LOADED: &str = "loaded correctly";
const STATUS_INCOMPATIBLE: &str = "disabled, incompatible with current version of the game";
const STATUS_NO_VERSION_DATA: &str = "no version data";

/// Extracts what SKSE reported about itself and its plugins from the text of
/// a `skse64.log`.
pub fn parse(text: &str) -> ExtenderReport {
    let mut report = ExtenderReport {
        extender_version: None,
        game_version: None,
        plugin_dir: GamePath::new(EXTENDER_PLUGIN_DIR),
        plugins: Vec::new(),
    };

    for line in text.lines() {
        let line = line.trim_end();
        if let Some(header) = line.strip_prefix(HEADER_PREFIX) {
            let (extender_version, game_version) = parse_header(header);
            report.extender_version = extender_version;
            report.game_version = game_version;
        } else if let Some(plugin) = parse_plugin_line(line) {
            report.plugins.push(plugin);
        }
    }

    report
}

/// Parses the part of the header after `version = `, for example
/// `2.3.1 01070680 01DD748122C36157, os = 6.2 (9200))`.
fn parse_header(header: &str) -> (Option<String>, Option<String>) {
    let versions = header.split(',').next().unwrap_or(header);
    let mut tokens = versions.split_whitespace();
    let extender_version = tokens.next().map(str::to_owned);
    let game_version = tokens.next().and_then(decode_runtime_version);
    (extender_version, game_version)
}

/// Decodes the game version SKSE logs as eight hex digits.
///
/// SKSE packs a version as `major << 24 | minor << 16 | build << 4 | sub`,
/// so `01070680` is version 1.7.104.
fn decode_runtime_version(hex: &str) -> Option<String> {
    if hex.len() != 8 {
        return None;
    }
    let packed = u32::from_str_radix(hex, 16).ok()?;
    let major = packed >> 24;
    let minor = (packed >> 16) & 0xFF;
    let build = (packed >> 4) & 0xFFF;
    Some(format!("{major}.{minor}.{build}"))
}

/// Parses a line that reports the outcome of loading one plugin.
///
/// Returns `None` for every other line, including the many unrelated lines
/// that also start with `plugin `, such as `plugin 38 allocated 14 bytes
/// from branch pool`.
fn parse_plugin_line(line: &str) -> Option<ExtenderPlugin> {
    let rest = line.strip_prefix(PLUGIN_PREFIX)?;

    // The file name ends where the bracketed plugin info begins. Lower-casing
    // ASCII keeps every byte offset valid for `rest`.
    let name_end = rest.to_ascii_lowercase().find(".dll (")? + ".dll".len();
    let logged_name = rest.get(..name_end)?;
    let file_name = logged_name.rsplit(['\\', '/']).next()?.trim();
    if file_name.is_empty() {
        return None;
    }

    let after_name = rest.get(name_end..)?;
    let message = message_after_info(after_name)?;

    Some(ExtenderPlugin {
        file_name: file_name.to_owned(),
        status: classify(message),
    })
}

/// Returns the status text that follows the plugin info block.
///
/// `after_name` looks like ` (00000001 Name 00000007) loaded correctly
/// (handle 3)`. The plugin's name is free text and may itself contain
/// brackets, so the block is taken to end at the first `) ` that directly
/// follows eight hex digits.
fn message_after_info(after_name: &str) -> Option<&str> {
    let info_end = after_name.match_indices(") ").find_map(|(index, _)| {
        let digits = after_name.get(index.checked_sub(8)?..index)?;
        digits
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
            .then_some(index)
    })?;
    let message = after_name.get(info_end + ") ".len()..)?;

    // Drop the trailing `(handle N)` and the numeric code SKSE prints
    // before it for plugins that were not loaded.
    let message = message
        .rfind(HANDLE_MARKER)
        .and_then(|index| message.get(..index))
        .unwrap_or(message)
        .trim();
    let message = match message.rsplit_once(' ') {
        Some((text, code)) if !code.is_empty() && code.bytes().all(|b| b.is_ascii_digit()) => {
            text.trim_end()
        }
        _ => message,
    };

    (!message.is_empty()).then_some(message)
}

fn classify(message: &str) -> ExtenderPluginStatus {
    if message == STATUS_LOADED {
        ExtenderPluginStatus::Loaded
    } else if message.starts_with(STATUS_INCOMPATIBLE) {
        ExtenderPluginStatus::IncompatibleGameVersion
    } else if message == STATUS_NO_VERSION_DATA {
        ExtenderPluginStatus::NoVersionData
    } else {
        ExtenderPluginStatus::NotLoaded {
            reason: message.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A shortened log in the shape SKSE 2.3.1 writes. Names and paths are
    /// made up.
    const SAMPLE: &str = "\
SKSE64 runtime: initialize (version = 2.3.1 01070680 01DD748122C36157, os = 6.2 (9200))
imagebase = 00007FF7B8E70000
plugin directory = C:\\Games\\Skyrim Special Edition\\Data\\SKSE\\Plugins\\
scanning plugin directory C:\\Games\\Skyrim Special Edition\\Data\\SKSE\\Plugins\\
checking plugin Fixes.dll
checking plugin helper.dll
checking plugin Old.dll
checking plugin Odd.dll
checking plugin Broken.dll
plugin Fixes.dll (00000001 Fixes 00000007) loaded correctly (handle 1)
plugin helper.dll (00000000  00000000) no version data 0 (handle 0)
plugin Old.dll (00000001 old 00000001) disabled, incompatible with current version of the game 0 (handle 0)
plugin Odd.dll (00000001 Odd (NG) Plugin 00000001) loaded correctly (handle 2)
plugin Broken.dll (00000001 Broken 00000002) disabled, fatal error occurred while loading plugin 0 (handle 0)
registering plugin listener for SKSE at 1 of 5
plugin 2 allocated 11 bytes from branch pool
plugin 1 allocated 1536 bytes from local pool
";

    fn status_of<'a>(report: &'a ExtenderReport, file_name: &str) -> &'a ExtenderPluginStatus {
        &report
            .plugins
            .iter()
            .find(|plugin| plugin.file_name == file_name)
            .unwrap_or_else(|| panic!("{file_name} missing from report"))
            .status
    }

    #[test]
    fn reads_versions_from_the_header() {
        let report = parse(SAMPLE);
        assert_eq!(report.extender_version.as_deref(), Some("2.3.1"));
        assert_eq!(report.game_version.as_deref(), Some("1.7.104"));
    }

    #[test]
    fn decodes_packed_game_versions() {
        assert_eq!(
            decode_runtime_version("01070680").as_deref(),
            Some("1.7.104")
        );
        assert_eq!(
            decode_runtime_version("01050610").as_deref(),
            Some("1.5.97")
        );
        assert_eq!(decode_runtime_version("0106").as_deref(), None);
        assert_eq!(decode_runtime_version("zzzzzzzz").as_deref(), None);
    }

    #[test]
    fn finds_exactly_the_plugin_result_lines() {
        let report = parse(SAMPLE);
        let names: Vec<_> = report
            .plugins
            .iter()
            .map(|plugin| plugin.file_name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Fixes.dll",
                "helper.dll",
                "Old.dll",
                "Odd.dll",
                "Broken.dll"
            ]
        );
    }

    #[test]
    fn classifies_each_status() {
        let report = parse(SAMPLE);
        assert_eq!(
            status_of(&report, "Fixes.dll"),
            &ExtenderPluginStatus::Loaded
        );
        assert_eq!(
            status_of(&report, "helper.dll"),
            &ExtenderPluginStatus::NoVersionData
        );
        assert_eq!(
            status_of(&report, "Old.dll"),
            &ExtenderPluginStatus::IncompatibleGameVersion
        );
        assert_eq!(
            status_of(&report, "Broken.dll"),
            &ExtenderPluginStatus::NotLoaded {
                reason: "disabled, fatal error occurred while loading plugin".to_owned()
            }
        );
    }

    #[test]
    fn plugin_names_may_contain_brackets() {
        let report = parse(SAMPLE);
        assert_eq!(status_of(&report, "Odd.dll"), &ExtenderPluginStatus::Loaded);
    }

    #[test]
    fn a_logged_directory_is_stripped_from_the_file_name() {
        let report = parse(
            r"plugin C:\Games\Data\SKSE\Plugins\Deep.dll (00000001 Deep 00000001) loaded correctly (handle 4)",
        );
        assert_eq!(
            status_of(&report, "Deep.dll"),
            &ExtenderPluginStatus::Loaded
        );
    }

    #[test]
    fn windows_line_endings_are_handled() {
        let report = parse(&SAMPLE.replace('\n', "\r\n"));
        assert_eq!(report.plugins.len(), 5);
        assert_eq!(
            status_of(&report, "Fixes.dll"),
            &ExtenderPluginStatus::Loaded
        );
    }

    #[test]
    fn an_empty_log_gives_an_empty_report() {
        let report = parse("");
        assert_eq!(report.extender_version, None);
        assert_eq!(report.game_version, None);
        assert_eq!(report.plugins, Vec::new());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_text(text in ".*") {
            let _ = parse(&text);
        }

        #[test]
        fn never_panics_on_damaged_plugin_lines(
            name in ".{0,40}",
            info in ".{0,40}",
            tail in ".{0,60}",
        ) {
            let _ = parse(&format!("plugin {name}.dll ({info}) {tail}"));
            let _ = parse(&format!("plugin {name}.dll (00000001 {info} 00000001) {tail} (handle 1)"));
        }
    }
}
