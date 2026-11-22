//! What the ingest stage learned about a setup.
//!
//! Facts are plain data. Adapters produce them from files and logs; analyses
//! read them and never touch the disk themselves (ADR 0004).

pub mod crashes;
pub mod doors;
pub mod installer;
pub mod knowledge;
pub mod markers;
pub mod models;
pub mod navmeshes;
pub mod objects;
pub mod terrain;
pub mod versions;

use crate::GamePath;

pub use crashes::CrashFacts;
pub use doors::DoorFacts;
pub use installer::InstallerFacts;
pub use knowledge::KnowledgeFacts;
pub use markers::MarkerFacts;
pub use models::ModelFacts;
pub use navmeshes::NavmeshFacts;
pub use objects::ObjectFacts;
pub use terrain::TerrainFacts;
pub use versions::VersionFacts;

/// Everything known about one setup at the time of a scan.
///
/// A missing or empty fact means "could not be determined", and analyses
/// that need it simply report nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    /// What the script extender reported the last time the game started.
    pub extender: Option<ExtenderReport>,
    /// Every file that lies in the game's data directory as an ordinary
    /// file. A loose file takes precedence over the same file in any
    /// archive.
    pub loose_files: Vec<GamePath>,
    /// The archives the game loads, from lowest to highest precedence.
    pub archives: Vec<ArchiveFacts>,
    /// The active plugins, in load order.
    pub plugins: Vec<PluginFacts>,
    /// Why the load order could not be determined, if it could not. Without
    /// a load order there are no archive and plugin facts.
    pub load_order_problem: Option<String>,
    /// Files that should have been read but could not be.
    pub unreadable: Vec<UnreadableFile>,
    /// The installers of the installed mods, as far as they could be read.
    pub installers: Vec<InstallerFacts>,
    /// Loose meshes that replace a mesh from an archive.
    pub mesh_overrides: Vec<MeshOverride>,
    /// Meshes in the variant of the format of another game, or of another
    /// edition of this one, loose or in the archives of mods.
    pub foreign_meshes: Vec<ForeignMesh>,
    /// The surface textures every mesh names, loose or in the archives of
    /// mods, where they could be read.
    pub mesh_textures: Vec<MeshTextures>,
    /// The objects mods placed in the world.
    pub objects: ObjectFacts,
    /// The shape of the land.
    pub terrain: TerrainFacts,
    /// What community-maintained rules say about the active plugins.
    pub knowledge: KnowledgeFacts,
    /// The places of the world map.
    pub markers: MarkerFacts,
    /// The navigation meshes of other plugins that mods change.
    pub navmeshes: NavmeshFacts,
    /// The placed objects that Mod Mallet's patch in the game hides, by
    /// the plugin that placed them (ADR 0025).
    pub hidden_objects: Vec<RecordId>,
    /// The recent crashes, as the crash loggers reported them.
    pub crashes: CrashFacts,
    /// The shapes of the models of objects that may collide.
    pub models: ModelFacts,
    /// The plugins of mods the mod manager keeps switched off, other than
    /// the game's own, and whether the game loads them all the same
    /// (ADR 0061).
    pub switched_off_plugins: Vec<crate::SwitchedOffPlugin>,
    /// Every door, where it leads and which paths go through it
    /// (ADR 0067).
    pub doors: DoorFacts,
    /// The versions of the game, its script extender and the extender's
    /// plugins, as their files tell them now (ADR 0079).
    pub versions: VersionFacts,
}

/// The surface textures a mesh names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshTextures {
    /// Where the mesh lies.
    pub path: GamePath,
    /// The archive it is packed in, or `None` for a loose file.
    pub archive: Option<GamePath>,
    /// The textures, by the path the game looks them up under.
    pub textures: Vec<GamePath>,
}

