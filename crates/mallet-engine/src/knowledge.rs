//! Reading LOOT's masterlist for a scan (ADR 0030).

use std::path::{Path, PathBuf};
use std::time::Instant;

use mallet_core::facts::{KnowledgeFacts, PluginFacts};
use mallet_core::{LocationOrigin, LocationRole, ScanLocation};

use crate::ScanOptions;

/// The masterlist a scan with `options` reads, and how it was found.
///
/// Found automatically only if the mod manager's state is, too: whoever
/// names the state wants a scan of exactly the files they named, not one
/// that also takes rules from this machine.
pub(crate) fn planned_masterlist(options: &ScanOptions) -> Option<(PathBuf, LocationOrigin)> {
    match (&options.masterlist, &options.manager_state) {
        (Some(given), _) => Some((given.clone(), LocationOrigin::Given)),
        (None, None) => {
            mallet_knowledge::find_masterlist().map(|path| (path, LocationOrigin::Standard))
        }
        (None, Some(_)) => None,
    }
}

/// What the masterlist says about the active `plugins`. Without a
/// masterlist, or if it cannot be read, nothing is known and the scan goes
/// on; the reason is logged.
pub(crate) fn read(
    options: &ScanOptions,
    game_dir: &Path,
    plugins: &[PluginFacts],
    locations: &mut Vec<ScanLocation>,
) -> KnowledgeFacts {
    let Some((path, origin)) = planned_masterlist(options) else {
        tracing::info!("no LOOT masterlist found; its rules are not checked");
        return KnowledgeFacts::default();
    };
    let found = path.is_file();
    locations.push(ScanLocation {
        role: LocationRole::Masterlist,
        path: path.display().to_string(),
        origin,
        found,
    });
    if !found || plugins.is_empty() {
        return KnowledgeFacts::default();
    }

    let started = Instant::now();
    match mallet_knowledge::read(
        game_dir,
        options.local_dir.as_deref(),
        &path,
        plugins,
        &mallet_game_bethesda::locate::shipped_plugins(game_dir),
    ) {
        Ok(knowledge) => {
            tracing::info!(
                plugins = knowledge.plugins.len(),
                elapsed_ms = started.elapsed().as_millis(),
                "read LOOT's rules"
            );
            knowledge
        }
        Err(error) => {
            tracing::warn!(%error, "LOOT's rules could not be read");
            KnowledgeFacts::default()
        }
    }
}
