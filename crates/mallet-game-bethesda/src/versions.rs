//! The versions a setup is made of, read from the files as they are now
//! (ADR 0079): the game's program, the script extender and the game
//! versions it brings a library for, the Address Library's files, and what
//! every DLL in the extender's plugin folder says about the game versions
//! it was built for.
//!
//! Only reads, and only the few parts of each file needed.

use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use mallet_core::GamePath;
use mallet_core::facts::versions::{
    ExtenderInstall, NativePlugin, PluginDeclaration, PluginVersionData, RuntimeVersion,
    VersionFacts,
};

use crate::game::Game;
use crate::pe::{PeFile, PeInfo, u32_at};

/// The size of the version data of SKSE plugins since SKSE 2.1
/// (`SKSEPluginVersionData`, version 1).
const VERSION_DATA_SIZE: usize = 848;

/// The flags of `versionIndependence` SKSE knows.
const INDEPENDENT_ADDRESS_LIBRARY: u32 = 1 << 0;
const INDEPENDENT_SIGNATURES: u32 = 1 << 1;
const INDEPENDENT_STRUCTS_POST_629: u32 = 1 << 2;
/// The flag of `versionIndependenceEx` SKSE knows.
const INDEPENDENT_NO_STRUCT_USE: u32 = 1 << 0;

/// Programs larger than this are not opened as programs.
const MAX_PROGRAM: u64 = 512 * 1024 * 1024;

/// The names under which a plugin of `game`'s script extender gives its
/// version data and answers the older question, for the games whose
/// version data Mod Mallet knows: so far SKSE for Skyrim Special Edition.
const fn exports(game: Game) -> Option<(&'static str, &'static str)> {
    match game {
        Game::SkyrimSe => Some(("SKSEPlugin_Version", "SKSEPlugin_Query")),
        _ => None,
    }
}

/// Reads the versions of `game`, installed in `install` (the folder of its
/// program). `plugin_dirs` are the folders the script extender's plugins
/// lie in as the game sees them: the one below the installation and, for a
/// mod manager that lays folders over the data directory, the same folder
/// in each of those.
pub fn read(game: Game, install: &Path, plugin_dirs: &[PathBuf]) -> VersionFacts {
    VersionFacts {
        game: game_version(game, install),
        extender: extender(game, install),
        address_library: game
            .address_library()
            .map(|library| address_library(library.extension, plugin_dirs)),
        native_plugins: native_plugins(game, plugin_dirs),
        changes: None,
        newer_versions: std::collections::BTreeMap::new(),
    }
}

/// The game's version, from the version information of its program.
pub fn game_version(game: Game, install: &Path) -> Option<RuntimeVersion> {
    let info = program_info(&install.join(game.executable()))?;
    // A version whose parts do not fit the script extender's packing
    // (New Vegas calls itself 1.4.0.525) is not one to compare this way.
    RuntimeVersion::parse(&crate::pe::dotted(
        info.product_version.or(info.file_version)?,
    ))
}

/// The script extender, if its loader is there.
pub fn extender(game: Game, install: &Path) -> Option<ExtenderInstall> {
    let extender = game.script_extender();
    let loader = install.join(extender.loader);
    if !loader.is_file() {
        return None;
    }
    // The loader of SKSE 2.3.1 calls itself 0.2.3.1.
    let version = program_info(&loader)
        .and_then(|info| info.file_version)
        .map(|parts| match parts {
            [0, major, minor, build] | [major, minor, build, _] => {
                RuntimeVersion::new(u32::from(major), u32::from(minor), u32::from(build), 0)
            }
        });
    let runtimes = extender.versioned_library.map_or_default(|prefix| {
        // The libraries lie beside the loader.
        let mut runtimes: Vec<RuntimeVersion> = file_names(loader.parent().unwrap_or(install))
            .iter()
            .filter_map(|name| runtime_of_library(prefix, name))
            .collect();
        runtimes.sort_unstable();
        runtimes.dedup();
        runtimes
    });
    Some(ExtenderInstall {
        name: extender.short_name.to_owned(),
        version,
        runtimes,
        versioned: extender.versioned_library.is_some(),
    })
}

