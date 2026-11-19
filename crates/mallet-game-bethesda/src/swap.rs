//! Hiding placed objects with Base Object Swapper.
//!
//! Base Object Swapper is a script extender plugin that, while the game
//! runs, swaps the base object of a placed object for another one. It reads
//! its instructions from files named `*_SWAP.ini` in the data directory. A
//! line in the `[References]` section names one placed object, by its
//! number and the plugin that placed it, and what to show instead.
//!
//! Mod Mallet hides an object by swapping it for the game's invisible
//! marker. Nothing in any plugin changes; removing the file brings the
//! object back.

use std::collections::BTreeSet;
use std::fmt::Write;

use mallet_core::{ObjectRef, Patch, PatchPart};

/// The name of the one swap file of patches written before patches had
/// parts (ADR 0025).
pub const SWAP_FILE: &str = "ModMallet_SWAP.ini";

/// The plugin file of Base Object Swapper, below the data directory.
pub const SWAPPER_PLUGIN: &str = "SKSE/Plugins/po3_BaseObjectSwapper.dll";

/// The game's invisible marker (`XMarker`), which shows nothing and stops
/// nothing.
const INVISIBLE: &str = "0x3B~Skyrim.esm";

/// The text of a swap file that hides `objects`.
pub fn swap_ini(objects: &[ObjectRef]) -> String {
    let mut text = String::from(
        "; Written by Mod Mallet. Each line hides one placed object by showing an\n\
         ; invisible marker in its place while the game runs. Remove this mod to\n\
         ; bring the objects back.\n\
         [References]\n",
    );
    text.push_str(&lines(objects));
    text
}

/// The objects a swap file written by [`swap_ini`] hides: every line of
/// its `[References]` section that swaps an object for the invisible
/// marker. Lines that do anything else, and lines that cannot be read, are
/// left out.
pub fn hidden_objects(text: &str) -> Vec<ObjectRef> {
    let mut in_references = false;
    let mut found = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            in_references = line.eq_ignore_ascii_case("[References]");
            continue;
        }
        if !in_references {
            continue;
        }
        let Some((original, replacement)) = line.split_once('|') else {
            continue;
        };
        if !replacement.trim().eq_ignore_ascii_case(INVISIBLE) {
            continue;
        }
        let Some((number, plugin)) = original.trim().split_once('~') else {
            continue;
        };
        let digits = number
            .strip_prefix("0x")
            .or_else(|| number.strip_prefix("0X"))
            .unwrap_or(number);
        let Ok(local_id) = u32::from_str_radix(digits, 16) else {
            continue;
        };
        let plugin = plugin.trim();
        if plausible_plugin(plugin) {
            found.insert(ObjectRef {
                plugin: plugin.to_owned(),
                local_id,
            });
        }
    }
    found.into_iter().collect()
}

/// The comment key that marks a swap file as a part of a patch of Mod
/// Mallet, followed by the patch's name (ADR 0053).
const PATCH_KEY: &str = "mod-mallet-patch";

/// The text of the swap file of one part of `patch`. Its first lines say,
/// as comments, which patch and part it is, so that the file describes
/// itself wherever it is installed.
pub fn part_ini(patch: &Patch, part: &PatchPart) -> String {
    let mut text = String::from(
        "; Written by Mod Mallet. Each line hides one placed object by showing an\n\
         ; invisible marker in its place while the game runs. Remove the patch, or\n\
         ; leave this part out in its installer, to bring the objects back.\n",
    );
    let mut field = |key: &str, value: &str| {
        let _ = writeln!(text, "; {key}: {}", one_line(value));
    };
    field(PATCH_KEY, &patch.name);
    if let Some(author) = &patch.author {
        field("author", author);
    }
    if let Some(version) = &patch.version {
        field("version", version);
    }
    field("plugin", &part.plugin);
    if let Some(against) = &part.against {
        field("against", against);
    }
    text.push_str("[References]\n");
    text.push_str(&lines(&part.objects));
    text
}

