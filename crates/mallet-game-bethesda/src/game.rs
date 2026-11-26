//! The games of the Bethesda family this adapter reads, and what differs
//! between them.
//!
//! The games share their file formats and most of what their records mean
//! (ADR 0036). What differs is collected here, in one table per game: where
//! the game is installed and keeps its settings, which plugins and archives
//! are its own, how archives are named after plugins, which script extender
//! it has and where that writes its log, and which tools go with it.
//!
//! Everything here is a constant. Finding a game on this machine is
//! [`crate::locate`]'s work.

use mallet_core::{GameLack, GameSupport, PakFolder};

/// A game this adapter knows. Skyrim Special Edition, the first game Mod
/// Mallet read, is the one assumed where nothing else is said.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Game {
    /// The Elder Scrolls V: Skyrim Special Edition, and the Anniversary
    /// Edition, which is the same game with more Creation Club content.
    #[default]
    SkyrimSe,
    /// The Elder Scrolls V: Skyrim VR.
    SkyrimVr,
    /// The Elder Scrolls V: Skyrim, as released in 2011 ("Legendary
    /// Edition").
    Skyrim,
    /// Fallout 4.
    Fallout4,
    /// Fallout 4 VR.
    Fallout4Vr,
    /// Fallout: New Vegas.
    FalloutNv,
    /// Fallout 3.
    Fallout3,
    /// The Elder Scrolls IV: Oblivion, as released in 2006.
    Oblivion,
    /// The Elder Scrolls IV: Oblivion Remastered (2025). It runs the old
    /// game's logic on the old game's data, below a folder of its own
    /// (`ObvData`), and draws with Unreal Engine (ADR 0065).
    OblivionRemastered,
    /// Enderal: Forgotten Stories (Special Edition), a total conversion of
    /// Skyrim Special Edition by the studio Sure AI, sold as a game of its
    /// own. It runs on Skyrim Special Edition's program and formats, with
    /// plugins, archives and settings of its own (ADR 0078).
    EnderalSe,
}

/// Groups of games that share how their records are laid out and what the
/// game numbers in them mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Skyrim, Skyrim Special Edition and Skyrim VR.
    Skyrim,
    /// Fallout 4 and Fallout 4 VR.
    Fallout4,
    /// Fallout 3 and Fallout: New Vegas.
    Fallout3,
    /// Oblivion and Oblivion Remastered.
    Oblivion,
}

/// How a game finds the archives that belong to a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveNaming {
    /// `<plugin>.bsa` and `<plugin> - Textures.bsa` (Skyrim).
    Skyrim,
    /// `<plugin> - <anything>.ba2` (Fallout 4).
    Suffixed,
    /// Every archive whose name starts with the plugin's name (Oblivion,
    /// Fallout 3, Fallout: New Vegas).
    Prefixed,
}

/// Where a script extender writes its log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPlace {
    /// In a folder below the game's folder in `Documents/My Games`.
    MyGames {
        /// The folder below the game's own, such as `SKSE` or `OBSE/Logs`,
        /// its parts separated by `/`.
        folder: &'static str,
        /// The log's file name.
        file: &'static str,
    },
    /// Directly in the game's directory.
    GameDir {
        /// The log's file name.
        file: &'static str,
    },
}

/// Where OBSE64 writes its log, as its library names it (ADR 0092).
const OBSE64_LOG: LogPlace = LogPlace::MyGames {
    folder: "OBSE/Logs",
    file: "obse64.txt",
};

/// A game's script extender: the loader that starts the game with it, and
/// where its plugins and log are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptExtender {
    /// The identifier of the extender in the toolbox, such as `skse`.
    pub tool: &'static str,
    /// The short name people know it by, such as `SKSE`.
    pub short_name: &'static str,
    /// The program that starts the game with the extender, relative to the
    /// directory the game is installed in.
    pub loader: &'static str,
    /// The directory its plugins are loaded from, relative to the
    /// directory the game is installed in, in lower case.
    pub plugin_dir: &'static str,
    /// Where it writes its log, if Mod Mallet reads it.
    pub log: Option<LogPlace>,
    /// The start of the name of the library it ships for each game
    /// version, such as `skse64` for `skse64_1_6_1170.dll`, if the
    /// extender names its libraries that way and logs the game version
    /// in the same form.
    pub versioned_library: Option<&'static str>,
    /// Its page.
    pub page: &'static str,
}