/// The game version the extender library `name` is for, if it is one:
/// `skse64_1_7_104.dll` is for 1.7.104. The loader's helper
/// (`skse64_steam_loader.dll`) is not.
fn runtime_of_library(prefix: &str, name: &str) -> Option<RuntimeVersion> {
    let lower = name.to_ascii_lowercase();
    let rest = lower.strip_prefix(prefix)?.strip_prefix('_')?;
    let rest = rest.strip_suffix(".dll")?;
    RuntimeVersion::parse(&rest.replace('_', "."))
}

/// The game versions the Address Library has a file for, in any of
/// `dirs`: `versionlib-1-7-104-0.bin` is for 1.7.104.
fn address_library(extension: &str, dirs: &[PathBuf]) -> Vec<RuntimeVersion> {
    let mut versions: Vec<RuntimeVersion> = dirs
        .iter()
        .flat_map(|dir| file_names(dir))
        .filter_map(|name| runtime_of_database(extension, &name))
        .collect();
    versions.sort_unstable();
    versions.dedup();
    versions
}

/// The game version the Address Library file `name` is for, if it is
/// one. Files for Skyrim 1.5 are called `version-…`, those for 1.6 and
/// later `versionlib-…`; some carry a further number after the version
/// (`versionlib-1-6-1170-0-1.bin`).
fn runtime_of_database(extension: &str, name: &str) -> Option<RuntimeVersion> {
    let lower = name.to_ascii_lowercase();
    let stem = lower.strip_suffix(&format!(".{extension}"))?;
    let rest = stem
        .strip_prefix("versionlib-")
        .or_else(|| stem.strip_prefix("version-"))?;
    let parts: Vec<&str> = rest.split('-').collect();
    let version = parts.get(..4).unwrap_or(&parts).join(".");
    RuntimeVersion::parse(&version)
}

