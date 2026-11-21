//! Reads the name and the functions of a compiled Papyrus script (`.pex`).
//!
//! Scripts hold the game logic of quests, objects and menus. Two mods that
//! ship the same script disagree about how that logic goes; which
//! functions differ says roughly where. The reader takes only the header,
//! the string table and the debug information, which the Papyrus compiler
//! writes unless told not to:
//!
//! ```text
//! magic 0xFA57C0DE (big-endian in Skyrim, little-endian from Fallout 4 on)
//! major and minor version (one byte each), game (u16), compile time (u64)
//! source file name, user name, machine name: u16 length and bytes each
//! string table: u16 count, then texts as above
//! debug information: one byte, non-zero if present, then
//!   modification time (u64), u16 count of functions, each:
//!   object, state and function name (u16 indices into the table),
//!   function type (u8: 0 plain, 1 property getter, 2 property setter),
//!   u16 count of instructions, then one u16 source line per instruction
//! user flags: u16 count, each a u16 name index and a u8 bit
//! objects: u16 count, each starting with its name (u16 index)
//! ```
//!
//! A script is untrusted input like any other (ADR 0013).

use crate::decode_text;

/// The most functions read from one script. Real scripts have a few
/// hundred at most.
const MAX_FUNCTIONS: usize = 10_000;

/// Why a script could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PexError {
    /// The file does not start like a compiled script.
    #[error("the file is not a compiled script")]
    NotAScript,
    /// The data ends too early, or a size or index in it is wrong.
    #[error("the compiled script is damaged or cut off")]
    Damaged,
}

/// What kind of function an entry of the debug information is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionKind {
    /// An ordinary function or event.
    Plain,
    /// What reads a property.
    Getter,
    /// What sets a property.
    Setter,
}

/// One function of a script, as its debug information describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexFunction {
    /// The state it belongs to; empty for the default state.
    pub state: String,
    /// Its name, or the property's for getters and setters.
    pub name: String,
    /// What kind of function it is.
    pub kind: FunctionKind,
    /// The source line of each instruction, counted from the function's
    /// first line, so that a function moved in the source compares equal.
    pub lines: Vec<u16>,
}

/// What a compiled script says about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PexScript {
    /// The script's name: its first object, or else its source file.
    pub name: String,
    /// The source file it was compiled from, as the compiler wrote it.
    pub source: String,
    /// Whether debug information is present; without it the functions are
    /// unknown.
    pub has_debug_info: bool,
    /// Its functions, in the order the debug information lists them.
    pub functions: Vec<PexFunction>,
}

/// A cursor that reads in the byte order of the script.
struct Reader<'a> {
    data: &'a [u8],
    position: usize,
    big_endian: bool,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.position.checked_add(count)?;
        let slice = self.data.get(self.position..end)?;
        self.position = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    fn u16(&mut self) -> Option<u16> {
        let bytes: [u8; 2] = self.take(2)?.try_into().ok()?;
        Some(if self.big_endian {
            u16::from_be_bytes(bytes)
        } else {
            u16::from_le_bytes(bytes)
        })
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.take(count).map(|_| ())
    }

    fn text(&mut self) -> Option<String> {
        let length = usize::from(self.u16()?);
        self.take(length).map(decode_text)
    }
}

/// Reads the name and functions of the compiled script in `data`.
pub fn read_script(data: &[u8]) -> Result<PexScript, PexError> {
    let big_endian = match data.get(..4) {
        Some([0xFA, 0x57, 0xC0, 0xDE]) => true,
        Some([0xDE, 0xC0, 0x57, 0xFA]) => false,
        _ => return Err(PexError::NotAScript),
    };
    let mut reader = Reader {
        data,
        position: 4,
        big_endian,
    };
    let damaged = || PexError::Damaged;
    // Version, game, compile time.
    reader.skip(1 + 1 + 2 + 8).ok_or_else(damaged)?;
    let source = reader.text().ok_or_else(damaged)?;
    reader.text().ok_or_else(damaged)?; // user
    reader.text().ok_or_else(damaged)?; // machine

    let count = usize::from(reader.u16().ok_or_else(damaged)?);
    let mut strings = Vec::with_capacity(count);
    for _ in 0..count {
        strings.push(reader.text().ok_or_else(damaged)?);
    }
    let lookup = |index: u16| strings.get(usize::from(index)).cloned();

    let has_debug_info = reader.u8().ok_or_else(damaged)? != 0;
    let mut functions = Vec::new();
    if has_debug_info {
        reader.skip(8).ok_or_else(damaged)?; // modification time
        let count = usize::from(reader.u16().ok_or_else(damaged)?);
        for _ in 0..count.min(MAX_FUNCTIONS) {
            reader.u16().ok_or_else(damaged)?; // object
            let state = reader.u16().ok_or_else(damaged)?;
            let name = reader.u16().ok_or_else(damaged)?;
            let kind = match reader.u8().ok_or_else(damaged)? {
                1 => FunctionKind::Getter,
                2 => FunctionKind::Setter,
                _ => FunctionKind::Plain,
            };
            let instructions = usize::from(reader.u16().ok_or_else(damaged)?);
            let mut lines = Vec::with_capacity(instructions);
            for _ in 0..instructions {
                lines.push(reader.u16().ok_or_else(damaged)?);
            }
            let first = lines.iter().copied().min().unwrap_or(0);
            for line in &mut lines {
                *line = line.saturating_sub(first);
            }
            functions.push(PexFunction {
                state: lookup(state).ok_or_else(damaged)?,
                name: lookup(name).ok_or_else(damaged)?,
                kind,
                lines,
            });
        }
    }

    // The first object's name, where the rest can be read; Fallout 4 puts
    // more debug information before it, so this is a best effort.
    let object = if has_debug_info && !big_endian {
        None
    } else {
        first_object(&mut reader, &strings)
    };
    let name = object.filter(|name| !name.is_empty()).unwrap_or_else(|| {
        let file = source.rsplit(['/', '\\']).next().unwrap_or(&source);
        file.rsplit_once('.')
            .map_or(file, |(stem, _)| stem)
            .to_owned()
    });
    Ok(PexScript {
        name,
        source,
        has_debug_info,
        functions,
    })
}

