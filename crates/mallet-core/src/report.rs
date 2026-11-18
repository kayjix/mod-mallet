//! The result of a scan.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Finding, FindingId, ModRef, ObjectRef};

/// Version of the [`ScanReport`] contract.
///
/// Raise it whenever a field is removed or changes meaning, so consumers
/// such as the desktop app or a mod manager extension can tell (ADR 0012).
pub const SCHEMA_VERSION: u32 = 1;

/// The game a scan looked at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    /// Identifier of the game adapter, for example `skyrimse`.
    pub id: String,
    /// The game's name.
    pub name: String,
    /// The game's version, if it could be determined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The script extender's version, if one is installed and has run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extender_version: Option<String>,
}

/// What a place the scan looked at is for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum LocationRole {
    /// The directory the game is installed in.
    GameDir,
    /// The game's list of active plugins.
    PluginList,
    /// The log of the script extender.
    ExtenderLog,
    /// The mod manager's saved state: installed mods and installer choices.
    ManagerState,
    /// The directory the mod manager keeps downloaded mod archives in.
    Downloads,
    /// The program used to look into mod archives.
    Unpacker,
    /// The mod manager's record of the files it deployed.
    DeploymentManifest,
    /// The list of community-maintained rules about plugins, the LOOT
    /// masterlist.
    Masterlist,
    /// A folder crash loggers write their reports to.
    CrashLogs,
    /// The folder the mod manager keeps every installed mod's files in,
    /// deployed or not. Only the names of the files are read.
    ModStaging,
}

/// How Mod Mallet came to look at a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LocationOrigin {
    /// The user named it.
    Given,
    /// It was read from the entry the game's launcher writes into the
    /// Windows registry.
    Registry,
    /// It is the place where the game or the mod manager always puts this
    /// file.
    Standard,
    /// It was found in one of Steam's game libraries.
    Steam,
    /// It was read from the entry GOG Galaxy writes into the Windows
    /// registry.
    Gog,
    /// It is named in the mod manager's record of the files it deployed.
    Manager,
}

/// How far Mod Mallet can look at a setup of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum GameSupport {
    /// Every check Mod Mallet has runs for this game.
    Full,
    /// The load order, files, archives, plugins, placed objects, the map
    /// and the toolbox are read; checks that need more knowledge of the
    /// game, such as the kinds of its map's places, are still missing.
    Basic,
}

/// The games Mod Mallet knows, and which of them the user looks at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GameChoice {
    /// The identifier of the game the user looks at.
    pub active: String,
    /// Every game Mod Mallet knows, in the order they are offered.
    pub games: Vec<KnownGame>,
}

/// A game Mod Mallet knows, and where it is on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnownGame {
    /// Identifier of the game, the same the mod manager uses, for example
    /// `skyrimse`.
    pub id: String,
    /// The game's name.
    pub name: String,
    /// How far Mod Mallet can look at a setup of this game.
    pub support: GameSupport,
    /// The directory the game is installed in, if it was found or named.
    pub dir: Option<String>,
    /// How the directory was found.
    pub origin: Option<LocationOrigin>,
    /// The short name of the game's script extender, such as `SKSE`.
    pub script_extender: String,
}

/// A place the scan looked at, so that nobody has to wonder what was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScanLocation {
    /// What the place is for.
    pub role: LocationRole,
    /// The path on this machine.
    pub path: String,
    /// How Mod Mallet came to look there.
    pub origin: LocationOrigin,
    /// Whether something was there to read.
    pub found: bool,
}

/// Everything one scan found.
///
/// This is the contract between the engine and whatever shows the result.
/// Its JSON form is described by `schemas/scan-report.schema.json`, which is
/// generated from this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    /// Version of this contract, see [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// When the scan ran, in seconds since 1970-01-01 UTC.
    pub scanned_at: u64,
    /// The game that was scanned.
    pub game: GameInfo,
    /// How many script extender plugins were examined.
    pub extender_plugins_checked: u32,
    /// How many active plugins were read.
    #[serde(default)]
    pub plugins_checked: u32,
    /// How many archives were read.
    #[serde(default)]
    pub archives_checked: u32,
    /// How many mod installers were read.
    #[serde(default)]
    pub installers_checked: u32,
    /// How many replacement meshes were compared with the mesh they
    /// replace.
    #[serde(default)]
    pub meshes_checked: u32,
    /// The places the scan looked at. Mod Mallet does not search the disk:
    /// it reads these places and what lies below the game directory.
    #[serde(default)]
    pub locations: Vec<ScanLocation>,
    /// The findings, most severe first.
    pub findings: Vec<Finding>,
    /// What the mod manager knows about the installed mods.
    #[serde(default)]
    pub mods: Vec<ModInfo>,
    /// Where mods place objects, and where they collide.
    #[serde(default)]
    pub map: WorldMap,
    /// Whether Base Object Swapper is installed, the script extender plugin
    /// that hides objects for Mod Mallet.
    #[serde(default)]
    pub object_swapper: bool,
    /// The patch Mod Mallet wrote earlier, as it is in the game now, if
    /// there is one.
    #[serde(default)]
    pub installed_patch: Option<InstalledPatch>,
    /// The findings about two mods, gathered pair by pair.
    #[serde(default)]
    pub duels: Vec<crate::duel::ModDuel>,
}

