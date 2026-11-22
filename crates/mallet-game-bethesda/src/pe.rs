//! Reads what a Windows program or DLL says about itself: the version in
//! its version information, its product name, and the data it exports
//! under a name, which is how a script extender plugin tells the extender
//! which game versions it was built for (ADR 0079).
//!
//! Only the few parts needed are read, from wherever they lie in the file,
//! so that a large DLL is not read whole. A DLL is untrusted input like
//! any other file (ADR 0013): every offset is checked, a damaged file
//! gives `None` for what cannot be read, and nothing here panics.

use std::io::{Read, Seek, SeekFrom};

/// The most bytes read in one piece: more than any version information or
/// table of names this module looks at.
const MAX_PIECE: usize = 1 << 20;
/// The most names of exports looked at.
const MAX_EXPORTS: usize = 20_000;
/// The most sections looked at.
const MAX_SECTIONS: usize = 96;
/// How deep the blocks of version information are followed.
const MAX_DEPTH: u32 = 6;
/// The resource type of version information, `RT_VERSION`.
const RT_VERSION: u32 = 16;
/// The signature at the start of the fixed version information.
const FIXED_SIGNATURE: u32 = 0xFEEF_04BD;

/// What a program or DLL says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeInfo {
    /// The file version of its fixed version information, four numbers.
    pub file_version: Option<[u16; 4]>,
    /// The product version of its fixed version information.
    pub product_version: Option<[u16; 4]>,
    /// The product name of its version information.
    pub product_name: Option<String>,
    /// The names it exports.
    pub exports: Vec<String>,
}

impl PeInfo {
    /// The file version written with dots, `1.7.104.0`.
    pub fn file_version_text(&self) -> Option<String> {
        self.file_version.map(dotted)
    }
}

/// The four numbers of a version written with dots.
pub fn dotted(parts: [u16; 4]) -> String {
    let [a, b, c, d] = parts;
    format!("{a}.{b}.{c}.{d}")
}

/// A program or DLL opened for reading.
pub struct PeFile<R> {
    reader: R,
    sections: Vec<Section>,
    /// The RVA and size of the export directory.
    exports: Option<(u32, u32)>,
    /// The RVA and size of the resource directory.
    resources: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Copy)]
struct Section {
    virtual_address: u32,
    virtual_size: u32,
    raw_size: u32,
    raw_pointer: u32,
}

impl<R: Read + Seek> PeFile<R> {
    /// Opens the program or DLL `reader` reads, or `None` if it is not one.
    pub fn open(mut reader: R) -> Option<Self> {
        let dos = read_at(&mut reader, 0, 64)?;
        if dos.get(..2)? != b"MZ" {
            return None;
        }
        let pe_offset = u64::from(u32_at(&dos, 0x3C)?);
        let header = read_at(&mut reader, pe_offset, 24)?;
        if header.get(..4)? != b"PE\0\0" {
            return None;
        }
        let section_count = usize::from(u16_at(&header, 6)?).min(MAX_SECTIONS);
        let optional_size = u16_at(&header, 20)?;
        let optional = read_at(&mut reader, pe_offset + 24, usize::from(optional_size))?;
        let directories = match u16_at(&optional, 0)? {
            0x10B => 96,
            0x20B => 112,
            _ => return None,
        };
        let directory = |index: usize| -> Option<(u32, u32)> {
            let at = directories + index * 8;
            let rva = u32_at(&optional, at)?;
            let size = u32_at(&optional, at + 4)?;
            (rva != 0 && size != 0).then_some((rva, size))
        };
        let exports = directory(0);
        let resources = directory(2);
        let table = read_at(
            &mut reader,
            pe_offset + 24 + u64::from(optional_size),
            section_count * 40,
        )?;
        let sections = table
            .as_chunks::<40>()
            .0
            .iter()
            .filter_map(|entry| {
                Some(Section {
                    virtual_size: u32_at(entry, 8)?,
                    virtual_address: u32_at(entry, 12)?,
                    raw_size: u32_at(entry, 16)?,
                    raw_pointer: u32_at(entry, 20)?,
                })
            })
            .collect();
        Some(Self {
            reader,
            sections,
            exports,
            resources,
        })
    }

    /// Everything this module reads about the file.
    pub fn info(&mut self) -> PeInfo {
        let (file_version, product_version, product_name) = self.version_info();
        PeInfo {
            file_version,
            product_version,
            product_name,
            exports: self.export_names(),
        }
    }

    /// `len` bytes at the address `rva` of the loaded image, as far as the
    /// file holds them.
    pub fn read_rva(&mut self, rva: u32, len: usize) -> Option<Vec<u8>> {
        let offset = self.offset_of(rva)?;
        read_at(&mut self.reader, offset, len)
    }

