//! What Mod Organizer knows about one mod, from the `meta.ini` in the
//! mod's folder:
//!
//! ```text
//! [General]
//! gameName=SkyrimSE
//! modid=1234
//! version=1.2.0
//! newestVersion=1.3.0
//! url=
//! hasCustomURL=false
//! installationFile=Some Mod-1234-1-2-0.7z
//! ```

use crate::ini::Ini;

/// The name of the file in each mod's folder in which Mod Organizer keeps
/// what it knows about the mod. It is not a file of the game.
pub const META_FILE: &str = "meta.ini";

/// What Mod Organizer knows about a mod.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModMeta {
    /// The game the mod was downloaded for, as Mod Organizer names it
    /// (`SkyrimSE`) or as Nexus Mods does (`skyrimspecialedition`).
    pub game_name: Option<String>,
    /// The mod's number on Nexus Mods. `None` for mods that do not come
    /// from there.
    pub mod_id: Option<u64>,
    /// The installed version, as the mod's author numbers it.
    pub version: Option<String>,
    /// The newest version Mod Organizer knows of.
    pub newest_version: Option<String>,
    /// The web page the user or Mod Organizer noted for the mod.
    pub url: Option<String>,
    /// The archive the mod was installed from, as Mod Organizer noted it:
    /// a file name or a full path.
    pub installation_file: Option<String>,
}

impl ModMeta {
    /// Reads the text of a `meta.ini`.
    pub fn parse(text: &str) -> Self {
        let ini = Ini::parse(text);
        let text = |key: &str| ini.text("General", key);
        Self {
            game_name: text("gameName"),
            mod_id: text("modid")
                .and_then(|id| id.parse::<u64>().ok())
                .filter(|id| *id > 0),
            version: text("version"),
            newest_version: text("newestVersion"),
            url: text("url"),
            installation_file: text("installationFile"),
        }
    }

    /// The newest version Mod Organizer knows of, if it is another one than
    /// the installed one.
    pub fn newer_version(&self) -> Option<String> {
        self.newest_version
            .clone()
            .filter(|newest| Some(newest) != self.version.as_ref())
    }

    /// The archive's file name, without the folder Mod Organizer may have
    /// noted with it.
    pub fn archive_name(&self) -> Option<String> {
        let file = self.installation_file.as_deref()?;
        let name = file.rsplit(['/', '\\']).next().unwrap_or(file).trim();
        (!name.is_empty()).then(|| name.to_owned())
    }

    /// The web page of the mod: the one noted, else the Nexus Mods page of
    /// a mod downloaded from there. Only `https` addresses are taken, so
    /// that what is shown can be opened without a second thought.
    pub fn page(&self) -> Option<String> {
        if let Some(url) = &self.url
            && url.starts_with("https://")
        {
            return Some(url.clone());
        }
        let domain = nexus_domain(self.game_name.as_deref()?)?;
        let id = self.mod_id?;
        Some(format!("https://www.nexusmods.com/{domain}/mods/{id}"))
    }
}

/// The part of a Nexus Mods address that names the game, for a game name
/// as Mod Organizer writes it into `meta.ini`.
fn nexus_domain(game_name: &str) -> Option<&'static str> {
    const DOMAINS: &[(&str, &str)] = &[
        ("skyrimse", "skyrimspecialedition"),
        ("skyrimspecialedition", "skyrimspecialedition"),
        ("skyrimvr", "skyrimspecialedition"),
        ("skyrim", "skyrim"),
        ("fallout4", "fallout4"),
        ("fallout4vr", "fallout4"),
        ("falloutnv", "newvegas"),
        ("newvegas", "newvegas"),
        ("fallout3", "fallout3"),
        ("oblivion", "oblivion"),
        ("enderal", "enderal"),
        ("enderalse", "enderalspecialedition"),
        ("enderalspecialedition", "enderalspecialedition"),
    ];
    let wanted = game_name.trim().to_ascii_lowercase();
    DOMAINS
        .iter()
        .find(|(name, _)| *name == wanted)
        .map(|(_, domain)| *domain)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// The shape of a `meta.ini` Mod Organizer 2.5 writes; names made up.
    const META: &str = "[General]\r\ngameName=SkyrimSE\r\nmodid=1234\r\nversion=1.2.0\r\nnewestVersion=1.3.0\r\nignoredVersion=\r\ncategory=\"28,\"\r\nnexusFileStatus=1\r\ninstallationFile=C:/Modding/downloads/Patch Hub-1234-1-2-0.7z\r\nrepository=Nexus\r\nurl=\r\nhasCustomURL=false\r\n\r\n[installedFiles]\r\n1\\modid=1234\r\n1\\fileid=5678\r\nsize=1\r\n";

    #[test]
    fn reads_version_id_and_archive() {
        let meta = ModMeta::parse(META);
        assert_eq!(meta.game_name.as_deref(), Some("SkyrimSE"));
        assert_eq!(meta.mod_id, Some(1234));
        assert_eq!(meta.version.as_deref(), Some("1.2.0"));
        assert_eq!(meta.newer_version().as_deref(), Some("1.3.0"));
        assert_eq!(
            meta.archive_name().as_deref(),
            Some("Patch Hub-1234-1-2-0.7z")
        );
        assert_eq!(meta.url, None);
    }

    #[test]
    fn the_page_is_the_noted_one_or_the_nexus_page() {
        let meta = ModMeta::parse(META);
        assert_eq!(
            meta.page().as_deref(),
            Some("https://www.nexusmods.com/skyrimspecialedition/mods/1234")
        );
        let custom = ModMeta {
            url: Some("https://example.org/mod".to_owned()),
            ..meta.clone()
        };
        assert_eq!(custom.page().as_deref(), Some("https://example.org/mod"));
        let unsafe_url = ModMeta {
            url: Some("javascript:alert(1)".to_owned()),
            mod_id: None,
            ..meta
        };
        assert_eq!(unsafe_url.page(), None);
    }

    #[test]
    fn a_mod_made_by_hand_has_no_details() {
        let meta = ModMeta::parse("[General]\nmodid=0\nversion=\n");
        assert_eq!(meta, ModMeta::default());
        assert_eq!(meta.page(), None);
        assert_eq!(meta.newer_version(), None);
    }

    #[test]
    fn the_same_version_is_not_newer() {
        let meta = ModMeta::parse("[General]\nversion=2.0\nnewestVersion=2.0\n");
        assert_eq!(meta.newer_version(), None);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(text in ".*") {
            let meta = ModMeta::parse(&text);
            let _ = (meta.page(), meta.archive_name(), meta.newer_version());
        }
    }
}
