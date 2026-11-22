//! The versions a setup is made of: the game's program, its script
//! extender, the libraries most of the extender's plugins need, and what
//! each of those plugins says about the versions it was built for
//! (ADR 0079).
//!
//! These are read from the files as they are now, not from a log of the
//! last start, so that a game Steam updated is noticed before it is
//! started again.

use crate::GamePath;

/// A version of the game or of the script extender, packed as the script
/// extender packs them: `major << 24 | minor << 16 | build << 4 | sub`.
/// Packed this way, versions compare in their natural order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeVersion(pub u32);

impl RuntimeVersion {
    /// The version `major.minor.build.sub`. Parts too large for their
    /// place are cut to it.
    pub const fn new(major: u32, minor: u32, build: u32, sub: u32) -> Self {
        Self(((major & 0xFF) << 24) | ((minor & 0xFF) << 16) | ((build & 0xFFF) << 4) | (sub & 0xF))
    }

    /// Reads a version written with dots, such as `1.7.104` or
    /// `1.7.104.0`. `None` for anything else.
    pub fn parse(text: &str) -> Option<Self> {
        let mut parts = text.trim().split('.').map(|part| part.parse::<u32>().ok());
        let major = parts.next()??;
        let minor = parts.next()??;
        let build = parts.next()??;
        let sub = match parts.next() {
            Some(sub) => sub?,
            None => 0,
        };
        if parts.next().is_some() || major > 0xFF || minor > 0xFF || build > 0xFFF || sub > 0xF {
            return None;
        }
        Some(Self::new(major, minor, build, sub))
    }

    /// The first part, `1` of `1.7.104`.
    pub const fn major(self) -> u32 {
        self.0 >> 24
    }

    /// The second part, `7` of `1.7.104`.
    pub const fn minor(self) -> u32 {
        (self.0 >> 16) & 0xFF
    }

    /// The third part, `104` of `1.7.104`.
    pub const fn build(self) -> u32 {
        (self.0 >> 4) & 0xFFF
    }

    /// The fourth part, which is nearly always 0.
    pub const fn sub(self) -> u32 {
        self.0 & 0xF
    }
}

impl std::fmt::Display for RuntimeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.build())?;
        if self.sub() != 0 {
            write!(f, ".{}", self.sub())?;
        }
        Ok(())
    }
}

/// The versions of a setup, as its files tell them now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VersionFacts {
    /// The game's version, as its program tells it: the version Steam
    /// installed, whether or not the game has run since. `None` if the
    /// program could not be read.
    pub game: Option<RuntimeVersion>,
    /// The script extender, if it is installed.
    pub extender: Option<ExtenderInstall>,
    /// The game versions the Address Library has a file for, or `None` for
    /// a game without an Address Library. Empty if it is not installed.
    pub address_library: Option<Vec<RuntimeVersion>>,
    /// Every DLL in the script extender's plugin folder.
    pub native_plugins: Vec<NativePlugin>,
    /// What changed since the scan before this one, if that scan is known
    /// (ADR 0079).
    pub changes: Option<crate::versions::VersionChanges>,
    /// The newest version the mod manager knows of each mod that has a
    /// newer one, by the mod's identifier.
    pub newer_versions: std::collections::BTreeMap<String, String>,
}

/// The script extender as it is installed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtenderInstall {
    /// The short name people know it by, such as `SKSE`.
    pub name: String,
    /// Its own version, from its loader (`2.3.1`), if that tells it.
    pub version: Option<RuntimeVersion>,
    /// The game versions it brings a library for (`skse64_1_7_104.dll`),
    /// lowest first. Empty for an extender with one library for all.
    pub runtimes: Vec<RuntimeVersion>,
    /// Whether it ships one library per game version, so that
    /// [`ExtenderInstall::runtimes`] says which game versions it supports.
    pub versioned: bool,
}

impl ExtenderInstall {
    /// Whether it can run the game in `version`. An extender that does not
    /// say which versions it supports is taken to support any.
    pub fn supports(&self, version: RuntimeVersion) -> bool {
        !self.versioned || self.runtimes.contains(&version)
    }
}