    /// The address of the export named `name`, if the file exports it.
    pub fn export_rva(&mut self, name: &str) -> Option<u32> {
        let (names, ordinals, functions, count) = self.export_tables()?;
        (0..count).find_map(|index| {
            let name_rva = u32_at(&names, index * 4)?;
            if self.c_string(name_rva)? != name {
                return None;
            }
            let ordinal = usize::from(u16_at(&ordinals, index * 2)?);
            self.read_rva(functions.checked_add(u32::try_from(ordinal * 4).ok()?)?, 4)
                .and_then(|bytes| u32_at(&bytes, 0))
        })
    }

    /// The names the file exports.
    fn export_names(&mut self) -> Vec<String> {
        let Some((names, _, _, count)) = self.export_tables() else {
            return Vec::new();
        };
        (0..count)
            .filter_map(|index| {
                let rva = u32_at(&names, index * 4)?;
                self.c_string(rva)
            })
            .collect()
    }

    /// The table of name addresses, the table of ordinals, the address of
    /// the table of functions, and how many names there are.
    fn export_tables(&mut self) -> Option<(Vec<u8>, Vec<u8>, u32, usize)> {
        let (rva, _) = self.exports?;
        let directory = self.read_rva(rva, 40)?;
        let count = usize::try_from(u32_at(&directory, 24)?)
            .ok()?
            .min(MAX_EXPORTS);
        let functions = u32_at(&directory, 28)?;
        let names = self.read_rva(u32_at(&directory, 32)?, count * 4)?;
        let ordinals = self.read_rva(u32_at(&directory, 36)?, count * 2)?;
        Some((names, ordinals, functions, count))
    }

    /// The text ending in a zero byte at `rva`, if it is short and plain.
    fn c_string(&mut self, rva: u32) -> Option<String> {
        let offset = self.offset_of(rva)?;
        let bytes = read_up_to(&mut self.reader, offset, 256)?;
        let end = bytes.iter().position(|&byte| byte == 0)?;
        String::from_utf8(bytes.get(..end)?.to_vec()).ok()
    }

    /// The file offset of the address `rva`.
    fn offset_of(&self, rva: u32) -> Option<u64> {
        self.sections.iter().find_map(|section| {
            let start = section.virtual_address;
            let size = section.virtual_size.max(section.raw_size);
            let within = rva.checked_sub(start)?;
            (within < size && within < section.raw_size)
                .then(|| u64::from(section.raw_pointer) + u64::from(within))
        })
    }

    /// The file version, the product version and the product name, from
    /// the first version information among the file's resources.
    fn version_info(&mut self) -> (Option<[u16; 4]>, Option<[u16; 4]>, Option<String>) {
        let Some(bytes) = self.version_resource() else {
            return (None, None, None);
        };
        let (file_version, product_version) = fixed_versions(&bytes).unzip();
        let product_name = string_value(&bytes, "ProductName");
        (file_version, product_version, product_name)
    }

    /// The bytes of the version information resource.
    fn version_resource(&mut self) -> Option<Vec<u8>> {
        let (root_rva, _) = self.resources?;
        // Type, then name, then language: the first entry of each level
        // below the type of version information.
        let name_dir = self.resource_entry(root_rva, 0, Some(RT_VERSION))?;
        let language_dir = self.resource_entry(root_rva, name_dir, None)?;
        let data_entry = self.resource_entry(root_rva, language_dir, None)?;
        let entry = self.read_rva(root_rva.checked_add(data_entry)?, 16)?;
        let data_rva = u32_at(&entry, 0)?;
        let size = usize::try_from(u32_at(&entry, 4)?).ok()?.min(MAX_PIECE);
        self.read_rva(data_rva, size)
    }

    /// The offset, from the start of the resources, that the entry with
    /// the identifier `id` (or the first entry) of the directory at
    /// `offset` points to, without the flag that marks a directory.
    fn resource_entry(&mut self, root_rva: u32, offset: u32, id: Option<u32>) -> Option<u32> {
        let directory_rva = root_rva.checked_add(offset)?;
        let header = self.read_rva(directory_rva, 16)?;
        let count = usize::from(u16_at(&header, 12)?) + usize::from(u16_at(&header, 14)?);
        let entries = self.read_rva(directory_rva.checked_add(16)?, count.min(4096) * 8)?;
        entries.as_chunks::<8>().0.iter().find_map(|entry| {
            let name = u32_at(entry, 0)?;
            let target = u32_at(entry, 4)?;
            id.is_none_or(|id| name == id)
                .then_some(target & 0x7FFF_FFFF)
        })
    }
}

