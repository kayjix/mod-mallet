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

use mallet_core::ObjectRef;

/// The name of the file Base Object Swapper reads.
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
    let unique: BTreeSet<&ObjectRef> = objects
        .iter()
        .filter(|object| plausible_plugin(&object.plugin))
        .collect();
    for object in unique {
        let _ = writeln!(
            text,
            "0x{:X}~{}|{INVISIBLE}",
            object.local_id, object.plugin
        );
    }
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
}
