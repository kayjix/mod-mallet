//! Readers for the file formats shared by Bethesda's games.
//!
//! Skyrim, Fallout 4 and their relatives store game data in the same two
//! containers: *plugins* (`.esp`, `.esm`, `.esl`), which hold records, and
//! *archives* (`.bsa`), which pack assets. This crate reads the structure of
//! both. What a record *means* differs per game and is left to the game
//! adapters.
//!
//! The readers extract only what Mod Mallet's analyses need, which keeps
//! them small and fast: [`bsa`] reads the list of file names and unpacks
//! single files on request, [`plugin`] skips every group of records that
//! was not asked for, and [`nif`] reads only the texture paths of a mesh.
//!
//! Every file is untrusted input (ADR 0013). The readers never panic, never
//! trust a size field without checking it against the data, and stop at the
//! first damaged spot while keeping what they read up to there.
//!
//! [`fixture`] builds small files in these formats for tests.

pub mod bsa;
pub mod fixture;
pub mod nif;
pub mod plugin;

/// A cursor over bytes that fails instead of reading past the end.
struct Bytes<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Bytes<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.position.checked_add(count)?;
        let slice = self.data.get(self.position..end)?;
        self.position = end;
        Some(slice)
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.take(count).map(|_| ())
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn signature(&mut self) -> Option<[u8; 4]> {
        self.take(4)?.try_into().ok()
    }
}

/// Decodes text stored in a Windows code page.
///
/// File names and editor names in these formats are almost always ASCII.
/// The rest is mapped byte by byte, which never fails and keeps distinct
/// names distinct.
fn decode_text(bytes: &[u8]) -> String {
    bytes.iter().map(|&byte| char::from(byte)).collect()
}