/// Every version of one file that the setup has, as pictures for the
/// window: the copies in archives first, the one the game uses last.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    /// The file, below the data directory.
    pub path: String,
    /// Its versions, the one the game uses last.
    pub versions: Vec<PreviewVersion>,
}

/// One version of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewVersion {
    /// The archive it is in, or `None` for a loose file.
    pub archive: Option<String>,
    /// The mod it comes from, if the mod manager knows.
    pub provider: Option<ModRef>,
    /// The picture, a PNG as a `data:` address, if the file could be shown.
    pub image: Option<String>,
}

/// A picture of a worldspace from above, made from the terrain textures in
/// the user's own game, for drawing the map on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MapImage {
    /// The worldspace.
    pub world: String,
    /// The westmost cell column it shows.
    pub west: i32,
    /// The southmost cell row it shows.
    pub south: i32,
    /// How many cell columns it shows.
    pub columns: u32,
    /// How many cell rows it shows.
    pub rows: u32,
    /// The picture, a PNG as a `data:` address. Its top row is the north.
    pub image: String,
}

/// A patch of Mod Mallet that is in the game: the objects it hides, and
/// which mod brought it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPatch {
    /// The objects it hides.
    pub objects: Vec<ObjectRef>,
    /// The mod the file belongs to, if the mod manager knows it.
    pub provider: Option<ModRef>,
}

/// What a mod manager knows about one installed mod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    /// The mod.
    #[serde(rename = "mod")]
    pub mod_ref: ModRef,
    /// Whether the mod is enabled in the profile in use.
    pub enabled: bool,
    /// The installed version, as the mod's author numbers it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The newest version the mod manager knows of, if it is another one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newer_version: Option<String>,
    /// Who made the mod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// The page the mod was downloaded from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
}

/// Where mods place objects in the world, cell by cell, and where objects
/// of two mods collide.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorldMap {
    /// The mods (or plugins, where no mod is known) that place objects.
    /// Cells refer to them by their position in this list.
    pub owners: Vec<String>,
    /// The worldspaces in which mods place objects outside: first the one
    /// with the most cells where several mods meet, then by objects.
    pub worlds: Vec<MapWorld>,
    /// Every collision that a finding reports.
    pub spots: Vec<MapSpot>,
}

/// One worldspace on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MapWorld {
    /// The worldspace's internal name, as the game's console knows it.
    pub name: String,
    /// The cells in which mods place objects.
    pub cells: Vec<MapCell>,
    /// The shape of the land, where it is known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain: Option<MapTerrain>,
    /// The places of the world map: the game's own and those mods add,
    /// change or remove.
    #[serde(default)]
    pub markers: Vec<MapMarker>,
}

/// A place of the world map, such as a town or a cave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MapMarker {
    /// The name the game's map shows.
    pub name: String,
    /// What kind of place it is.
    pub kind: PlaceKind,
    /// Where it is now, in game units: east, north and height.
    pub position: [i32; 3],
    /// Whose place it is.
    pub origin: PlaceOrigin,
    /// The mod that added, changed or removed it, as a position in
    /// [`WorldMap::owners`]. Absent for the game's own places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<u32>,
    /// Where the game puts it, if a mod moved it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moved_from: Option<[i32; 3]>,
    /// What the game calls it, if a mod renamed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renamed_from: Option<String>,
    /// The command of the game's console that takes the player there.
    pub console: String,
}

/// Whose a place of the world map is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceOrigin {
    /// The game's own, unchanged.
    Game,
    /// Added by a mod.
    Added,
    /// The game's own, moved, renamed or made another kind by a mod.
    Changed,
    /// The game's own or another mod's, removed by a mod.
    Removed,
}