/// One DLL in the script extender's plugin folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePlugin {
    /// Where it lies, `data/skse/plugins/x.dll`.
    pub path: GamePath,
    /// Its file name in the spelling on the disk.
    pub file_name: String,
    /// The version its version information gives (`7.0.21.0`), if any.
    pub file_version: Option<String>,
    /// The product name its version information gives, if any.
    pub product_name: Option<String>,
    /// What it says about itself to the script extender.
    pub declaration: PluginDeclaration,
}

/// What a DLL in the plugin folder says about itself to the script
/// extender.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginDeclaration {
    /// It describes itself with the version data the script extender reads
    /// since the game's version 1.6 (`SKSEPlugin_Version`).
    VersionData(PluginVersionData),
    /// It only answers the older question of the script extender
    /// (`SKSEPlugin_Query`): it was built for the game before version 1.6,
    /// and the script extender for 1.6 and later does not load it.
    QueryOnly,
    /// It describes itself in neither way: a support library a mod ships
    /// next to its plugin, which the script extender skips.
    Library,
    /// It could not be read as a DLL.
    Unreadable,
    /// It was not looked at: Mod Mallet does not know the version data of
    /// this game's script extender.
    NotRead,
}

/// The version data a script extender plugin gives about itself.
// The flags mirror the bits of the script extender's own structure.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginVersionData {
    /// The plugin's name, as it gives it.
    pub name: String,
    /// Its author, as it gives them; may be empty.
    pub author: String,
    /// Its own version number, as it gives it.
    pub plugin_version: u32,
    /// It finds the game's code through the Address Library, so it runs
    /// in every game version the Address Library has a file for.
    pub address_library: bool,
    /// It finds the game's code by patterns, without fixed addresses.
    pub signatures: bool,
    /// It uses the layout of the game's structures since version 1.6.629.
    pub structs_post_629: bool,
    /// It uses no structure of the game, or works with both layouts.
    pub no_struct_use: bool,
    /// It claims a way of being independent of the game version that the
    /// script extender does not know.
    pub unknown_independence: bool,
    /// The game versions it was built for, if it is not independent of the
    /// version.
    pub compatible: Vec<RuntimeVersion>,
    /// The oldest script extender it accepts, if it names one.
    pub extender_required: Option<RuntimeVersion>,
}

impl PluginVersionData {
    /// Whether it says it runs in more than the versions it lists.
    pub fn independent(&self) -> bool {
        self.address_library || self.signatures || self.unknown_independence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_packed_like_the_script_extender_packs_them() {
        assert_eq!(RuntimeVersion::new(1, 7, 104, 0).0, 0x0107_0680);
        assert_eq!(RuntimeVersion::new(1, 6, 1170, 0).0, 0x0106_4920);
        assert_eq!(
            RuntimeVersion::parse("1.7.104"),
            Some(RuntimeVersion(0x0107_0680))
        );
        assert_eq!(
            RuntimeVersion::parse("1.7.104.0"),
            Some(RuntimeVersion(0x0107_0680))
        );
        assert_eq!(RuntimeVersion(0x0107_0680).to_string(), "1.7.104");
        assert_eq!(RuntimeVersion::new(2, 3, 1, 0).to_string(), "2.3.1");
    }

    #[test]
    fn versions_compare_in_their_natural_order() {
        let parse = |text| RuntimeVersion::parse(text).unwrap();
        assert!(parse("1.6.1170") < parse("1.7.104"));
        assert!(parse("1.5.97") < parse("1.6.317"));
        assert!(parse("1.6.640") < parse("1.6.1170"));
    }

    #[test]
    fn what_is_not_a_version_is_not_read_as_one() {
        for text in [
            "",
            "1.7",
            "a.b.c",
            "1.7.104.0.0",
            "1.7.5000",
            "300.1.1",
            "1..1",
        ] {
            assert_eq!(RuntimeVersion::parse(text), None, "{text}");
        }
    }

    #[test]
    fn an_unversioned_extender_supports_any_game_version() {
        let any = ExtenderInstall::default();
        assert!(any.supports(RuntimeVersion::new(1, 7, 104, 0)));
        let skse = ExtenderInstall {
            name: "SKSE".to_owned(),
            version: None,
            runtimes: vec![RuntimeVersion::new(1, 6, 1170, 0)],
            versioned: true,
        };
        assert!(!skse.supports(RuntimeVersion::new(1, 7, 104, 0)));
        assert!(skse.supports(RuntimeVersion::new(1, 6, 1170, 0)));
    }
}
