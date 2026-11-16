//! Reads string tables (`.strings`), where localized plugins keep their
//! names.
//!
//! A plugin flagged as localized does not write a name into its records
//! but a number. The text belongs to a table next to it,
//! `strings/<plugin>_<language>.strings`:
//!
//! ```text
//! u32 count             how many strings
//! u32 size              how many bytes of text follow the directory
//! count × (u32 id, u32 offset)   offset into the text
//! text                  zero-terminated strings
//! ```
//!
//! The tables for descriptions (`.dlstrings`, `.ilstrings`) prefix each
//! text with its length instead; they are not needed for names and not
//! read.

use std::collections::HashMap;

use crate::Bytes;

/// The strings of a `.strings` table, by their number. Entries that do not
/// fit into the table are left out; a table that is not one yields an empty
/// map.
pub fn parse_strings(data: &[u8]) -> HashMap<u32, String> {
    let mut bytes = Bytes::new(data);
    let (Some(count), Some(size)) = (bytes.u32(), bytes.u32()) else {
        return HashMap::new();
    };
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    let size = usize::try_from(size).unwrap_or(usize::MAX);
    // Each entry of the directory takes eight bytes.
    let Some(directory_len) = count.checked_mul(8).filter(|len| *len <= bytes.remaining()) else {
        return HashMap::new();
    };
    let Some(directory) = bytes.take(directory_len) else {
        return HashMap::new();
    };
    let text = bytes.take(size.min(bytes.remaining())).unwrap_or_default();

    let mut strings = HashMap::with_capacity(count);
    for entry in directory.as_chunks::<8>().0 {
        let (id, offset) = entry.split_at(4);
        let (Ok(id), Ok(offset)) = (<[u8; 4]>::try_from(id), <[u8; 4]>::try_from(offset)) else {
            continue;
        };
        let offset = usize::try_from(u32::from_le_bytes(offset)).unwrap_or(usize::MAX);
        let Some(rest) = text.get(offset..) else {
            continue;
        };
        strings.insert(
            u32::from_le_bytes(id),
            decode(crate::plugin::until_nul(rest)),
        );
    }
    strings
}

/// Text of a string table: UTF-8 where it is valid, else a Windows code
/// page read byte by byte.
fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => crate::decode_text(bytes),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A table of `(id, text)` pairs.
    fn table(entries: &[(u32, &[u8])]) -> Vec<u8> {
        let mut directory = Vec::new();
        let mut text = Vec::new();
        for (id, value) in entries {
            directory.extend(id.to_le_bytes());
            directory.extend(u32::try_from(text.len()).unwrap().to_le_bytes());
            text.extend(*value);
            text.push(0);
        }
        let mut data = u32::try_from(entries.len()).unwrap().to_le_bytes().to_vec();
        data.extend(u32::try_from(text.len()).unwrap().to_le_bytes());
        data.extend(directory);
        data.extend(text);
        data
    }

    #[test]
    fn strings_are_found_by_their_number() {
        let strings = parse_strings(&table(&[
            (7, b"Millbrook"),
            (0x1234, "Mühlbach".as_bytes()),
        ]));
        assert_eq!(strings.get(&7).map(String::as_str), Some("Millbrook"));
        assert_eq!(strings.get(&0x1234).map(String::as_str), Some("Mühlbach"));
        assert_eq!(strings.len(), 2);
    }

    #[test]
    fn text_in_a_windows_code_page_is_read_as_well() {
        let strings = parse_strings(&table(&[(1, b"M\xfchlbach"), (2, "Höhle".as_bytes())]));
        assert_eq!(strings.get(&1).map(String::as_str), Some("Mühlbach"));
        assert_eq!(strings.get(&2).map(String::as_str), Some("Höhle"));
    }

    #[test]
    fn an_entry_pointing_past_the_text_is_left_out() {
        let mut data = table(&[(1, b"A")]);
        // Point the only entry far beyond the text.
        data[12..16].copy_from_slice(&1000u32.to_le_bytes());
        assert!(parse_strings(&data).is_empty());
        assert!(parse_strings(b"short").is_empty());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = parse_strings(&data);
        }
    }
}