/// The kinds of places of the world map, grouped as the game's map shows
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceKind {
    /// A city, the seat of a hold, or a hold's palace.
    City,
    /// A town.
    Town,
    /// A small settlement.
    Settlement,
    /// A castle, a hold's keep or another stronghold of a lord.
    Castle,
    /// A cave.
    Cave,
    /// A camp, often of bandits.
    Camp,
    /// A fort.
    Fort,
    /// A Nordic ruin or barrow.
    NordicRuin,
    /// A Dwemer ruin.
    DwemerRuin,
    /// A shipwreck.
    Shipwreck,
    /// A grove.
    Grove,
    /// A landmark, such as a standing stone or a temple.
    Landmark,
    /// A dragon lair or a word wall on a peak.
    DragonLair,
    /// A farm.
    Farm,
    /// A wood mill.
    WoodMill,
    /// A mine.
    Mine,
    /// A camp of the Imperial Legion.
    ImperialCamp,
    /// A camp of the Stormcloaks.
    StormcloakCamp,
    /// A standing stone.
    Doomstone,
    /// A wheat mill.
    WheatMill,
    /// A smelter.
    Smelter,
    /// A stable.
    Stable,
    /// An Imperial watchtower.
    ImperialTower,
    /// A clearing.
    Clearing,
    /// A mountain pass.
    Pass,
    /// An altar or shrine to the Divines.
    Shrine,
    /// A lighthouse.
    Lighthouse,
    /// An Orc stronghold.
    OrcStronghold,
    /// A giant camp.
    GiantCamp,
    /// A shack.
    Shack,
    /// A Nordic tower.
    NordicTower,
    /// A Nordic dwelling.
    NordicDwelling,
    /// Docks.
    Docks,
    /// A shrine of a Daedric Prince.
    DaedricShrine,
    /// A passage to another land, such as the boat to Solstheim.
    Passage,
    /// A vault of Vault-Tec (the Fallout games).
    Vault,
    /// The ruins of a town or of city blocks (the Fallout games).
    Ruin,
    /// A factory or another industrial site.
    Factory,
    /// A station of the subway or the railway.
    Station,
    /// A building of offices, homes, a school, a hospital or the like (the
    /// Fallout games).
    Building,
    /// An Ayleid ruin (Oblivion).
    AyleidRuin,
    /// A gate to Oblivion (Oblivion).
    OblivionGate,
    /// An inn, a tavern or stables.
    Inn,
    /// A marker without a kind of its own.
    Other,
}

/// The shape of the land of a worldspace, as a small height image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MapTerrain {
    /// The westmost cell column the image covers.
    pub west: i32,
    /// The southmost cell row the image covers.
    pub south: i32,
    /// How many cells wide the image is.
    pub columns: u32,
    /// How many cells high the image is.
    pub rows: u32,
    /// How many samples each cell has along each side.
    pub samples: u32,
    /// Two bytes per sample (little endian), written in Base64: rows from
    /// north to south, samples from west to east. 0 means nothing is known
    /// there; 1 to 65535 is the height of the ground, or of the water
    /// surface where water covers it, from [`MapTerrain::lowest`] to
    /// [`MapTerrain::highest`].
    pub heights: String,
    /// One byte per sample, in the order of the heights, written in Base64:
    /// 1 where water covers the ground, 0 elsewhere. Empty in reports of
    /// versions that did not keep it.
    #[serde(default)]
    pub water: String,
    /// The lowest height, of ground or water, in game units.
    pub lowest: i32,
    /// The highest height, of ground or water, in game units.
    pub highest: i32,
}

/// One cell of a worldspace: 4096 game units, about 58 metres, square.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MapCell {
    /// The cell's column; east is larger.
    pub x: i32,
    /// The cell's row; north is larger.
    pub y: i32,
    /// How many objects mods place in the cell.
    pub objects: u32,
    /// Which mods place them, as positions in [`WorldMap::owners`].
    pub owners: Vec<u32>,
}

/// Where a collision of two mods' objects is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MapSpot {
    /// The finding that reports the collision.
    pub finding: FindingId,
    /// Where it is.
    pub location: SpotLocation,
    /// The objects involved, by their internal names.
    pub objects: String,
    /// The objects involved, one by one.
    #[serde(default)]
    pub parts: Vec<SpotPart>,
    /// The command of the game's console that takes the player there.
    pub console: String,
    /// The point between the objects involved, in game units: east, north
    /// and height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<[i32; 3]>,
    /// What kind of collision it is.
    #[serde(default)]
    pub kind: SpotKind,
    /// How far the two models reach into each other, in game units, where
    /// their meshes were compared; `None` where only their boxes were.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
}

/// What kind of collision a spot is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SpotKind {
    /// The same object placed twice in almost the same spot.
    Duplicate,
    /// Two objects standing largely inside each other.
    #[default]
    Overlap,
    /// Two objects whose models touch or cut a little into each other.
    Touch,
}

/// One object of a collision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpotPart {
    /// The object.
    pub object: ObjectRef,
    /// The internal name of what it is.
    pub name: String,
    /// Whether it can be hidden without harm: nothing refers to it from
    /// elsewhere, and the distant view is not built around it.
    pub hideable: bool,
    /// Whether hiding this one rather than the other is suggested: of two
    /// copies, the one from the plugin that places fewer objects.
    pub suggested: bool,
}

/// Where a spot is: outside in a worldspace, or in a closed space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SpotLocation {
    /// Outside.
    Exterior {
        /// The worldspace's internal name.
        world: String,
        /// The cell's column.
        x: i32,
        /// The cell's row.
        y: i32,
    },
    /// In a closed space such as a house or a cave.
    Interior {
        /// The cell's internal name.
        cell: String,
    },
}