/// The file and product versions of the fixed version information in a
/// version resource.
fn fixed_versions(bytes: &[u8]) -> Option<([u16; 4], [u16; 4])> {
    let start = (0..bytes.len().saturating_sub(52))
        .step_by(4)
        .find(|&at| u32_at(bytes, at) == Some(FIXED_SIGNATURE))?;
    // Each version is two numbers of four bytes, the more significant
    // first, each with its more significant half second.
    let parts = |at: usize| -> Option<[u16; 4]> {
        let at = start + at;
        Some([
            u16_at(bytes, at + 2)?,
            u16_at(bytes, at)?,
            u16_at(bytes, at + 6)?,
            u16_at(bytes, at + 4)?,
        ])
    };
    Some((parts(8)?, parts(16)?))
}

/// The value of the string `key` in the string tables of a version
/// resource.
fn string_value(bytes: &[u8], key: &str) -> Option<String> {
    let mut found = None;
    walk_block(bytes, 0, bytes.len(), 0, &mut |name, value| {
        if found.is_none() && name == key {
            found = Some(value.to_owned());
        }
    });
    found.filter(|value| !value.trim().is_empty())
}

/// Calls `visit` with the key and text value of every block of version
/// information between `start` and `end`.
fn walk_block(
    bytes: &[u8],
    start: usize,
    end: usize,
    depth: u32,
    visit: &mut dyn FnMut(&str, &str),
) {
    if depth > MAX_DEPTH {
        return;
    }
    let mut at = start;
    while at + 6 <= end {
        let Some(length) = u16_at(bytes, at).map(usize::from) else {
            return;
        };
        if length < 6 {
            return;
        }
        let block_end = (at + length).min(end);
        let value_length = u16_at(bytes, at + 2).map_or(0, usize::from);
        let is_text = u16_at(bytes, at + 4) == Some(1);
        let (key, after_key) = utf16_until_zero(bytes, at + 6, block_end);
        let value_start = align4(after_key);
        let value_bytes = if is_text {
            value_length * 2
        } else {
            value_length
        };
        if is_text {
            let (value, _) = utf16_until_zero(bytes, value_start, block_end);
            visit(&key, &value);
        }
        let children = align4(value_start + value_bytes);
        if children < block_end {
            walk_block(bytes, children, block_end, depth + 1, visit);
        }
        at = align4(at + length);
    }
}

/// The UTF-16 text from `start` to its zero, and where the zero ends.
fn utf16_until_zero(bytes: &[u8], start: usize, end: usize) -> (String, usize) {
    let mut units = Vec::new();
    let mut at = start;
    while at + 2 <= end {
        let Some(unit) = u16_at(bytes, at) else {
            break;
        };
        at += 2;
        if unit == 0 {
            break;
        }
        units.push(unit);
    }
    (String::from_utf16_lossy(&units), at)
}

const fn align4(at: usize) -> usize {
    (at + 3) & !3
}

/// Exactly `len` bytes at `offset`, or `None`.
fn read_at<R: Read + Seek>(reader: &mut R, offset: u64, len: usize) -> Option<Vec<u8>> {
    if len > MAX_PIECE {
        return None;
    }
    reader.seek(SeekFrom::Start(offset)).ok()?;
    let mut buffer = vec![0; len];
    reader.read_exact(&mut buffer).ok()?;
    Some(buffer)
}

/// Up to `len` bytes at `offset`, fewer at the end of the file.
fn read_up_to<R: Read + Seek>(reader: &mut R, offset: u64, len: usize) -> Option<Vec<u8>> {
    reader.seek(SeekFrom::Start(offset)).ok()?;
    let mut buffer = Vec::with_capacity(len);
    reader
        .take(u64::try_from(len).ok()?)
        .read_to_end(&mut buffer)
        .ok()?;
    Some(buffer)
}

/// The little-endian number of two bytes at `at`.
pub(crate) fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    let piece = bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(piece.try_into().ok()?))
}

/// The little-endian number of four bytes at `at`.
pub(crate) fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let piece = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(piece.try_into().ok()?))
}

#[allow(clippy::cast_possible_truncation)]
pub mod fixture {
    //! Builds small made-up DLLs and programs for tests (ADR 0017): the
    //! exports and the version information passed in, and nothing else.

    /// Where the one section lies in the file and in memory.
    const SECTION_RAW: u32 = 0x200;
    const SECTION_RVA: u32 = 0x1000;

