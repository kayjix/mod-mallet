//! Why the game relies on placed objects, and whether they can be hidden
//! all the same (ADR 0046). Only reads.

use std::sync::{Arc, Mutex};

use mallet_core::ObjectRef;
use mallet_core::anchors::{AnchorVerdict, ObjectAnchors};
use mallet_core::facts::RecordId;
use mallet_game_bethesda::anchors::AnchorReader;

use crate::ScanOptions;

/// The pointers between the records of the setup last asked about, by
/// game and folder. Reading them takes a few seconds, so they are kept.
static READER: Mutex<Option<(String, Arc<Mutex<AnchorReader>>)>> = Mutex::new(None);

fn reader(options: &ScanOptions) -> Option<Arc<Mutex<AnchorReader>>> {
    let (game, game_dir) = options.known_game()?;
    let key = format!("{}|{}", game.id(), game_dir.display());
    let mut kept = READER.lock().ok()?;
    if let Some((known, reader)) = kept.as_ref()
        && *known == key
    {
        return Some(Arc::clone(reader));
    }
    let reader = Arc::new(Mutex::new(AnchorReader::open(
        game,
        &game_dir,
        options.local_dir.as_deref(),
    )));
    *kept = Some((key, Arc::clone(&reader)));
    Some(reader)
}

/// What ties each of `objects` to the game, and whether it can be hidden,
/// in the same order. `world_or_cell` is the worldspace the objects are
/// in, by internal name such as `Tamriel`, or the interior cell. The first
/// call for a setup reads every plugin, which takes a few seconds; asking
/// with no objects does only that. `None` if the setup cannot be read.
pub fn object_anchors(
    options: &ScanOptions,
    world_or_cell: &str,
    objects: &[ObjectRef],
) -> Option<Vec<ObjectAnchors>> {
    let started = std::time::Instant::now();
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let ids: Vec<RecordId> = objects
        .iter()
        .map(|object| RecordId {
            plugin: object.plugin.clone(),
            local_id: object.local_id,
        })
        .collect();
    let found = reader.anchors(world_or_cell, &ids);
    let answers: Vec<ObjectAnchors> = objects
        .iter()
        .zip(found.into_iter().chain(std::iter::repeat_with(Vec::new)))
        .map(|(object, reasons)| ObjectAnchors {
            object: object.clone(),
            verdict: AnchorVerdict::of(&reasons),
            reasons,
        })
        .collect();
    tracing::info!(
        world_or_cell,
        objects = answers.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "found what ties objects to the game"
    );
    Some(answers)
}
