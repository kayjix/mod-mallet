//! The INI files Mod Organizer writes: `ModOrganizer.ini`, each mod's
//! `meta.ini` and each profile's `settings.ini`.
//!
//! Mod Organizer writes them through Qt's settings class, which escapes
//! values in its own way:
//!
//! ```text
//! [General]
//! gameName=Skyrim Special Edition
//! gamePath=@ByteArray(C:\\Games\\Skyrim Special Edition)
//! selected_profile=@ByteArray(Default)
//!
//! [Settings]
//! mod_directory=%BASE_DIR%/mods
//! ```
//!
//! A backslash starts an escape (`\\`, `\"`, `\n`, `\x263a`), double quotes
//! enclose text that is taken as it is, and a value can be wrapped as
//! `@ByteArray(...)`. Only what Mod Mallet needs is understood; anything
//! else is kept as written.

use std::collections::HashMap;

/// The values of an INI file, by section and key. Section and key names are
/// compared without regard to case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ini {
    values: HashMap<(String, String), String>,
}

impl Ini {
    /// Reads the text of an INI file. Lines that are not understood are
    /// skipped; a later value for the same key replaces an earlier one.
    pub fn parse(text: &str) -> Self {
        let mut values = HashMap::new();
        let mut section = String::new();
        for line in text.trim_start_matches('\u{feff}').lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                section = name.trim().to_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            values.insert(
                (section.clone(), key.trim().to_lowercase()),
                unescape(value.trim()),
            );
        }
        Self { values }
    }

    /// The value of `key` in `section`, unescaped. `None` if it is not
    /// there.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.values
            .get(&(section.to_lowercase(), key.to_lowercase()))
            .map(String::as_str)
    }

    /// The value of `key` in `section`, if it is there and not blank.
    pub fn text(&self, section: &str, key: &str) -> Option<String> {
        self.get(section, key)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }

    /// The value of `key` in `section` as a yes or no. Anything but `true`
    /// (or `1`) is no.
    pub fn flag(&self, section: &str, key: &str) -> bool {
        self.get(section, key)
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true") || value.trim() == "1")
    }
}

/// A value as Qt wrote it, with its escapes resolved, its quotes taken off
/// and a `@ByteArray(...)` wrapper removed. Other `@` types, which hold
/// binary data, are read as nothing.
pub fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            // Quotes only keep Qt from reading a comma as the end of a list
            // item. Mod Mallet reads every value as one text.
            '"' => {}
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('0') => out.push('\0'),
                // Control characters have no place in a path or a name.
                Some('a' | 'b' | 'f' | 'v') | None => {}
                Some('x') => {
                    let mut code = 0u32;
                    let mut digits = 0;
                    while digits < 4 {
                        let Some(digit) = chars.peek().and_then(|next| next.to_digit(16)) else {
                            break;
                        };
                        code = code * 16 + digit;
                        digits += 1;
                        chars.next();
                    }
                    if let Some(decoded) = char::from_u32(code).filter(|_| digits > 0) {
                        out.push(decoded);
                    }
                }
                Some(other) => out.push(other),
            },
            other => out.push(other),
        }
    }
    if let Some(inner) = out
        .strip_prefix("@ByteArray(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        return inner.to_owned();
    }
    if out.starts_with('@') && !out.starts_with("@@") {
        return String::new();
    }
    // Qt doubles a leading `@` that is meant literally.
    if let Some(rest) = out.strip_prefix('@') {
        return rest.to_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn reads_values_by_section_and_key() {
        let ini = Ini::parse(
            "\u{feff}[General]\r\ngameName=Skyrim Special Edition\r\n; a comment\r\n\r\n[Settings]\r\nMod_Directory = %BASE_DIR%/mods\r\n",
        );
        assert_eq!(
            ini.get("general", "GAMENAME"),
            Some("Skyrim Special Edition")
        );
        assert_eq!(
            ini.get("Settings", "mod_directory"),
            Some("%BASE_DIR%/mods")
        );
        assert_eq!(ini.get("General", "mod_directory"), None);
    }

    #[test]
    fn byte_arrays_and_escaped_backslashes_become_plain_paths() {
        assert_eq!(
            unescape(r"@ByteArray(C:\\Games\\Skyrim Special Edition)"),
            r"C:\Games\Skyrim Special Edition"
        );
        assert_eq!(unescape("@ByteArray(Default)"), "Default");
    }

    #[test]
    fn quotes_and_other_escapes_are_resolved() {
        assert_eq!(unescape(r#""D:/Mods, Large""#), "D:/Mods, Large");
        assert_eq!(unescape(r#"say \"hi\""#), r#"say "hi""#);
        assert_eq!(unescape(r"caf\xe9"), "café");
        assert_eq!(unescape(r"a\tb"), "a\tb");
    }

    #[test]
    fn binary_values_are_read_as_nothing() {
        assert_eq!(unescape("@Variant(\\0\\0\\0\\x7f)"), "");
        assert_eq!(unescape("@Invalid()"), "");
        assert_eq!(unescape("@@literal"), "@literal");
    }

    #[test]
    fn flags_and_blank_texts() {
        let ini = Ini::parse("[General]\nLocalSaves=true\nLocalSettings=false\nempty=  \n");
        assert!(ini.flag("General", "LocalSaves"));
        assert!(!ini.flag("General", "LocalSettings"));
        assert!(!ini.flag("General", "missing"));
        assert_eq!(ini.text("General", "empty"), None);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(text in ".*") {
            let ini = Ini::parse(&text);
            let _ = ini.get("general", "gamePath");
            let _ = unescape(&text);
        }
    }
}