/// Reads a swap file written by [`part_ini`]: the patch, with only this
/// part in it. `None` if the file is not a part of a patch of Mod Mallet.
pub fn read_part(text: &str) -> Option<Patch> {
    let mut fields: Vec<(&str, &str)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        let Some(comment) = line.strip_prefix(';') else {
            continue;
        };
        if let Some((key, value)) = comment.split_once(':') {
            fields.push((key.trim(), value.trim()));
        }
    }
    let value = |key: &str| {
        fields
            .iter()
            .find(|(found, _)| found.eq_ignore_ascii_case(key))
            .map(|(_, value)| (*value).to_owned())
            .filter(|value| !value.is_empty())
    };
    let name = value(PATCH_KEY)?;
    Some(Patch {
        name,
        author: value("author"),
        version: value("version"),
        parts: vec![PatchPart {
            plugin: value("plugin").unwrap_or_default(),
            against: value("against"),
            objects: hidden_objects(text),
        }],
    })
}

/// Whether a file of the data directory is one Base Object Swapper reads.
pub fn is_swap_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("_swap.ini") && !lower.contains(['/', '\\'])
}

/// The names of the swap files of `patch`'s parts, in the order of its
/// parts, each different from the others: the patch's and the plugins'
/// names, reduced to characters every file system and installer accepts.
pub fn part_file_names(patch: &Patch) -> Vec<String> {
    let mut taken = BTreeSet::new();
    patch
        .parts
        .iter()
        .map(|part| {
            let mut words = vec![file_word(&patch.name), file_word(stem(&part.plugin))];
            if let Some(against) = &part.against {
                words.push(file_word(stem(against)));
            }
            words.retain(|word| !word.is_empty());
            let base = words.join(" - ");
            let base = if base.is_empty() {
                "Mod Mallet".to_owned()
            } else {
                base
            };
            let mut name = format!("{base}_SWAP.ini");
            let mut number = 2;
            while !taken.insert(name.to_ascii_lowercase()) {
                name = format!("{base} ({number})_SWAP.ini");
                number += 1;
            }
            name
        })
        .collect()
}

/// The file name of the archive of the patch named `name`. Mod managers
/// name the installed mod after it.
pub fn archive_name(name: &str) -> String {
    let word = file_word(name);
    if word.is_empty() {
        format!("{}.zip", mallet_core::DEFAULT_PATCH_NAME)
    } else {
        format!("{word}.zip")
    }
}

/// A plugin's name without its extension.
fn stem(plugin: &str) -> &str {
    plugin.rsplit_once('.').map_or(plugin, |(stem, _)| stem)
}

