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
pub mod anchors;
pub mod cache;
pub mod catalogue;
pub mod duel;
pub mod encoding;
pub mod facts;
pub mod finding;
pub mod kinds;
pub mod mesh;
pub mod mods;
pub mod patch;
pub mod path;
pub mod progress;
pub mod reach;
pub mod report;
pub mod toolbox;
pub mod unreal;

pub use action::{Action, ActionOutcome, ActionStatus, ObjectRef, OptionToTick};
pub use cache::{NoCache, TextCache};
pub use facts::Facts;
pub use finding::{ArgValue, Confidence, Finding, FindingId, Party, Severity, Subject};
pub use kinds::KindInfo;
pub use mods::{ModRef, ModSource, NoModSource};
pub use patch::{DEFAULT_PATCH_NAME, Patch, PatchInGame, PatchPart};
pub use path::GamePath;
pub use progress::{Progress, ScanProgress, ScanStep};
pub use report::{
    FilePreview, GameChoice, GameInfo, GameSupport, InstalledPatch, KnownGame, LocationOrigin,
    LocationRole, MapBounds, MapCell, MapImage, MapMarker, MapSpot, MapTerrain, MapWorld, ModInfo,
    PlaceKind, PlaceOrigin, PreviewVersion, ScanLocation, ScanReport, SpotKind, SpotLocation,
    SpotPart, WorldMap, WorldOrigin,
};
pub use toolbox::{Tool, ToolGroup, ToolNeed, ToolState};
pub use unreal::{PakFolder, UnrealPak};