/// A mesh made for another game or another edition of the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignMesh {
    /// Where the mesh lies.
    pub path: GamePath,
    /// The archive it is packed in, or `None` for a loose file.
    pub archive: Option<GamePath>,
    /// The Bethesda version of its format, such as 83 for Skyrim (2011).
    pub bethesda_version: u32,
    /// Whether it bends with a skeleton, as bodies, clothes and armour do.
    pub skinned: bool,
}

/// A loose mesh that replaces a mesh from an archive, with the textures
/// both versions use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshOverride {
    /// Where the mesh lies.
    pub path: GamePath,
    /// The textures the loose mesh uses.
    pub textures: Vec<GamePath>,
    /// The archive whose version of the mesh the game would otherwise use.
    pub replaced_in: GamePath,
    /// The textures that version uses.
    pub replaced_textures: Vec<GamePath>,
}

/// An archive the game loads, and what it contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveFacts {
    /// Where the archive lies.
    pub path: GamePath,
    /// The files inside it, by the path the game sees them under.
    pub files: Vec<GamePath>,
}

/// An active plugin, and the parts of it the analyses look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginFacts {
    /// The plugin's file name in the spelling of the load order.
    pub name: String,
    /// Where the plugin lies.
    pub path: GamePath,
    /// File names of the plugins this one builds on, as it names them.
    pub masters: Vec<String>,
    /// The characters the plugin defines or changes.
    pub npcs: Vec<NpcFacts>,
}

/// The identity of a record, independent of the load order (ADR 0007).
///
/// Inside a plugin file a record is numbered relative to that plugin's list
/// of masters. Resolving the number to the plugin that first defined the
/// record gives an identity that is the same in every plugin and every
/// setup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecordId {
    /// File name of the plugin that defines the record, in lower case.
    pub plugin: String,
    /// The record's number within that plugin.
    pub local_id: u32,
}

impl std::fmt::Display for RecordId {
    /// The plugin and the record's number in it, for example
    /// `skyrim.esm:013BA1`. Unlike a form ID as the game shows it, this does
    /// not depend on the load order.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{:06X}", self.plugin, self.local_id)
    }
}

/// One plugin's version of a character.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcFacts {
    /// Which character this is.
    pub id: RecordId,
    /// The character's internal name, if the record has one.
    pub editor_id: Option<String>,
    /// Whether the character takes its looks from a template instead of
    /// having a face of its own.
    pub inherits_appearance: bool,
    /// Everything in the record that determines the face, in a form that
    /// compares equal exactly when two plugins describe the same face.
    pub appearance: Vec<u8>,
}

/// A file that should have been read but could not be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableFile {
    /// Where the file lies.
    pub path: GamePath,
    /// What went wrong, in the words of the reader.
    pub reason: String,
}

/// The outcome of a script extender loading its native plugins.
///
/// A script extender (SKSE for Skyrim, F4SE for Fallout 4, ...) is the loader
/// that lets mods ship native code as DLL files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtenderReport {
    /// Version of the script extender itself, for example `2.3.1`.
    pub extender_version: Option<String>,
    /// Version of the game the extender was running in, for example
    /// `1.7.104`.
    pub game_version: Option<String>,
    /// The directory the extender loads plugins from.
    pub plugin_dir: GamePath,
    /// One entry per DLL the extender examined.
    pub plugins: Vec<ExtenderPlugin>,
}

/// One DLL in the script extender's plugin directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtenderPlugin {
    /// File name without directory, in the spelling the extender logged.
    pub file_name: String,
    /// What happened when the extender tried to load it.
    pub status: ExtenderPluginStatus,
}

/// What happened when the script extender tried to load a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtenderPluginStatus {
    /// The plugin is running.
    Loaded,
    /// The plugin was built for a different version of the game and was
    /// switched off.
    IncompatibleGameVersion,
    /// The DLL does not describe itself as a plugin at all. This is normal
    /// for support libraries that a mod ships next to its plugin.
    NoVersionData,
    /// The plugin was not loaded for another reason, given in the
    /// extender's own words.
    NotLoaded {
        /// The reason as the extender logged it.
        reason: String,
    },
}
