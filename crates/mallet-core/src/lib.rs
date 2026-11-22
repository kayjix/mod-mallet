//! Game-neutral domain model of Mod Mallet.
//!
//! Everything else in the workspace speaks in the types defined here:
//!
//! * [`Facts`] are what the ingest stage learned about a setup.
//! * A [`Finding`] is one problem an analysis derived from those facts.
//! * A [`ScanReport`] is the result handed to the command line and the
//!   desktop app.
//! * An [`Action`] is something that can be done about a finding, by the
//!   tool that owns the change.
//! * [`ModSource`] is the seam behind which a mod manager lives.
//!
//! This crate knows no game and no mod manager, and it performs no file,
//! process or network access (ADR 0004 and ADR 0010). Keeping it that way
//! is what lets adapters for further games and managers be added later
//! without touching the analyses.

pub mod action;
pub mod action_places;
pub mod anchors;
pub mod area_paths;
pub mod cache;
pub mod catalogue;
pub mod compare;
pub mod doors;
pub mod duel;
pub mod encoding;
pub mod facts;
pub mod finding;
pub mod kinds;
pub mod managers;
pub mod mesh;
pub mod mods;
pub mod moves;
pub mod patch;
pub mod path;
pub mod progress;
pub mod reach;
pub mod report;
pub mod toolbox;
pub mod unreal;
pub mod versions;
pub mod walkway_repair;

pub use action::{Action, ActionOutcome, ActionStatus, ObjectRef, OptionToTick};
pub use cache::{NoCache, TextCache};
pub use doors::{DoorDestination, DoorGroup, DoorMerge, RecordChange, StackedDoor};
pub use facts::Facts;
pub use finding::{ArgValue, Confidence, Finding, FindingId, Party, Severity, Subject};
pub use kinds::KindInfo;
pub use managers::{KnownInstance, KnownManager, ManagerChoice};
pub use mods::{ModRef, ModSource, NoModSource, SwitchedOffPlugin};
pub use patch::{
    BookPatch, DEFAULT_PATCH_NAME, LoadedPatch, PATCH_MANIFEST_FILE, PATCH_MANIFEST_FORMAT, Patch,
    PatchBook, PatchInGame, PatchManifest, PatchPart,
};
pub use path::GamePath;
pub use progress::{Progress, ScanProgress, ScanStep};
pub use report::{
    FilePreview, GameChoice, GameInfo, GameLack, GameSupport, InstalledPatch, KnownGame,
    LoadedPlugin, LocationOrigin, LocationRole, MapBounds, MapCell, MapImage, MapMarker,
    MapPassage, MapSpot, MapTerrain, MapWorld, ModInfo, PassageSeverity, PlaceKind, PlaceOrigin,
    PreviewVersion, ScanLocation, ScanReport, SpotKind, SpotLocation, SpotPart, WorldMap,
    WorldOrigin,
};
pub use toolbox::{Tool, ToolGroup, ToolNeed, ToolState};
pub use unreal::{PakFolder, UnrealPak};
pub use versions::{
    ModStep, PluginStep, PluginVersion, VersionChanges, VersionSnapshot, VersionStep,
};