    /// One export: its name and the bytes it points at.
    pub struct Export<'a> {
        /// The name it is exported under.
        pub name: &'a str,
        /// The bytes the export points at.
        pub data: Vec<u8>,
    }

    /// A 64-bit DLL with the given exports and, if `version` is given, a
    /// version resource with that file version and `product` as its
    /// product name.
    pub fn dll(exports: &[Export<'_>], version: Option<[u16; 4]>, product: &str) -> Vec<u8> {
        // The section: the export directory and its tables, the export
        // data, the names, then the resources.
        let mut section = vec![0_u8; 40];
        let rva = |section: &Vec<u8>| SECTION_RVA + section.len() as u32;
        let mut data_rvas = Vec::new();
        for export in exports {
            align(&mut section, 8);
            data_rvas.push(rva(&section));
            section.extend_from_slice(&export.data);
        }
        let mut name_rvas = Vec::new();
        for export in exports {
            name_rvas.push(rva(&section));
            section.extend_from_slice(export.name.as_bytes());
            section.push(0);
        }
        align(&mut section, 4);
        let functions = rva(&section);
        for data in &data_rvas {
            section.extend_from_slice(&data.to_le_bytes());
        }
        let names = rva(&section);
        for name in &name_rvas {
            section.extend_from_slice(&name.to_le_bytes());
        }
        let ordinals = rva(&section);
        for index in 0..exports.len() {
            section.extend_from_slice(&(index as u16).to_le_bytes());
        }
        let count = exports.len() as u32;
        put(&mut section, 20, count);
        put(&mut section, 24, count);
        put(&mut section, 28, functions);
        put(&mut section, 32, names);
        put(&mut section, 36, ordinals);

        align(&mut section, 4);
        let resources = rva(&section);
        let resource_size = version.map_or(0, |version| {
            let start = section.len();
            write_resources(&mut section, resources, version, product);
            (section.len() - start) as u32
        });
        align(&mut section, 0x200);

        let mut file = vec![0_u8; SECTION_RAW as usize];
        set(&mut file, 0, b"MZ");
        put(&mut file, 0x3C, 0x80);
        set(&mut file, 0x80, b"PE\0\0");
        put16(&mut file, 0x80 + 4, 0x8664);
        put16(&mut file, 0x80 + 6, 1);
        put16(&mut file, 0x80 + 20, 240);
        let optional = 0x80 + 24;
        put16(&mut file, optional, 0x20B);
        put(&mut file, optional + 108, 16);
        put(&mut file, optional + 112, SECTION_RVA);
        put(&mut file, optional + 116, 40);
        if version.is_some() {
            put(&mut file, optional + 112 + 16, resources);
            put(&mut file, optional + 112 + 20, resource_size);
        }
        let table = optional + 240;
        set(&mut file, table, b".data");
        put(&mut file, table + 8, section.len() as u32);
        put(&mut file, table + 12, SECTION_RVA);
        put(&mut file, table + 16, section.len() as u32);
        put(&mut file, table + 20, SECTION_RAW);
        file.extend_from_slice(&section);
        file
    }

    /// The resource directory with one version resource, at `base`.
    fn write_resources(section: &mut Vec<u8>, base: u32, version: [u16; 4], product: &str) {
        let directory = |section: &mut Vec<u8>, id: u32, target: u32| {
            section.extend_from_slice(&[0; 12]);
            section.extend_from_slice(&0_u16.to_le_bytes());
            section.extend_from_slice(&1_u16.to_le_bytes());
            section.extend_from_slice(&id.to_le_bytes());
            section.extend_from_slice(&target.to_le_bytes());
        };
        // Type 16, name 1, language 0x409; each directory 24 bytes.
        directory(section, 16, 0x8000_0000 | 0x18);
        directory(section, 1, 0x8000_0000 | 0x30);
        directory(section, 0x409, 72);
        let info = version_info(version, product);
        let data_rva = base + 72 + 16;
        section.extend_from_slice(&data_rva.to_le_bytes());
        section.extend_from_slice(&(info.len() as u32).to_le_bytes());
        section.extend_from_slice(&[0; 8]);
        section.extend_from_slice(&info);
    }

    /// A `VS_VERSIONINFO` block with a string table holding the product
    /// name.
    fn version_info(version: [u16; 4], product: &str) -> Vec<u8> {
        let [major, minor, build, sub] = version.map(u32::from);
        let mut fixed = Vec::new();
        for value in [
            0xFEEF_04BD_u32,
            0x0001_0000,
            (major << 16) | minor,
            (build << 16) | sub,
            (major << 16) | minor,
            (build << 16) | sub,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ] {
            fixed.extend_from_slice(&value.to_le_bytes());
        }
        let name = block("ProductName", Some(product), &[], &[]);
        let table = block("040904b0", None, &[], &[name]);
        let strings = block("StringFileInfo", None, &[], &[table]);
        block("VS_VERSION_INFO", None, &fixed, &[strings])
    }

    /// One block of version information: key, a text or binary value,
    /// children.
    fn block(key: &str, text: Option<&str>, binary: &[u8], children: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0_u8; 6];
        for unit in key.encode_utf16().chain([0]) {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        align(&mut out, 4);
        let (value_length, is_text) = if let Some(text) = text {
            let units: Vec<u16> = text.encode_utf16().chain([0]).collect();
            for unit in &units {
                out.extend_from_slice(&unit.to_le_bytes());
            }
            (units.len(), 1_u16)
        } else {
            out.extend_from_slice(binary);
            (binary.len(), 0)
        };
        align(&mut out, 4);
        for child in children {
            out.extend_from_slice(child);
            align(&mut out, 4);
        }
        let length = out.len() as u16;
        set(&mut out, 0, &length.to_le_bytes());
        set(&mut out, 2, &(value_length as u16).to_le_bytes());
        set(&mut out, 4, &is_text.to_le_bytes());
        out
    }

    fn align(bytes: &mut Vec<u8>, to: usize) {
        while !bytes.len().is_multiple_of(to) {
            bytes.push(0);
        }
    }

    fn put(bytes: &mut [u8], at: usize, value: u32) {
        set(bytes, at, &value.to_le_bytes());
    }

    fn put16(bytes: &mut [u8], at: usize, value: u16) {
        set(bytes, at, &value.to_le_bytes());
    }

    /// Writes `value` at `at`, where the buffer has room for it.
    fn set(bytes: &mut [u8], at: usize, value: &[u8]) {
        if let Some(slot) = bytes.get_mut(at..at + value.len()) {
            slot.copy_from_slice(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use proptest::prelude::*;

    use super::fixture::{Export, dll};
    use super::*;

    #[test]
    fn reads_the_version_product_and_exports_of_a_made_up_dll() {
        let bytes = dll(
            &[
                Export {
                    name: "SKSEPlugin_Load",
                    data: vec![0xC3],
                },
                Export {
                    name: "SKSEPlugin_Version",
                    data: vec![1, 2, 3, 4],
                },
            ],
            Some([1, 7, 104, 0]),
            "Example Plugin",
        );
        let mut file = PeFile::open(Cursor::new(bytes)).unwrap();
        let info = file.info();
        assert_eq!(info.file_version, Some([1, 7, 104, 0]));
        assert_eq!(info.file_version_text().as_deref(), Some("1.7.104.0"));
        assert_eq!(info.product_name.as_deref(), Some("Example Plugin"));
        assert_eq!(info.exports, ["SKSEPlugin_Load", "SKSEPlugin_Version"]);
        let rva = file.export_rva("SKSEPlugin_Version").unwrap();
        assert_eq!(file.read_rva(rva, 4), Some(vec![1, 2, 3, 4]));
        assert_eq!(file.export_rva("SKSEPlugin_Query"), None);
    }

    #[test]
    fn a_dll_without_version_information_has_none() {
        let bytes = dll(&[], None, "");
        let info = PeFile::open(Cursor::new(bytes)).unwrap().info();
        assert_eq!(info, PeInfo::default());
    }

    #[test]
    fn what_is_not_a_program_is_not_opened() {
        assert!(PeFile::open(Cursor::new(b"not a program".to_vec())).is_none());
        assert!(PeFile::open(Cursor::new(Vec::new())).is_none());
        let mut bytes = dll(&[], None, "");
        bytes[0x80] = b'X';
        assert!(PeFile::open(Cursor::new(bytes)).is_none());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..2048)) {
            if let Some(mut file) = PeFile::open(Cursor::new(bytes)) {
                let _ = file.info();
                let _ = file.export_rva("SKSEPlugin_Version");
            }
        }

        #[test]
        fn never_panics_on_a_damaged_dll(
            flips in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..24),
        ) {
            let mut bytes = dll(
                &[Export { name: "SKSEPlugin_Version", data: vec![7; 848] }],
                Some([1, 2, 3, 4]),
                "Name",
            );
            for (index, value) in flips {
                let at = index.index(bytes.len());
                bytes[at] = value;
            }
            if let Some(mut file) = PeFile::open(Cursor::new(bytes)) {
                let _ = file.info();
                if let Some(rva) = file.export_rva("SKSEPlugin_Version") {
                    let _ = file.read_rva(rva, 848);
                }
            }
        }
    }
}