/// The name of the first object, read after the debug information.
fn first_object(reader: &mut Reader<'_>, strings: &[String]) -> Option<String> {
    let flags = usize::from(reader.u16()?);
    reader.skip(flags.checked_mul(3)?)?;
    if reader.u16()? == 0 {
        return None;
    }
    strings.get(usize::from(reader.u16()?)).cloned()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn text(out: &mut Vec<u8>, value: &str) {
        out.extend(u16::try_from(value.len()).unwrap().to_be_bytes());
        out.extend(value.as_bytes());
    }

    /// A Skyrim script named `Door` with two functions: `OnActivate`, and
    /// `Open` in the state `Locked` with the given source lines.
    fn script(debug: bool, open_lines: &[u16]) -> Vec<u8> {
        let mut out = vec![0xFA, 0x57, 0xC0, 0xDE, 3, 2, 0, 1];
        out.extend([0u8; 8]);
        text(&mut out, "Door.psc");
        text(&mut out, "someone");
        text(&mut out, "somewhere");
        let strings = ["Door", "", "OnActivate", "Locked", "Open"];
        out.extend(u16::try_from(strings.len()).unwrap().to_be_bytes());
        for value in strings {
            text(&mut out, value);
        }
        out.push(u8::from(debug));
        if debug {
            out.extend([0u8; 8]);
            out.extend(2u16.to_be_bytes());
            // OnActivate: object 0, state 1, name 2, plain, lines 10, 11.
            out.extend([0, 0, 0, 1, 0, 2, 0]);
            out.extend(2u16.to_be_bytes());
            out.extend(10u16.to_be_bytes());
            out.extend(11u16.to_be_bytes());
            // Open: object 0, state 3, name 4, plain.
            out.extend([0, 0, 0, 3, 0, 4, 0]);
            out.extend(u16::try_from(open_lines.len()).unwrap().to_be_bytes());
            for line in open_lines {
                out.extend(line.to_be_bytes());
            }
        }
        out.extend(0u16.to_be_bytes()); // user flags
        out.extend(1u16.to_be_bytes()); // one object
        out.extend(0u16.to_be_bytes()); // named Door
        out.extend(0u32.to_be_bytes());
        out
    }

    #[test]
    fn reads_the_name_and_the_functions() {
        let read = read_script(&script(true, &[40, 42, 41])).unwrap();
        assert_eq!(read.name, "Door");
        assert_eq!(read.source, "Door.psc");
        assert!(read.has_debug_info);
        assert_eq!(read.functions.len(), 2);
        assert_eq!(read.functions[0].name, "OnActivate");
        assert_eq!(read.functions[0].state, "");
        assert_eq!(read.functions[0].lines, vec![0, 1]);
        assert_eq!(read.functions[1].state, "Locked");
        assert_eq!(read.functions[1].lines, vec![0, 2, 1]);
    }

    #[test]
    fn a_function_moved_in_the_source_reads_the_same() {
        let here = read_script(&script(true, &[40, 41])).unwrap();
        let moved = read_script(&script(true, &[90, 91])).unwrap();
        assert_eq!(here.functions, moved.functions);
    }

    #[test]
    fn without_debug_information_only_the_name_is_known() {
        let read = read_script(&script(false, &[])).unwrap();
        assert_eq!(read.name, "Door");
        assert!(!read.has_debug_info);
        assert_eq!(read.functions, Vec::new());
    }

    #[test]
    fn other_files_are_not_scripts() {
        assert_eq!(read_script(b"DDS stuff"), Err(PexError::NotAScript));
        assert_eq!(read_script(&[]), Err(PexError::NotAScript));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..256)) {
            let _ = read_script(&data);
        }

        #[test]
        fn never_panics_on_a_cut_or_damaged_script(
            cut in 0usize..160,
            at in 0usize..160,
            value in any::<u8>(),
        ) {
            let mut data = script(true, &[5, 3, 9]);
            if let Some(byte) = data.get_mut(at) {
                *byte = value;
            }
            data.truncate(cut);
            let _ = read_script(&data);
        }
    }
}
