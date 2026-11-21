//! Which plugins need which: the load order with every plugin's masters,
//! and what would be left without a master if a mod went (ADR 0068).
//!
//! A plugin names the plugins it builds on, its masters. The game closes
//! while starting when a master is not loaded, so a mod cannot simply go
//! when other plugins need one of its plugins: those would have to go too,
//! and whatever needs them in turn.

use std::collections::{HashMap, HashSet, VecDeque};

use mallet_core::facts::PluginFacts;
use mallet_core::{LoadedPlugin, ModRef, WorldOrigin};

use crate::Context;

/// The active plugins in load order, each with its masters and the mod it
/// comes from. `origin` tells whether a plugin, by file name, comes with
/// the game, with its Creation Club content or with a mod.
pub fn load_order(
    context: &Context<'_>,
    origin: &dyn Fn(&str) -> WorldOrigin,
) -> Vec<LoadedPlugin> {
    context
        .facts
        .plugins
        .iter()
        .map(|plugin| LoadedPlugin {
            name: plugin.name.clone(),
            origin: origin(&plugin.name),
            masters: plugin.masters.clone(),
            provider: context.mods.provider_of(&plugin.path),
        })
        .collect()
}

/// An active plugin that would lose a master.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stranded {
    /// The plugin's file name in the spelling of the load order.
    pub plugin: String,
    /// The mod it comes from, if the mod manager knows.
    pub provider: Option<ModRef>,
    /// The masters it would lose, as it names them: plugins that went, or
    /// that would have to go because they lose a master themselves.
    pub lost: Vec<String>,
}

/// The positions in `plugins` of the plugins that would lose a master if
/// those at `gone` were no longer loaded: the plugins that need one of
/// them, those that need one of those, and so on. In load order; the
/// plugins at `gone` are not among them.
pub fn stranded_positions(plugins: &[PluginFacts], gone: &[usize]) -> Vec<usize> {
    // Who needs each plugin, by its name in lower case.
    let mut needed_by: HashMap<String, Vec<usize>> = HashMap::new();
    for (position, plugin) in plugins.iter().enumerate() {
        for master in &plugin.masters {
            needed_by
                .entry(master.to_lowercase())
                .or_default()
                .push(position);
        }
    }

    let mut lost: HashSet<usize> = gone.iter().copied().collect();
    let mut pending: VecDeque<usize> = gone.iter().copied().collect();
    let mut stranded = Vec::new();
    while let Some(position) = pending.pop_front() {
        let Some(plugin) = plugins.get(position) else {
            continue;
        };
        for &dependent in needed_by
            .get(&plugin.name.to_lowercase())
            .map_or(&[][..], Vec::as_slice)
        {
            if lost.insert(dependent) {
                stranded.push(dependent);
                pending.push_back(dependent);
            }
        }
    }
    stranded.sort_unstable();
    stranded
}

