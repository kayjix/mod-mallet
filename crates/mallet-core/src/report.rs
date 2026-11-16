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
    /// One byte per sample, written in Base64: rows from north to south,
    /// samples from west to east. 0 means no land is known there, 1 water,
    /// and 2 to 255 land from [`MapTerrain::lowest`] to
    /// [`MapTerrain::highest`].
    pub heights: String,
    /// The height of the lowest land, in game units.
    pub lowest: i32,
    /// The height of the highest land, in game units.
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