/// A library that tells script extender plugins where things are in each
/// version of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressLibrary {
    /// The identifier in the toolbox.
    pub tool: &'static str,
    /// The extension of its database files, which are named after the
    /// game version (`versionlib-1-6-1170-0.bin`).
    pub extension: &'static str,
    /// Its page.
    pub page: &'static str,
}

/// The editor Bethesda released for a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Editor {
    /// The identifier in the toolbox.
    pub tool: &'static str,
    /// The program, in the game's directory.
    pub program: &'static str,
    /// Where to get it.
    pub page: &'static str,
}

impl Game {
    /// Every game this adapter knows, in the order they are offered.
    pub const ALL: [Self; 10] = [
        Self::SkyrimSe,
        Self::SkyrimVr,
        Self::Skyrim,
        Self::EnderalSe,
        Self::Fallout4,
        Self::Fallout4Vr,
        Self::FalloutNv,
        Self::Fallout3,
        Self::Oblivion,
        Self::OblivionRemastered,
    ];

    /// The game with the identifier `id`, ignoring case.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|game| game.id().eq_ignore_ascii_case(id))
    }

    /// The game's identifier, the same Vortex uses: `skyrimse`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::SkyrimSe => "skyrimse",
            Self::SkyrimVr => "skyrimvr",
            Self::Skyrim => "skyrim",
            Self::Fallout4 => "fallout4",
            Self::Fallout4Vr => "fallout4vr",
            Self::FalloutNv => "falloutnv",
            Self::Fallout3 => "fallout3",
            Self::Oblivion => "oblivion",
            Self::OblivionRemastered => "oblivionremastered",
            Self::EnderalSe => "enderalspecialedition",
        }
    }

    /// The game's name as shown to people.
    pub const fn name(self) -> &'static str {
        match self {
            Self::SkyrimSe => "Skyrim Special Edition",
            Self::SkyrimVr => "Skyrim VR",
            Self::Skyrim => "Skyrim (2011)",
            Self::Fallout4 => "Fallout 4",
            Self::Fallout4Vr => "Fallout 4 VR",
            Self::FalloutNv => "Fallout: New Vegas",
            Self::Fallout3 => "Fallout 3",
            Self::Oblivion => "Oblivion",
            Self::OblivionRemastered => "Oblivion Remastered",
            Self::EnderalSe => "Enderal Special Edition",
        }
    }

    /// The group of games this one shares its records with.
    pub const fn family(self) -> Family {
        match self {
            Self::SkyrimSe | Self::SkyrimVr | Self::Skyrim | Self::EnderalSe => Family::Skyrim,
            Self::Fallout4 | Self::Fallout4Vr => Family::Fallout4,
            Self::FalloutNv | Self::Fallout3 => Family::Fallout3,
            Self::Oblivion | Self::OblivionRemastered => Family::Oblivion,
        }
    }

    /// How far Mod Mallet can look at a setup of this game.
    pub const fn support(self) -> GameSupport {
        match self {
            Self::SkyrimSe | Self::EnderalSe => GameSupport::Full,
            _ => GameSupport::Basic,
        }
    }

    /// The game's program, in its directory. Its presence tells a game
    /// directory from any other.
    pub const fn executable(self) -> &'static str {
        match self {
            // Enderal runs on Skyrim Special Edition's program;
            // [`Game::launcher`] tells the two apart.
            Self::SkyrimSe | Self::EnderalSe => "SkyrimSE.exe",
            Self::SkyrimVr => "SkyrimVR.exe",
            Self::Skyrim => "TESV.exe",
            Self::Fallout4 => "Fallout4.exe",
            Self::Fallout4Vr => "Fallout4VR.exe",
            Self::FalloutNv => "FalloutNV.exe",
            Self::Fallout3 => "Fallout3.exe",
            Self::Oblivion => "Oblivion.exe",
            Self::OblivionRemastered => "OblivionRemastered.exe",
        }
    }

    /// The folders, from the directory the game is installed in, down to
    /// the one that holds the game's `Data` folder. Empty for every game
    /// but Oblivion Remastered, which keeps the old game's data in
    /// `OblivionRemastered/Content/Dev/ObvData` (ADR 0065).
    pub const fn data_root(self) -> &'static [&'static str] {
        match self {
            Self::OblivionRemastered => &["OblivionRemastered", "Content", "Dev", "ObvData"],
            _ => &[],
        }
    }

    /// Whether the game draws its world from the meshes and textures in its
    /// data folder. Oblivion Remastered does not: it draws with Unreal
    /// Engine from assets of its own, so what Mod Mallet reads of meshes
    /// and textures, and its 3D views, say nothing about it (ADR 0065).
    pub const fn draws_its_data(self) -> bool {
        !matches!(self, Self::OblivionRemastered)
    }

    /// The folders below the directory the game is installed in from
    /// which Unreal Engine loads mods packed as `.pak` files, with the
    /// kind of each folder. Empty for the games not drawn with Unreal
    /// Engine.
    pub const fn unreal_pak_folders(self) -> &'static [(&'static str, PakFolder)] {
        match self {
            Self::OblivionRemastered => &[
                ("OblivionRemastered/Content/Paks/~mods", PakFolder::Mods),
                (
                    "OblivionRemastered/Content/Paks/LogicMods",
                    PakFolder::LogicMods,
                ),
            ],
            _ => &[],
        }
    }

    /// Whether the game is a 64-bit program. The older games are 32-bit,
    /// and so are the script extender plugins made for them.
    pub const fn is_64_bit(self) -> bool {
        matches!(
            self,
            Self::SkyrimSe
                | Self::EnderalSe
                | Self::SkyrimVr
                | Self::Fallout4
                | Self::Fallout4Vr
                | Self::OblivionRemastered
        )
    }

    /// The key below `HKEY_LOCAL_MACHINE\SOFTWARE\Bethesda Softworks` in
    /// which the game's launcher writes where it installed the game, if it
    /// writes one there. Enderal, not a game of Bethesda's, writes none.
    pub const fn registry_key(self) -> Option<&'static str> {
        Some(match self {
            Self::SkyrimSe => "Skyrim Special Edition",
            Self::SkyrimVr => "Skyrim VR",
            Self::Skyrim => "Skyrim",
            Self::Fallout4 => "Fallout4",
            Self::Fallout4Vr => "Fallout 4 VR",
            Self::FalloutNv => "FalloutNV",
            Self::Fallout3 => "Fallout3",
            Self::Oblivion => "Oblivion",
            Self::OblivionRemastered => "Oblivion Remastered",
            Self::EnderalSe => return None,
        })
    }

    /// A file in the directory the game is installed in that tells it from
    /// another game with the same program: Enderal's launcher, beside
    /// Skyrim Special Edition's program. `None` for the games whose
    /// program is their own.
    pub const fn launcher(self) -> Option<&'static str> {
        match self {
            Self::EnderalSe => Some("Enderal Launcher.exe"),
            _ => None,
        }
    }

    /// The identifiers of games whose picture may stand for this one where
    /// the mod manager has none of its own: Vortex brings one of the first
    /// Enderal but none of Enderal Special Edition.
    pub const fn related_art(self) -> &'static [&'static str] {
        match self {
            Self::EnderalSe => &["enderal"],
            _ => &[],
        }
    }

    /// The folders below `steamapps/common` that Steam installs the game
    /// into.
    pub const fn steam_folders(self) -> &'static [&'static str] {
        match self {
            Self::SkyrimSe => &["Skyrim Special Edition"],
            Self::SkyrimVr => &["SkyrimVR"],
            Self::Skyrim => &["Skyrim"],
            Self::Fallout4 => &["Fallout 4"],
            Self::Fallout4Vr => &["Fallout 4 VR"],
            Self::FalloutNv => &["Fallout New Vegas", "FalloutNV"],
            Self::Fallout3 => &["Fallout 3 goty", "Fallout 3"],
            Self::Oblivion => &["Oblivion"],
            Self::OblivionRemastered => &["Oblivion Remastered"],
            Self::EnderalSe => &["Enderal Special Edition"],
        }
    }

    /// The identifiers under which GOG Galaxy registers the game, if it
    /// sells it.
    pub const fn gog_ids(self) -> &'static [&'static str] {
        match self {
            Self::SkyrimSe => &["1711230643"],
            Self::Fallout4 => &["1998527297"],
            Self::FalloutNv => &["1454587428"],
            Self::Fallout3 => &["1454315831"],
            Self::Oblivion => &["1458058109"],
            // GOG sells Enderal Special Edition too; its identifier there
            // could not be confirmed, so only Steam's copy is looked for.
            Self::SkyrimVr
            | Self::Skyrim
            | Self::Fallout4Vr
            | Self::OblivionRemastered
            | Self::EnderalSe => &[],
        }
    }

    /// The game's folder below `Documents/My Games`, which holds its
    /// settings, saves and often the script extender's log.
    pub const fn my_games_folder(self) -> &'static str {
        match self {
            Self::SkyrimSe => "Skyrim Special Edition",
            Self::SkyrimVr => "Skyrim VR",
            Self::Skyrim => "Skyrim",
            Self::Fallout4 => "Fallout4",
            Self::Fallout4Vr => "Fallout4VR",
            Self::FalloutNv => "FalloutNV",
            Self::Fallout3 => "Fallout3",
            Self::Oblivion => "Oblivion",
            Self::OblivionRemastered => "Oblivion Remastered",
            Self::EnderalSe => "Enderal Special Edition",
        }
    }

    /// The domain of the game's mods on Nexus Mods
    /// (`https://www.nexusmods.com/<domain>`).
    pub const fn nexus_domain(self) -> &'static str {
        match self {
            // Skyrim VR plays the mods made for the Special Edition.
            Self::SkyrimSe | Self::SkyrimVr => "skyrimspecialedition",
            Self::Skyrim => "skyrim",
            Self::Fallout4 | Self::Fallout4Vr => "fallout4",
            Self::FalloutNv => "newvegas",
            Self::Fallout3 => "fallout3",
            Self::Oblivion => "oblivion",
            Self::OblivionRemastered => "oblivionremastered",
            Self::EnderalSe => "enderalspecialedition",
        }
    }

    /// The game's own plugins, in lower case. Changes made to them do not
    /// last: the game's updates replace them.
    pub const fn official_plugins(self) -> &'static [&'static str] {
        match self {
            Self::SkyrimSe => &[
                "skyrim.esm",
                "update.esm",
                "dawnguard.esm",
                "hearthfires.esm",
                "dragonborn.esm",
            ],
            Self::SkyrimVr => &[
                "skyrim.esm",
                "update.esm",
                "dawnguard.esm",
                "hearthfires.esm",
                "dragonborn.esm",
                "skyrimvr.esm",
            ],
            Self::Skyrim => &[
                "skyrim.esm",
                "update.esm",
                "dawnguard.esm",
                "hearthfires.esm",
                "dragonborn.esm",
                "highrestexturepack01.esp",
                "highrestexturepack02.esp",
                "highrestexturepack03.esp",
            ],
            // Enderal ships its own versions of Skyrim's masters, its own
            // master and SkyUI.
            Self::EnderalSe => &[
                "skyrim.esm",
                "update.esm",
                "dawnguard.esm",
                "hearthfires.esm",
                "dragonborn.esm",
                "enderal - forgotten stories.esm",
                "skyui_se.esp",
            ],
            Self::Fallout4 => &[
                "fallout4.esm",
                "dlcrobot.esm",
                "dlcworkshop01.esm",
                "dlccoast.esm",
                "dlcworkshop02.esm",
                "dlcworkshop03.esm",
                "dlcnukaworld.esm",
                "dlcultrahighresolution.esm",
            ],
            Self::Fallout4Vr => &["fallout4.esm", "fallout4_vr.esm"],
            Self::FalloutNv => &[
                "falloutnv.esm",
                "deadmoney.esm",
                "honesthearts.esm",
                "oldworldblues.esm",
                "lonesomeroad.esm",
                "gunrunnersarsenal.esm",
                "caravanpack.esm",
                "classicpack.esm",
                "mercenarypack.esm",
                "tribalpack.esm",
            ],
            Self::Fallout3 => &[
                "fallout3.esm",
                "anchorage.esm",
                "thepitt.esm",
                "brokensteel.esm",
                "pointlookout.esm",
                "zeta.esm",
            ],
            Self::Oblivion => &[
                "oblivion.esm",
                "dlcshiveringisles.esp",
                "knights.esp",
                "dlcbattlehorncastle.esp",
                "dlcfrostcrag.esp",
                "dlchorses.esp",
                "dlcmehrunesrazor.esp",
                "dlcorrery.esp",
                "dlcspelltomes.esp",
                "dlcthievesden.esp",
                "dlcvilelair.esp",
            ],
            Self::OblivionRemastered => &[
                "oblivion.esm",
                "dlcbattlehorncastle.esp",
                "dlcfrostcrag.esp",
                "dlchorsearmor.esp",
                "dlcmehrunesrazor.esp",
                "dlcorrery.esp",
                "dlcshiveringisles.esp",
                "dlcspelltomes.esp",
                "dlcthievesden.esp",
                "dlcvilelair.esp",
                "knights.esp",
                "altarespmain.esp",
                "altardeluxe.esp",
                "altargymnavigation.esp",
                "tamrielleveledregion.esp",
            ],
        }
    }

    /// Whether `plugin` is one of the game's own plugins.
    pub fn is_official(self, plugin: &str) -> bool {
        self.official_plugins()
            .iter()
            .any(|official| official.eq_ignore_ascii_case(plugin))
    }

    /// The file in the game directory that lists the Creation Club
    /// content the game counts as its own, if the game has one.
    pub const fn creation_club_list(self) -> Option<&'static str> {
        match self {
            Self::SkyrimSe => Some("Skyrim.ccc"),
            Self::Fallout4 => Some("Fallout4.ccc"),
            _ => None,
        }
    }

    /// The extension of the game's archives, without the dot.
    pub const fn archive_extension(self) -> &'static str {
        match self.family() {
            Family::Fallout4 => "ba2",
            _ => "bsa",
        }
    }

    /// How the game finds the archives of a plugin.
    pub const fn archive_naming(self) -> ArchiveNaming {
        match self.family() {
            Family::Skyrim => ArchiveNaming::Skyrim,
            Family::Fallout4 => ArchiveNaming::Suffixed,
            Family::Fallout3 | Family::Oblivion => ArchiveNaming::Prefixed,
        }
    }

    /// How the names of the game's own archives start, in lower case. The
    /// game's settings list them; they load before every plugin's.
    pub const fn base_archive_prefixes(self) -> &'static [&'static str] {
        match self {
            Self::SkyrimSe | Self::Skyrim => &["skyrim - "],
            // Enderal's own archives are named `E - Meshes.bsa`, `L - Voices.bsa`
            // and so on.
            Self::EnderalSe => &["skyrim - ", "e - ", "l - "],
            Self::SkyrimVr => &["skyrim - ", "skyrim_vr - "],
            Self::Fallout4 => &["fallout4 - "],
            Self::Fallout4Vr => &["fallout4 - ", "fallout4_vr - "],
            Self::FalloutNv | Self::Fallout3 => &["fallout - "],
            Self::Oblivion | Self::OblivionRemastered => &["oblivion - "],
        }
    }

    /// The game's own archives that load after the others, in lower case.
    pub const fn base_archives_last(self) -> &'static [&'static str] {
        match self {
            Self::SkyrimSe | Self::SkyrimVr | Self::Skyrim | Self::EnderalSe => {
                &["skyrim - patch.bsa"]
            }
            Self::FalloutNv => &["update.bsa"],
            _ => &[],
        }
    }

    /// The game's script extender.
    pub const fn script_extender(self) -> ScriptExtender {
        match self {
            Self::SkyrimSe | Self::EnderalSe => ScriptExtender {
                tool: "skse",
                short_name: "SKSE",
                loader: "skse64_loader.exe",
                plugin_dir: "data/skse/plugins",
                log: Some(LogPlace::MyGames {
                    folder: "SKSE",
                    file: "skse64.log",
                }),
                versioned_library: Some("skse64"),
                page: "https://skse.silverlock.org/",
            },
            Self::SkyrimVr => ScriptExtender {
                tool: "skse-vr",
                short_name: "SKSEVR",
                loader: "sksevr_loader.exe",
                plugin_dir: "data/skse/plugins",
                log: Some(LogPlace::MyGames {
                    folder: "SKSE",
                    file: "sksevr.log",
                }),
                versioned_library: Some("sksevr"),
                page: "https://skse.silverlock.org/",
            },
            Self::Skyrim => ScriptExtender {
                tool: "skse-classic",
                short_name: "SKSE",
                loader: "skse_loader.exe",
                plugin_dir: "data/skse/plugins",
                log: Some(LogPlace::MyGames {
                    folder: "SKSE",
                    file: "skse.log",
                }),
                versioned_library: None,
                page: "https://skse.silverlock.org/",
            },
            Self::Fallout4 => ScriptExtender {
                tool: "f4se",
                short_name: "F4SE",
                loader: "f4se_loader.exe",
                plugin_dir: "data/f4se/plugins",
                log: Some(LogPlace::MyGames {
                    folder: "F4SE",
                    file: "f4se.log",
                }),
                versioned_library: Some("f4se"),
                page: "https://f4se.silverlock.org/",
            },
            Self::Fallout4Vr => ScriptExtender {
                tool: "f4se-vr",
                short_name: "F4SEVR",
                loader: "f4sevr_loader.exe",
                plugin_dir: "data/f4se/plugins",
                log: Some(LogPlace::MyGames {
                    folder: "F4SE",
                    file: "f4sevr.log",
                }),
                versioned_library: Some("f4sevr"),
                page: "https://f4se.silverlock.org/",
            },
            Self::FalloutNv => ScriptExtender {
                tool: "nvse",
                short_name: "xNVSE",
                loader: "nvse_loader.exe",
                plugin_dir: "data/nvse/plugins",
                log: Some(LogPlace::GameDir { file: "nvse.log" }),
                versioned_library: None,
                page: "https://github.com/xNVSE/NVSE",
            },
            Self::Fallout3 => ScriptExtender {
                tool: "fose",
                short_name: "FOSE",
                loader: "fose_loader.exe",
                plugin_dir: "data/fose/plugins",
                log: Some(LogPlace::GameDir { file: "fose.log" }),
                versioned_library: None,
                page: "https://fose.silverlock.org/",
            },
            Self::Oblivion => ScriptExtender {
                tool: "obse",
                short_name: "OBSE",
                loader: "obse_loader.exe",
                plugin_dir: "data/obse/plugins",
                log: Some(LogPlace::GameDir { file: "obse.log" }),
                versioned_library: None,
                page: "https://github.com/llde/xOBSE",
            },
            // OBSE64 runs with the Steam version only. Its plugins lie
            // beside the game's Unreal program, not in the data folder
            // (ADR 0065). It is forked from SFSE and logs in SKSE's shapes
            // (ADR 0092).
            Self::OblivionRemastered => ScriptExtender {
                tool: "obse64",
                short_name: "OBSE64",
                loader: "OblivionRemastered/Binaries/Win64/obse64_loader.exe",
                plugin_dir: "oblivionremastered/binaries/win64/obse/plugins",
                log: Some(OBSE64_LOG),
                // This game's versions do not fit the packing (ADR 0092).
                versioned_library: None,
                page: "https://www.nexusmods.com/oblivionremastered/mods/282",
            },
        }
    }

    /// The address library of the game's script extender plugins, if the
    /// game has one.
    pub const fn address_library(self) -> Option<AddressLibrary> {
        match self {
            Self::SkyrimSe | Self::EnderalSe => Some(AddressLibrary {
                tool: "address-library",
                extension: "bin",
                page: "https://www.nexusmods.com/skyrimspecialedition/mods/32444",
            }),
            Self::SkyrimVr => Some(AddressLibrary {
                tool: "address-library-vr",
                extension: "csv",
                page: "https://www.nexusmods.com/skyrimspecialedition/mods/58101",
            }),
            Self::Fallout4 => Some(AddressLibrary {
                tool: "address-library-f4se",
                extension: "bin",
                page: "https://www.nexusmods.com/fallout4/mods/47327",
            }),
            _ => None,
        }
    }

    /// Where UE4SS, the loader of the Unreal Engine side's logic mods, puts
    /// its library, relative to the directory the game is installed in;
    /// `None` for the games not drawn with Unreal Engine.
    pub const fn ue4ss(self) -> Option<&'static str> {
        match self {
            Self::OblivionRemastered => Some("OblivionRemastered/Binaries/Win64/ue4ss/UE4SS.dll"),
            _ => None,
        }
    }

    /// The editor Bethesda released for the game, if there is one.
    /// Oblivion Remastered has none.
    pub const fn editor(self) -> Option<Editor> {
        Some(match self {
            Self::SkyrimSe | Self::SkyrimVr => Editor {
                tool: "creation-kit",
                program: "CreationKit.exe",
                page: "https://store.steampowered.com/app/1946180/",
            },
            Self::Skyrim => Editor {
                tool: "creation-kit",
                program: "CreationKit.exe",
                page: "https://store.steampowered.com/app/202480/",
            },
            Self::Fallout4 | Self::Fallout4Vr => Editor {
                tool: "creation-kit",
                program: "CreationKit.exe",
                page: "https://store.steampowered.com/app/1946160/",
            },
            Self::FalloutNv | Self::Fallout3 => Editor {
                tool: "geck",
                program: "GECK.exe",
                page: "https://geckwiki.com/",
            },
            Self::Oblivion => Editor {
                tool: "construction-set",
                program: "TESConstructionSet.exe",
                page: "https://cs.uesp.net/",
            },
            // Mods for Enderal are made with Skyrim Special Edition's
            // Creation Kit; Enderal brings no editor of its own.
            Self::OblivionRemastered | Self::EnderalSe => return None,
        })
    }

    /// Whether Mod Mallet's patches work in the game: they are read by
    /// Base Object Swapper, which exists for Skyrim Special Edition, and so
    /// for Enderal on its program, and for Skyrim VR.
    pub const fn has_object_swapper(self) -> bool {
        matches!(self, Self::SkyrimSe | Self::SkyrimVr | Self::EnderalSe)
    }

    /// Whether Mod Mallet writes plugins for the game: of moves, paths and
    /// doors. They copy fields as Skyrim Special Edition lays them out, which
    /// Enderal, on the same program, reads too.
    pub const fn writes_plugins(self) -> bool {
        matches!(self, Self::SkyrimSe | Self::EnderalSe)
    }

    /// Whether Mod Mallet reads the paths characters walk in the game: the
    /// navigation meshes of Skyrim, Fallout 4, Fallout 3 and New Vegas
    /// (ADR 0092). Oblivion keeps path grids instead.
    pub const fn reads_walkways(self) -> bool {
        matches!(
            self.family(),
            Family::Skyrim | Family::Fallout4 | Family::Fallout3
        )
    }

    /// Whether Mod Mallet reads the game's things for the catalogue: the
    /// item records of the Skyrim games and of Fallout 4 (ADR 0054, 0092).
    pub const fn has_catalogue(self) -> bool {
        matches!(self.family(), Family::Skyrim | Family::Fallout4)
    }

    /// The parts of Mod Mallet that do not apply to the game, for the
    /// window to say so (ADR 0078).
    pub fn lacks(self) -> Vec<GameLack> {
        [
            (GameLack::Catalogue, !self.has_catalogue()),
            (GameLack::Patches, !self.has_object_swapper()),
            (GameLack::WrittenPlugins, !self.writes_plugins()),
            (GameLack::Walkways, !self.reads_walkways()),
        ]
        .into_iter()
        .filter_map(|(lack, applies)| applies.then_some(lack))
        .collect()
    }

    /// Whether the game keeps each character's face in a mesh of its
    /// own, named after the character (`facegendata/facegeom`). Only then
    /// can a face be compared with the record it belongs to.
    pub const fn has_face_meshes(self) -> bool {
        matches!(self.family(), Family::Skyrim | Family::Fallout4)
    }

    /// Whether the game keeps pictures of its land from above in
    /// `textures/terrain/<world>/<world>.32.<x>.<y>.dds`.
    pub const fn has_terrain_lod_pictures(self) -> bool {
        matches!(self.family(), Family::Skyrim | Family::Fallout4)
    }

    /// The identifier of the game for the library that reads load orders.
    pub const fn load_order_id(self) -> loadorder::GameId {
        use loadorder::GameId;
        match self {
            // The library tells Enderal from Skyrim by its launcher, and then
            // reads its list of plugins from its own folder.
            Self::SkyrimSe | Self::EnderalSe => GameId::SkyrimSE,
            Self::SkyrimVr => GameId::SkyrimVR,
            Self::Skyrim => GameId::Skyrim,
            Self::Fallout4 => GameId::Fallout4,
            Self::Fallout4Vr => GameId::Fallout4VR,
            Self::FalloutNv => GameId::FalloutNV,
            Self::Fallout3 => GameId::Fallout3,
            Self::Oblivion => GameId::Oblivion,
            Self::OblivionRemastered => GameId::OblivionRemastered,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_unique_and_found_again() {
        for game in Game::ALL {
            assert_eq!(Game::from_id(game.id()), Some(game));
            assert_eq!(
                Game::ALL
                    .iter()
                    .filter(|other| other.id() == game.id())
                    .count(),
                1
            );
        }
        assert_eq!(Game::from_id("SkyrimSE"), Some(Game::SkyrimSe));
        assert_eq!(Game::from_id("morrowind"), None);
    }

    #[test]
    fn official_plugins_are_in_lower_case() {
        for game in Game::ALL {
            for plugin in game.official_plugins() {
                assert_eq!(*plugin, plugin.to_lowercase(), "{game:?}");
            }
            assert!(game.is_official(&game.official_plugins()[0].to_uppercase()));
        }
    }

    #[test]
    fn script_extender_plugin_dirs_are_below_the_data_directory() {
        for game in Game::ALL {
            let extender = game.script_extender();
            if game == Game::OblivionRemastered {
                // Beside the Unreal program, outside the data folder.
                assert!(!extender.plugin_dir.contains("data/"));
            } else {
                assert!(extender.plugin_dir.starts_with("data/"), "{game:?}");
            }
            assert_eq!(extender.plugin_dir, extender.plugin_dir.to_lowercase());
        }
    }

    #[test]
    fn oblivion_remastered_reads_like_oblivion_but_is_drawn_by_unreal() {
        let game = Game::OblivionRemastered;
        assert_eq!(game.family(), Family::Oblivion);
        assert_eq!(game.archive_extension(), "bsa");
        assert_eq!(game.archive_naming(), ArchiveNaming::Prefixed);
        assert!(!game.draws_its_data());
        assert!(Game::Oblivion.draws_its_data());
        assert_eq!(game.data_root().len(), 4);
        assert_eq!(Game::Oblivion.data_root().len(), 0);
        assert_eq!(game.unreal_pak_folders().len(), 2);
        assert_eq!(Game::Oblivion.unreal_pak_folders().len(), 0);
        assert!(game.editor().is_none());
        assert_eq!(
            game.script_extender().log,
            Some(LogPlace::MyGames {
                folder: "OBSE/Logs",
                file: "obse64.txt"
            })
        );
        assert!(game.is_official("AltarESPMain.esp"));
    }

    #[test]
    fn enderal_reads_like_skyrim_special_edition_with_files_of_its_own() {
        let game = Game::EnderalSe;
        assert_eq!(game.family(), Family::Skyrim);
        assert_eq!(game.executable(), Game::SkyrimSe.executable());
        assert_eq!(game.launcher(), Some("Enderal Launcher.exe"));
        assert_eq!(Game::SkyrimSe.launcher(), None);
        assert_eq!(game.load_order_id(), Game::SkyrimSe.load_order_id());
        assert_eq!(game.my_games_folder(), "Enderal Special Edition");
        assert_eq!(game.steam_folders(), ["Enderal Special Edition"]);
        assert_eq!(game.registry_key(), None);
        assert!(game.is_official("Enderal - Forgotten Stories.esm"));
        assert!(!Game::SkyrimSe.is_official("Enderal - Forgotten Stories.esm"));
        assert!(game.base_archive_prefixes().contains(&"e - "));
        assert_eq!(game.script_extender().loader, "skse64_loader.exe");
        assert!(game.has_object_swapper());
        assert!(game.editor().is_none());
        assert_eq!(game.related_art(), ["enderal"]);
        assert_eq!(Game::from_id("EnderalSpecialEdition"), Some(game));
    }

    #[test]
    fn what_does_not_apply_follows_from_the_table() {
        assert_eq!(Game::SkyrimSe.lacks(), Vec::new());
        assert_eq!(Game::EnderalSe.lacks(), Vec::new());
        assert_eq!(
            Game::Fallout4.lacks(),
            [GameLack::Patches, GameLack::WrittenPlugins]
        );
        assert!(Game::Oblivion.lacks().contains(&GameLack::Walkways));
        assert!(Game::Oblivion.lacks().contains(&GameLack::Catalogue));
        assert!(!Game::FalloutNv.lacks().contains(&GameLack::Walkways));
        assert!(!Game::SkyrimVr.lacks().contains(&GameLack::Patches));
    }

    #[test]
    fn fallout_4_packs_its_files_in_ba2_archives() {
        assert_eq!(Game::Fallout4.archive_extension(), "ba2");
        assert_eq!(Game::Fallout4Vr.archive_extension(), "ba2");
        assert_eq!(Game::FalloutNv.archive_extension(), "bsa");
        assert_eq!(Game::Oblivion.archive_extension(), "bsa");
    }
}
