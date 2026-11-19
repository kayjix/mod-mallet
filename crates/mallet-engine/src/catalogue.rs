//! The things of the setup one can look at, for the *Assets* page
//! (ADR 0053). Only reads.

use std::sync::{Arc, Mutex};

use mallet_core::GamePath;
use mallet_core::catalogue::Catalogue;
use mallet_game_bethesda::catalogue::read_catalogue;

use crate::ScanOptions;

/// The catalogue last read, by game, folder, language and the state of the
/// setup. Reading it takes a moment, so it is kept while the setup stays
/// the same.
static KEPT: Mutex<Option<(String, Arc<Catalogue>)>> = Mutex::new(None);

/// Every thing of the setup worth looking at, with names in the language
/// `lang` (`en`, `de`) where the game has them, and the mod that brings
/// each plugin. `None` if the setup cannot be read.
pub fn catalogue(options: &ScanOptions, lang: &str) -> Option<Arc<Catalogue>> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let key = format!(
        "{}|{}|{lang}|{}",
        game.id(),
        game_dir.display(),
        crate::setup_fingerprint(options)
    );
    if let Ok(kept) = KEPT.lock()
        && let Some((known, catalogue)) = kept.as_ref()
        && *known == key
    {
        return Some(Arc::clone(catalogue));
    }
    let started = std::time::Instant::now();
    let mut catalogue = read_catalogue(
        game,
        &game_dir,
        options.local_dir.as_deref(),
        game_language(lang),
    );
    let mods = crate::preview::deployment(options, &game_dir);
    for source in &mut catalogue.sources {
        source.mod_name = mods
            .provider_of(&GamePath::new(&format!("data/{}", source.plugin)))
            .map(|found| found.name);
    }
    tracing::info!(
        items = catalogue.items.len(),
        outfits = catalogue.outfits.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the catalogue"
    );
    let catalogue = Arc::new(catalogue);
    if let Ok(mut kept) = KEPT.lock() {
        *kept = Some((key, Arc::clone(&catalogue)));
    }
    Some(catalogue)
}

/// The name the game gives the language `lang` in its string tables.
fn game_language(lang: &str) -> &'static str {
    match lang {
        "de" => "german",
        "fr" => "french",
        "es" => "spanish",
        "it" => "italian",
        "pl" => "polish",
        "ru" => "russian",
        "ja" => "japanese",
        _ => "english",
    }
}