/// Every DLL in `dirs`, the first of a name winning, sorted by name.
fn native_plugins(game: Game, dirs: &[PathBuf]) -> Vec<NativePlugin> {
    let plugin_dir = GamePath::new(game.script_extender().plugin_dir);
    let mut found: Vec<NativePlugin> = Vec::new();
    for dir in dirs {
        for name in file_names(dir) {
            let is_dll = Path::new(&name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"));
            if !is_dll
                || found
                    .iter()
                    .any(|known| known.file_name.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            found.push(native_plugin(game, &dir.join(&name), &plugin_dir, &name));
        }
    }
    found.sort_by_key(|plugin| plugin.file_name.to_lowercase());
    found
}

/// What the DLL at `path` says about itself.
fn native_plugin(game: Game, path: &Path, plugin_dir: &GamePath, name: &str) -> NativePlugin {
    let opened = File::open(path)
        .ok()
        .and_then(|file| PeFile::open(BufReader::new(file)));
    let (info, declaration) = match opened {
        Some(mut file) => {
            let info = file.info();
            let declaration = declaration(game, &mut file, &info);
            (info, declaration)
        }
        None => (PeInfo::default(), PluginDeclaration::Unreadable),
    };
    NativePlugin {
        path: plugin_dir.join(name),
        file_name: name.to_owned(),
        file_version: info.file_version_text(),
        product_name: info.product_name,
        declaration,
    }
}

/// What a DLL of `game`'s plugin folder says about itself to the script
/// extender.
fn declaration<R: std::io::Read + std::io::Seek>(
    game: Game,
    file: &mut PeFile<R>,
    info: &PeInfo,
) -> PluginDeclaration {
    let Some((version_export, query_export)) = exports(game) else {
        return PluginDeclaration::NotRead;
    };
    if info.exports.iter().any(|name| name == version_export) {
        return file
            .export_rva(version_export)
            .and_then(|rva| file.read_rva(rva, VERSION_DATA_SIZE))
            .and_then(|bytes| version_data(&bytes))
            .map_or(
                PluginDeclaration::Unreadable,
                PluginDeclaration::VersionData,
            );
    }
    if info.exports.iter().any(|name| name == query_export) {
        PluginDeclaration::QueryOnly
    } else {
        PluginDeclaration::Library
    }
}

/// Reads SKSE's plugin version data (`SKSEPluginVersionData`), or `None`
/// if `bytes` do not hold it in the one version SKSE reads.
pub fn version_data(bytes: &[u8]) -> Option<PluginVersionData> {
    if u32_at(bytes, 0)? != 1 {
        return None;
    }
    let text = |start: usize, len: usize| -> Option<String> {
        let field = bytes.get(start..start + len)?;
        let end = field.iter().position(|&byte| byte == 0).unwrap_or(len);
        Some(String::from_utf8_lossy(field.get(..end)?).trim().to_owned())
    };
    let independence_ex = u32_at(bytes, 772)?;
    let independence = u32_at(bytes, 776)?;
    let known = INDEPENDENT_ADDRESS_LIBRARY | INDEPENDENT_SIGNATURES | INDEPENDENT_STRUCTS_POST_629;
    let compatible = (0..16)
        .map_while(|index| u32_at(bytes, 780 + index * 4).filter(|&packed| packed != 0))
        .map(RuntimeVersion)
        .collect();
    let required = u32_at(bytes, 844)?;
    Some(PluginVersionData {
        plugin_version: u32_at(bytes, 4)?,
        name: text(8, 256)?,
        author: text(264, 256)?,
        address_library: independence & INDEPENDENT_ADDRESS_LIBRARY != 0,
        signatures: independence & INDEPENDENT_SIGNATURES != 0,
        structs_post_629: independence & INDEPENDENT_STRUCTS_POST_629 != 0,
        no_struct_use: independence_ex & INDEPENDENT_NO_STRUCT_USE != 0,
        unknown_independence: independence & !known != 0,
        compatible,
        extender_required: (required != 0).then_some(RuntimeVersion(required)),
    })
}

/// The version information of a program, read from its file.
fn program_info(path: &Path) -> Option<PeInfo> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_PROGRAM {
        return None;
    }
    let mut pe = PeFile::open(BufReader::new(file))?;
    let info = pe.info();
    Some(info)
}

/// Reads the plugin version data from the bytes of a whole DLL, for tests
/// and tools that already hold it.
pub fn declaration_of_bytes(game: Game, bytes: Vec<u8>) -> PluginDeclaration {
    match PeFile::open(Cursor::new(bytes)) {
        Some(mut file) => {
            let info = file.info();
            declaration(game, &mut file, &info)
        }
        None => PluginDeclaration::Unreadable,
    }
}

/// The names of the files in `dir`; none if it cannot be listed.
fn file_names(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect()
        },
    )
}

pub mod fixture {
    //! Builds made-up SKSE plugin version data for tests (ADR 0017).