/// The active plugins that would lose a master if the mod `removed` went,
/// directly or because a plugin they need would have to go too. In load
/// order; the plugins of `removed` itself are not among them.
///
/// This is what a recommendation to remove a mod names: the plugins that
/// would have to go with it.
pub fn stranded_by(context: &Context<'_>, removed: &ModRef) -> Vec<Stranded> {
    let plugins = &context.facts.plugins;
    let providers: Vec<Option<ModRef>> = plugins
        .iter()
        .map(|plugin| context.mods.provider_of(&plugin.path))
        .collect();
    let gone: Vec<usize> = providers
        .iter()
        .enumerate()
        .filter(|(_, provider)| {
            provider
                .as_ref()
                .is_some_and(|known| known.source == removed.source && known.id == removed.id)
        })
        .map(|(position, _)| position)
        .collect();
    let stranded = stranded_positions(plugins, &gone);

    let lost_names: HashSet<String> = gone
        .iter()
        .chain(&stranded)
        .filter_map(|&position| plugins.get(position))
        .map(|plugin| plugin.name.to_lowercase())
        .collect();
    stranded
        .into_iter()
        .filter_map(|position| {
            let plugin = plugins.get(position)?;
            Some(Stranded {
                plugin: plugin.name.clone(),
                provider: providers.get(position).cloned().flatten(),
                lost: plugin
                    .masters
                    .iter()
                    .filter(|master| lost_names.contains(&master.to_lowercase()))
                    .cloned()
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use mallet_core::{Facts, GamePath, ModSource};

    use super::*;

    /// Every plugin belongs to the mod named, in lower case, before ` - `
    /// in its name, or else like the plugin; the game's own plugins to no
    /// mod.
    struct ByName;

    impl ModSource for ByName {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            let name = path.file_name();
            if is_master(name) {
                return None;
            }
            let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
            let mod_name = stem
                .split_once(" - ")
                .map_or(stem, |(mod_name, _)| mod_name);
            Some(mod_ref(mod_name))
        }
    }

    /// Whether a test plugin is one of the game's, by its extension.
    fn is_master(name: &str) -> bool {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("esm"))
    }

    fn mod_ref(name: &str) -> ModRef {
        ModRef {
            source: "test".to_owned(),
            id: name.to_owned(),
            name: name.to_owned(),
        }
    }

    fn plugin(name: &str, masters: &[&str]) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: masters.iter().map(|master| (*master).to_owned()).collect(),
            npcs: Vec::new(),
        }
    }

    fn setup() -> Facts {
        Facts {
            plugins: vec![
                plugin("Skyrim.esm", &[]),
                plugin("Floor.esp", &["Skyrim.esm"]),
                plugin("Floor - Extra.esp", &["Skyrim.esm", "Floor.esp"]),
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Town Floor Patch.esp", &["Town.esp", "floor.esp"]),
                plugin("Town Floor Patch Fix.esp", &["Town Floor Patch.esp"]),
                plugin("Lights.esp", &["Skyrim.esm"]),
            ],
            ..Facts::default()
        }
    }

    #[test]
    fn the_load_order_carries_masters_mods_and_origins() {
        let facts = setup();
        let context = Context::new(&facts, &ByName);
        let order = load_order(&context, &|name| {
            if is_master(name) {
                WorldOrigin::Game
            } else {
                WorldOrigin::Mod
            }
        });

        assert_eq!(order.len(), 7);
        assert_eq!(order[0].name, "Skyrim.esm");
        assert_eq!(order[0].origin, WorldOrigin::Game);
        assert_eq!(order[0].provider, None);
        assert_eq!(order[2].masters, vec!["Skyrim.esm", "Floor.esp"]);
        assert_eq!(order[2].provider, Some(mod_ref("floor")));
    }

    #[test]
    fn what_needs_a_mod_goes_with_it_transitively_and_case_blind() {
        let facts = setup();
        let context = Context::new(&facts, &ByName);
        let stranded = stranded_by(&context, &mod_ref("floor"));

        let names: Vec<&str> = stranded.iter().map(|s| s.plugin.as_str()).collect();
        assert_eq!(names, ["Town Floor Patch.esp", "Town Floor Patch Fix.esp"]);
        assert_eq!(stranded[0].lost, ["floor.esp"]);
        assert_eq!(stranded[0].provider, Some(mod_ref("town floor patch")));
        assert_eq!(stranded[1].lost, ["Town Floor Patch.esp"]);
    }

    #[test]
    fn a_mod_nothing_needs_strands_nothing() {
        let facts = setup();
        let context = Context::new(&facts, &ByName);
        assert_eq!(stranded_by(&context, &mod_ref("lights")), Vec::new());
        assert_eq!(stranded_by(&context, &mod_ref("Not installed")), Vec::new());
    }

    #[test]
    fn masters_loaded_later_and_cycles_end() {
        let plugins = vec![
            plugin("A.esp", &["B.esp"]),
            plugin("B.esp", &["A.esp"]),
            plugin("C.esp", &["B.esp"]),
        ];
        assert_eq!(stranded_positions(&plugins, &[0]), vec![1, 2]);
        assert_eq!(stranded_positions(&plugins, &[9]), Vec::<usize>::new());
    }
}
