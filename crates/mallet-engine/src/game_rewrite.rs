//! Whether the game itself last wrote the list of active plugins, and what
//! it switched off (ADR 0089). Only reads.

use std::path::Path;

use mallet_core::facts::PluginFacts;
use mallet_core::facts::plugin_list::GameRewrite;
use mallet_core::{ModSource, SwitchedOffPlugin};
use mallet_game_bethesda::Game;
use mallet_game_bethesda::ingest::plugin_list_path;
use mallet_game_bethesda::plugin_list::{PluginList, Writer, game_rewrite, text_of};

/// The file in which Vortex keeps every plugin in its order, next to the
/// list of active plugins.
const LOAD_ORDER_FILE: &str = "loadorder.txt";

/// The plugins the game switched off when it last started, if it wrote the
/// list of active plugins itself; `None` where it did not, or switched
/// nothing off that the mod manager had. `active` are the plugins the game
/// loads now, `mods` tells which mod brings a file and `switched_off` are
/// the plugins of mods the manager keeps off.
pub(crate) fn read(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    active: &[PluginFacts],
    mods: &dyn ModSource,
    switched_off: &[SwitchedOffPlugin],
) -> Option<GameRewrite> {
    let path = plugin_list_path(game, game_dir, local_dir)?;
    let list = PluginList::parse(&text_of(&std::fs::read(&path).ok()?), true);
    if list.writer != Writer::Game {
        return None;
    }
    // Vortex's own order, which still lists what the game switched off.
    let order = path
        .parent()
        .map(|folder| folder.join(LOAD_ORDER_FILE))
        .and_then(|file| std::fs::read(file).ok())
        .map(|bytes| PluginList::parse(&text_of(&bytes), false))
        .filter(|order| order.writer == Writer::Vortex);
    let names: Vec<String> = active.iter().map(|plugin| plugin.name.clone()).collect();
    let found = game_rewrite(
        game,
        &game_dir.join("Data"),
        &list,
        order.as_ref(),
        &names,
        mods,
        switched_off,
    );
    tracing::info!(
        list = %path.display(),
        manager_order = order.is_some(),
        switched_off = ?found.iter().map(|plugin| plugin.plugin.as_str()).collect::<Vec<_>>(),
        "the game wrote the list of active plugins itself"
    );
    if found.is_empty() {
        return None;
    }
    Some(GameRewrite {
        game: game.name().to_owned(),
        manager: order.is_some().then(|| "vortex".to_owned()),
        switched_off: found,
    })
}