/// `text` with only ASCII letters, digits and a few harmless signs,
/// shortened: zip archives and installers take such names everywhere.
fn file_word(text: &str) -> String {
    let kept: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric()
                || matches!(c, ' ' | '-' | '.' | '\'' | '(' | ')' | '&' | '+')
            {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = kept.split_whitespace().collect();
    words
        .join(" ")
        .chars()
        .take(48)
        .collect::<String>()
        .trim_matches(['.', ' '])
        .to_owned()
}

/// `text` on one line, for a comment.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The `[References]` lines that hide `objects`, each once.
fn lines(objects: &[ObjectRef]) -> String {
    let unique: BTreeSet<&ObjectRef> = objects
        .iter()
        .filter(|object| plausible_plugin(&object.plugin))
        .collect();
    let mut text = String::new();
    for object in unique {
        let _ = writeln!(
            text,
            "0x{:X}~{}|{INVISIBLE}",
            object.local_id, object.plugin
        );
    }
    text
}

/// Whether a plugin name can stand in a swap line: a plugin file name
/// without the characters the line uses as separators.
fn plausible_plugin(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".esp", ".esm", ".esl"]
        .iter()
        .any(|extension| lower.ends_with(extension))
        && !name.contains(['|', '~', '\n', '\r', '[', ']'])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(plugin: &str, local_id: u32) -> ObjectRef {
        ObjectRef {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    #[test]
    fn each_object_becomes_one_line_once() {
        let text = swap_ini(&[
            object("JK's Sky Haven Temple.esp", 0x0123),
            object("Village.esp", 0x0001_0F00),
            object("JK's Sky Haven Temple.esp", 0x0123),
        ]);
        let lines: Vec<&str> = text.lines().filter(|line| !line.starts_with(';')).collect();
        assert_eq!(
            lines,
            [
                "[References]",
                "0x123~JK's Sky Haven Temple.esp|0x3B~Skyrim.esm",
                "0x10F00~Village.esp|0x3B~Skyrim.esm",
            ]
        );
    }

    #[test]
    fn a_written_file_is_read_back() {
        let objects = vec![
            object("JK's Sky Haven Temple.esp", 0x0123),
            object("Village.esp", 0x0001_0F00),
        ];
        assert_eq!(hidden_objects(&swap_ini(&objects)), objects);
    }

    #[test]
    fn other_sections_and_other_swaps_are_not_hidden_objects() {
        let text = "[Forms]
0x1~A.esp|0x3B~Skyrim.esm
[References]
                    0x2~B.esp|0x10~Other.esp
nonsense
0xZZ~C.esp|0x3B~Skyrim.esm
                    0x3~D.esp|0x3B~Skyrim.esm
";
        assert_eq!(hidden_objects(text), vec![object("D.esp", 3)]);
    }

    proptest::proptest! {
        #[test]
        fn reading_never_panics(text in ".{0,400}") {
            let _ = hidden_objects(&text);
        }
    }

    #[test]
    fn names_that_would_break_a_line_are_left_out() {
        let text = swap_ini(&[object("Odd|Name.esp", 1), object("readme.txt", 2)]);
        assert!(!text.contains("Odd"));
        assert!(!text.contains("readme"));
    }

    fn patch(parts: Vec<PatchPart>) -> Patch {
        Patch {
            name: "Tidy Roads".to_owned(),
            author: Some("Someone".to_owned()),
            version: Some("1.2".to_owned()),
            parts,
        }
    }

    fn part(plugin: &str, against: Option<&str>, objects: Vec<ObjectRef>) -> PatchPart {
        PatchPart {
            plugin: plugin.to_owned(),
            against: against.map(str::to_owned),
            objects,
        }
    }

    #[test]
    fn a_part_describes_itself_and_is_read_back() {
        let road = part(
            "Roads.esp",
            Some("Town.esp"),
            vec![object("Roads.esp", 0x10), object("Skyrim.esm", 0x20)],
        );
        let text = part_ini(&patch(vec![road.clone()]), &road);
        assert!(text.contains("; mod-mallet-patch: Tidy Roads\n"));
        assert!(text.contains("0x10~Roads.esp|0x3B~Skyrim.esm\n"));
        assert_eq!(read_part(&text), Some(patch(vec![road])));
    }

    #[test]
    fn a_part_against_nothing_has_no_against() {
        let sign = part("Roads.esp", None, vec![object("Roads.esp", 0x11)]);
        let mut own = patch(vec![sign.clone()]);
        own.author = None;
        own.version = None;
        let read = read_part(&part_ini(&own, &sign)).unwrap();
        assert_eq!(read, own);
    }

    #[test]
    fn other_swap_files_are_no_parts() {
        assert_eq!(read_part(&swap_ini(&[object("A.esp", 1)])), None);
        assert_eq!(read_part("[References]\n0x1~A.esp|0x3B~Skyrim.esm\n"), None);
    }

    #[test]
    fn a_name_with_line_breaks_stays_in_its_comment() {
        let sign = part("Roads.esp", None, Vec::new());
        let mut odd = patch(vec![sign.clone()]);
        odd.name = "Two\nlines\r\n[References]\n0x1~A.esp|0x3B~Skyrim.esm".to_owned();
        let text = part_ini(&odd, &sign);
        assert_eq!(hidden_objects(&text), [] as [ObjectRef; 0]);
    }

    #[test]
    fn file_names_are_plain_and_different() {
        let names = part_file_names(&patch(vec![
            part("Roads.esp", Some("Town.esp"), Vec::new()),
            part("Roads.esp", None, Vec::new()),
            part("Roads.esp", None, Vec::new()),
            part("Bad/Name?.esp", Some("Town.esp"), Vec::new()),
        ]));
        assert_eq!(
            names,
            [
                "Tidy Roads - Roads - Town_SWAP.ini",
                "Tidy Roads - Roads_SWAP.ini",
                "Tidy Roads - Roads (2)_SWAP.ini",
                "Tidy Roads - Bad Name - Town_SWAP.ini",
            ]
        );
        assert!(names.iter().all(|name| is_swap_file(name)));
    }

    proptest::proptest! {
        #[test]
        fn reading_a_part_never_panics(text in ".{0,400}") {
            let _ = read_part(&text);
        }

        #[test]
        fn any_name_gives_a_swap_file_name(name in ".{0,80}", plugin in ".{0,80}") {
            let mut odd = patch(vec![part(&plugin, Some(&plugin), Vec::new())]);
            odd.name = name;
            for file in part_file_names(&odd) {
                proptest::prop_assert!(is_swap_file(&file));
                proptest::prop_assert!(!file.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|']));
            }
        }
    }
}
