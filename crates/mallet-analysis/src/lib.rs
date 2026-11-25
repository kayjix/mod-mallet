//! Mod Mallet's analyses.
//!
//! An analysis looks at the [`Facts`] of a setup and reports [`Finding`]s.
//! Analyses are pure: they depend on `mallet-core` only and never read a
//! file, start a process or use the network (ADR 0004). That keeps them easy
//! to test with hand-written facts, and it is why this crate may not depend
//! on any game or mod manager adapter.
//!
//! To add an analysis: implement [`Analysis`] in a new module and list it in
//! [`all`].

/// The finding with the action of switching its mod off, if the mod is
/// known: what the user can do when a mod breaks the game and no fixed
/// version is at hand.
pub(crate) fn offer_disabling(finding: mallet_core::Finding) -> mallet_core::Finding {
    match finding.provider.clone() {
        Some(mod_ref) => finding.with_action(mallet_core::Action::DisableMod { mod_ref }),
        None => finding,
    }
}

pub mod assets;
mod contact;
pub mod crashes;
pub mod dependencies;
pub mod doors;
pub mod duels;
pub mod extender;
pub mod faces;
pub mod foreign_meshes;
pub mod generated;
mod geometry;
pub mod installer;
pub mod knowledge;
pub mod map;
pub mod masters;
pub mod meshes;
pub mod missing_textures;
pub mod navmesh_repair;
pub mod navmeshes;
pub mod objects;
pub mod path_repair;
pub mod paths_origin;
mod places;
pub mod reachability;
pub mod recommendations;
pub mod setup;
pub mod switched_off;
mod tin;
pub mod versions;
pub mod walkways;

use std::cell::OnceCell;

use mallet_core::{Facts, Finding, ModSource};

pub use assets::{AssetIndex, AssetSource};

/// What an analysis may look at.
pub struct Context<'a> {
    /// The facts gathered about the setup.
    pub facts: &'a Facts,
    /// The mod manager, for attributing files to mods.
    pub mods: &'a dyn ModSource,
    /// Which loose file or archive the game takes each asset from.
    pub assets: AssetIndex<'a>,
    /// The comparison of placed objects, made when first asked for.
    collisions: OnceCell<objects::Collisions>,
    /// The comparison of doors, made when first asked for.
    doors: OnceCell<doors::DoorGroups>,
}

impl<'a> Context<'a> {
    /// Prepares the context for a set of facts.
    pub fn new(facts: &'a Facts, mods: &'a dyn ModSource) -> Self {
        Self {
            facts,
            mods,
            assets: AssetIndex::new(facts),
            collisions: OnceCell::new(),
            doors: OnceCell::new(),
        }
    }

    /// Where objects of different mods collide. The comparison takes a
    /// moment, so it is made once and shared by the analysis that reports
    /// it and the map that shows it.
    pub fn collisions(&self) -> &objects::Collisions {
        self.collisions.get_or_init(|| objects::compute(self))
    }

    /// Where doors stand on top of each other (ADR 0067), shared by the
    /// analysis that reports it, the map and the comparison of objects.
    pub fn doors(&self) -> &doors::DoorGroups {
        self.doors.get_or_init(|| doors::compute(self))
    }
}

/// One self-contained check of a setup.
pub trait Analysis {
    /// A short stable name, used in logs.
    fn id(&self) -> &'static str;

    /// Reports every problem this analysis sees. An analysis that lacks the
    /// facts it needs reports nothing.
    fn run(&self, context: &Context<'_>) -> Vec<Finding>;
}

/// Every analysis Mod Mallet runs.
pub fn all() -> Vec<Box<dyn Analysis>> {
    vec![
        Box::new(setup::SetupProblems),
        Box::new(extender::ExtenderPlugins),
        Box::new(versions::VersionChecks),
        Box::new(masters::MissingMasters),
        Box::new(generated::GeneratedPlugins),
        Box::new(knowledge::KnownRules),
        Box::new(faces::NpcFaces),
        Box::new(installer::InstallerOptions),
        Box::new(foreign_meshes::ForeignMeshes),
        Box::new(meshes::BypassedRetextures),
        Box::new(missing_textures::MissingTextures),
        Box::new(objects::PlacedObjects),
        Box::new(doors::StackedDoors),
        Box::new(navmeshes::NavmeshConflicts),
        Box::new(assets::ArchiveOverrides),
        Box::new(crashes::RecentCrashes),
        Box::new(recommendations::WideCollisions),
        Box::new(recommendations::MostlyOverridden),
        Box::new(recommendations::DuplicateMods),
        Box::new(walkways::HiddenWalkways),
        Box::new(switched_off::SwitchedOffButLoaded),
        Box::new(switched_off::SwitchedOffLeftBehind),
    ]
}

/// Runs every analysis and returns the findings in report order: most
/// severe first, then by identity so the order is the same on every run.
pub fn run_all(context: &Context<'_>) -> Vec<Finding> {
    let mut findings: Vec<Finding> = all()
        .iter()
        .flat_map(|analysis| analysis.run(context))
        .collect();
    findings
        .sort_by(|a, b| (a.severity, a.confidence, &a.id).cmp(&(b.severity, b.confidence, &b.id)));
    findings
}

/// Whether the game takes the file at `path` from where it was read: the
/// loose file if `archive` is `None`, that archive otherwise. A file in an
/// archive that a loose file or a later archive covers is never used.
fn is_used(
    context: &Context<'_>,
    path: &mallet_core::GamePath,
    archive: Option<&mallet_core::GamePath>,
) -> bool {
    let Some(archive) = archive else {
        return true;
    };
    match context.assets.source(path) {
        Some(AssetSource::Archive(index)) => context
            .facts
            .archives
            .get(index)
            .is_some_and(|used| used.path == *archive),
        _ => false,
    }
}

/// Up to `limit` items joined for display, in a stable order.
fn examples<'a>(items: impl IntoIterator<Item = &'a str>, limit: usize) -> String {
    let mut items: Vec<&str> = items.into_iter().collect();
    items.sort_unstable();
    items.dedup();
    items.truncate(limit);
    items.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn examples_are_sorted_unique_and_limited() {
        assert_eq!(examples(["c", "a", "b", "a"], 2), "a, b");
        assert_eq!(examples([], 3), "");
    }
}