    /// The bytes of `SKSEPluginVersionData` for a plugin called `name`
    /// (up to 255 bytes), with the flags of `versionIndependence` and
    /// `versionIndependenceEx`, up to 16 packed game versions it was built
    /// for, and the packed oldest script extender it accepts (0: any).
    pub fn version_data(
        name: &str,
        independence: u32,
        independence_ex: u32,
        compatible: &[u32],
        required: u32,
    ) -> Vec<u8> {
        let mut bytes = vec![0_u8; super::VERSION_DATA_SIZE];
        set(&mut bytes, 0, &1_u32.to_le_bytes());
        set(&mut bytes, 4, &0x0102_0030_u32.to_le_bytes());
        set(
            &mut bytes,
            8,
            name.as_bytes().get(..255).unwrap_or(name.as_bytes()),
        );
        set(&mut bytes, 264, b"Author");
        set(&mut bytes, 772, &independence_ex.to_le_bytes());
        set(&mut bytes, 776, &independence.to_le_bytes());
        for (index, version) in compatible.iter().take(16).enumerate() {
            set(&mut bytes, 780 + index * 4, &version.to_le_bytes());
        }
        set(&mut bytes, 844, &required.to_le_bytes());
        bytes
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
    use proptest::prelude::*;

    use super::fixture::version_data as data;
    use super::*;
    use crate::pe::fixture::{Export, dll};

    const V1_6_1170: u32 = 0x0106_4920;
    const V1_7_104: u32 = 0x0107_0680;

    #[test]
    fn reads_a_plugin_built_for_one_game_version() {
        let read = version_data(&data("skee", 0, 0, &[V1_6_1170], 0)).unwrap();
        assert_eq!(read.name, "skee");
        assert_eq!(read.author, "Author");
        assert_eq!(read.compatible, [RuntimeVersion(V1_6_1170)]);
        assert!(!read.independent());
        assert_eq!(read.extender_required, None);
    }

    #[test]
    fn reads_a_plugin_that_finds_the_game_through_the_address_library() {
        let read = version_data(&data("Fixes", 0b101, 0, &[], 0x0203_0010)).unwrap();
        assert!(read.address_library);
        assert!(read.structs_post_629);
        assert!(!read.signatures);
        assert!(read.independent());
        assert_eq!(read.compatible, []);
        assert_eq!(
            read.extender_required.map(|version| version.to_string()),
            Some("2.3.1".to_owned())
        );
    }

    #[test]
    fn unknown_ways_of_independence_are_noted() {
        let read = version_data(&data("Odd", 1 << 7, 1, &[], 0)).unwrap();
        assert!(read.unknown_independence);
        assert!(read.no_struct_use);
    }

    #[test]
    fn data_of_another_layout_is_not_read() {
        let mut bytes = data("x", 0, 0, &[], 0);
        bytes[0] = 2;
        assert_eq!(version_data(&bytes), None);
        assert_eq!(version_data(&bytes[..100]), None);
    }

    #[test]
    fn a_dll_says_what_kind_of_plugin_it_is() {
        let declared = dll(
            &[Export {
                name: "SKSEPlugin_Version",
                data: data("skee", 0, 0, &[V1_6_1170, V1_7_104], 0),
            }],
            Some([0, 4, 19, 0]),
            "RaceMenu",
        );
        let PluginDeclaration::VersionData(read) = declaration_of_bytes(Game::SkyrimSe, declared)
        else {
            panic!("expected version data");
        };
        assert_eq!(
            read.compatible,
            [RuntimeVersion(V1_6_1170), RuntimeVersion(V1_7_104)]
        );

        let old = dll(
            &[Export {
                name: "SKSEPlugin_Query",
                data: vec![0xC3],
            }],
            None,
            "",
        );
        assert_eq!(
            declaration_of_bytes(Game::SkyrimSe, old),
            PluginDeclaration::QueryOnly
        );
        let helper = dll(&[], None, "");
        assert_eq!(
            declaration_of_bytes(Game::SkyrimSe, helper.clone()),
            PluginDeclaration::Library
        );
        assert_eq!(
            declaration_of_bytes(Game::Fallout4, helper),
            PluginDeclaration::NotRead
        );
        assert_eq!(
            declaration_of_bytes(Game::SkyrimSe, b"MZ".to_vec()),
            PluginDeclaration::Unreadable
        );
    }

    #[test]
    fn names_of_libraries_and_databases_give_their_game_versions() {
        let parse = |text| RuntimeVersion::parse(text);
        assert_eq!(
            runtime_of_library("skse64", "skse64_1_7_104.dll"),
            parse("1.7.104")
        );
        assert_eq!(
            runtime_of_library("skse64", "SKSE64_1_5_97.DLL"),
            parse("1.5.97")
        );
        assert_eq!(
            runtime_of_library("skse64", "skse64_steam_loader.dll"),
            None
        );
        assert_eq!(runtime_of_library("skse64", "skse64_loader.exe"), None);
        assert_eq!(
            runtime_of_database("bin", "versionlib-1-7-104-0.bin"),
            parse("1.7.104")
        );
        assert_eq!(
            runtime_of_database("bin", "versionlib-1-6-1170-0-1.bin"),
            parse("1.6.1170")
        );
        assert_eq!(
            runtime_of_database("bin", "version-1-5-97-0.bin"),
            parse("1.5.97")
        );
        assert_eq!(runtime_of_database("bin", "version-1-4-15-0.csv"), None);
        assert_eq!(
            runtime_of_database("csv", "version-1-4-15-0.csv"),
            parse("1.4.15")
        );
    }

    #[test]
    fn reads_a_made_up_installation() {
        let dir = tempfile::tempdir().unwrap();
        let install = dir.path();
        let plugins = install.join("Data/SKSE/Plugins");
        std::fs::create_dir_all(&plugins).unwrap();
        std::fs::write(
            install.join("SkyrimSE.exe"),
            dll(&[], Some([1, 7, 104, 0]), "Skyrim"),
        )
        .unwrap();
        std::fs::write(
            install.join("skse64_loader.exe"),
            dll(&[], Some([0, 2, 3, 1]), "SKSE"),
        )
        .unwrap();
        for name in [
            "skse64_1_6_1170.dll",
            "skse64_1_7_104.dll",
            "skse64_steam_loader.dll",
        ] {
            std::fs::write(install.join(name), b"").unwrap();
        }
        std::fs::write(plugins.join("versionlib-1-6-1170-0.bin"), b"").unwrap();
        std::fs::write(
            plugins.join("skee64.dll"),
            dll(
                &[Export {
                    name: "SKSEPlugin_Version",
                    data: data("skee", 0, 0, &[V1_6_1170], 0),
                }],
                Some([0, 4, 19, 16]),
                "skee64",
            ),
        )
        .unwrap();
        std::fs::write(plugins.join("helper.dll"), dll(&[], None, "")).unwrap();
        std::fs::write(plugins.join("notes.txt"), b"").unwrap();

        let facts = read(Game::SkyrimSe, install, &[plugins]);
        assert_eq!(
            facts.game.map(|v| v.to_string()).as_deref(),
            Some("1.7.104")
        );
        let extender = facts.extender.unwrap();
        assert_eq!(
            extender.version.map(|v| v.to_string()).as_deref(),
            Some("2.3.1")
        );
        assert_eq!(
            extender.runtimes,
            [RuntimeVersion(V1_6_1170), RuntimeVersion(V1_7_104)]
        );
        assert_eq!(facts.address_library, Some(vec![RuntimeVersion(V1_6_1170)]));
        let names: Vec<_> = facts
            .native_plugins
            .iter()
            .map(|plugin| plugin.file_name.as_str())
            .collect();
        assert_eq!(names, ["helper.dll", "skee64.dll"]);
        let skee = &facts.native_plugins[1];
        assert_eq!(skee.path.as_str(), "data/skse/plugins/skee64.dll");
        assert_eq!(skee.file_version.as_deref(), Some("0.4.19.16"));
        assert!(matches!(
            skee.declaration,
            PluginDeclaration::VersionData(_)
        ));
        assert_eq!(
            facts.native_plugins[0].declaration,
            PluginDeclaration::Library
        );
    }

    #[test]
    fn a_game_without_its_files_has_no_versions() {
        let dir = tempfile::tempdir().unwrap();
        let facts = read(
            Game::SkyrimSe,
            dir.path(),
            &[dir.path().join("Data/SKSE/Plugins")],
        );
        assert_eq!(facts.game, None);
        assert_eq!(facts.extender, None);
        assert_eq!(facts.address_library, Some(Vec::new()));
        assert_eq!(facts.native_plugins, []);
    }

    proptest! {
        #[test]
        fn version_data_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
            let _ = version_data(&bytes);
        }

        #[test]
        fn version_data_with_a_valid_start_never_panics(
            rest in proptest::collection::vec(any::<u8>(), 0..900),
        ) {
            let mut bytes = 1_u32.to_le_bytes().to_vec();
            bytes.extend(rest);
            let _ = version_data(&bytes);
        }

        #[test]
        fn names_of_files_never_panic(name in ".{0,40}") {
            let _ = runtime_of_library("skse64", &name);
            let _ = runtime_of_database("bin", &name);
        }
    }
}
