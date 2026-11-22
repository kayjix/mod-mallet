//! Plugins of mods the mod manager keeps switched off that the game loads
//! all the same (ADR 0061), and what such a mod left behind (ADR 0073).
//!
//! Switching a mod off in Vortex takes its files out of the game when
//! Vortex deploys. Until then, the game still loads them. And a copy that
//! was in the data directory before Vortex deployed the mod, which Vortex
//! set aside and put back when it took its own away, stays for good; Vortex
//! keeps such a plugin active. In both cases the mod is off in the mod
//! manager and on in the game. Its collisions are real and stay listed;
//! this finding says why they are still there, and what takes the mod out:
//! deploying, or switching the plugin left behind off in the mod manager's
//! list of plugins, which holds it although the manager did not install it.

use mallet_core::{Action, Finding, GamePath, Subject, SwitchedOffPlugin, kinds};

use crate::{Analysis, Context, examples};

/// How many plugins that build on it a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// A plugin of a switched-off mod the game loads, with its files there.
struct StillLoaded<'a> {
    plugin: &'a SwitchedOffPlugin,
    path: GamePath,
    /// Whether the mod manager itself still has the files in the game:
    /// switched off, but not deployed since.
    deployed: bool,
    /// The plugin and the archives of its name in the data directory.
    files: Vec<String>,
}

/// The plugins of switched-off mods that the game loads all the same.
fn still_loaded<'a>(context: &Context<'a>) -> Vec<StillLoaded<'a>> {
    context
        .facts
        .switched_off_plugins
        .iter()
        .filter(|plugin| plugin.loaded)
        .filter_map(|plugin| {
            let facts = context
                .facts
                .plugins
                .iter()
                .find(|active| active.name.eq_ignore_ascii_case(&plugin.plugin))?;
            let deployed = context.mods.provider_of(&facts.path).as_ref() == Some(&plugin.mod_ref);
            // What no mod manager put there, as the engine found it in the
            // data directory; else the archives the scan read.
            let files = if !deployed && !plugin.left_behind.is_empty() {
                plugin.left_behind.clone()
            } else {
                archives_of(context, plugin, &facts.name)
            };
            Some(StillLoaded {
                plugin,
                path: facts.path.clone(),
                deployed,
                files,
            })
        })
        .collect()
}

/// The plugin named `name` and the archives of its name the switched-off
/// mod brings, as the scan read them.
fn archives_of(context: &Context<'_>, plugin: &SwitchedOffPlugin, name: &str) -> Vec<String> {
    let ours = |path: &GamePath| match context.mods.provider_of(path) {
        Some(provider) => provider == plugin.mod_ref,
        None => context
            .mods
            .switched_off_with(path)
            .contains(&plugin.mod_ref),
    };
    let stem = plugin_stem(name).to_lowercase();
    let archives = context.facts.archives.iter().filter(|archive| {
        let archive_stem = plugin_stem(archive.path.file_name());
        (archive_stem == stem || archive_stem.starts_with(&format!("{stem} - ")))
            && ours(&archive.path)
    });
    std::iter::once(name.to_owned())
        .chain(archives.map(|archive| archive.path.file_name().to_owned()))
        .collect()
}

/// A file name without its ending.
fn plugin_stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// A count as a finding's argument.
fn number(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

/// Values one per line, as a finding carries a list for the window to take
/// apart.
fn lines<'a>(values: impl Iterator<Item = &'a str>) -> String {
    values.collect::<Vec<_>>().join("\n")
}

/// Reports plugins of switched-off mods that the game loads all the same.
pub struct SwitchedOffButLoaded;

impl Analysis for SwitchedOffButLoaded {
    fn id(&self) -> &'static str {
        "switched-off-but-loaded"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let plugins = &context.facts.plugins;
        still_loaded(context)
            .into_iter()
            .map(|found| {
                // Everything that would lose a master once the plugin is
                // out, directly or through another (ADR 0068).
                let position: Vec<usize> = plugins
                    .iter()
                    .position(|plugin| plugin.name.eq_ignore_ascii_case(&found.plugin.plugin))
                    .into_iter()
                    .collect();
                let stranded: Vec<_> = crate::dependencies::stranded_positions(plugins, &position)
                    .into_iter()
                    .filter_map(|at| plugins.get(at))
                    .map(|plugin| (plugin, context.mods.provider_of(&plugin.path)))
                    .collect();
                let dependents: Vec<&str> = stranded
                    .iter()
                    .map(|(plugin, _)| plugin.name.as_str())
                    .collect();
                let mut finding = Finding::new(
                    &kinds::PLUGINS_SWITCHED_OFF_LOADED,
                    Subject::File { path: found.path },
                )
                .with_arg("plugin", found.plugin.plugin.clone())
                .with_arg("mod", found.plugin.mod_ref.name.clone())
                .with_arg("deployed", i64::from(found.deployed))
                .with_arg("fileCount", number(found.files.len()))
                .with_arg("files", found.files.join(", "))
                .with_arg("fileList", lines(found.files.iter().map(String::as_str)))
                .with_arg("dependentCount", number(dependents.len()))
                .with_arg(
                    "dependents",
                    examples(dependents.iter().copied(), EXAMPLE_LIMIT),
                )
                // Every one of them, in load order, with its mod, for the
                // window to link to (ADR 0068).
                .with_arg("dependentPlugins", lines(dependents.iter().copied()))
                .with_arg(
                    "dependentModIds",
                    lines(stranded.iter().map(|(_, provider)| {
                        provider.as_ref().map_or("", |known| known.id.as_str())
                    })),
                );
                // A copy the mod manager did not put there goes out of the
                // game when it is switched off in the manager's list of
                // plugins, together with what builds on it; on the user's
                // click only (ADR 0020). Files the manager still has in the
                // game go when it deploys, which is one click there.
                if !found.deployed {
                    finding = finding.with_action(Action::DisablePlugins {
                        plugins: std::iter::once(found.plugin.plugin.clone())
                            .chain(dependents.iter().map(|name| (*name).to_owned()))
                            .collect(),
                    });
                }
                finding
            })
            .collect()
    }
}

/// Reports the files a switched-off mod left in the data directory once the
/// game no longer loads its plugin (ADR 0073). They do no harm there, but
/// the mod manager does not know them: switched on again, the mod is set
/// aside for them once more.
pub struct SwitchedOffLeftBehind;

impl Analysis for SwitchedOffLeftBehind {
    fn id(&self) -> &'static str {
        "switched-off-left-behind"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        context
            .facts
            .switched_off_plugins
            .iter()
            .filter(|plugin| !plugin.loaded && !plugin.left_behind.is_empty())
            .map(|plugin| {
                Finding::new(
                    &kinds::PLUGINS_SWITCHED_OFF_LEFT_BEHIND,
                    Subject::File {
                        path: GamePath::new("data").join(&plugin.plugin),
                    },
                )
                .with_arg("plugin", plugin.plugin.clone())
                .with_arg("mod", plugin.mod_ref.name.clone())
                .with_arg("fileCount", number(plugin.left_behind.len()))
                .with_arg("files", plugin.left_behind.join(", "))
                .with_arg(
                    "fileList",
                    lines(plugin.left_behind.iter().map(String::as_str)),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::objects::{BaseKind, BaseObject, Bounds, Place, PlacedObject};
    use mallet_core::facts::{ArchiveFacts, PluginFacts, RecordId};
    use mallet_core::{Facts, ModRef, ModSource};

    use super::*;

    fn citizens() -> ModRef {
        ModRef {
            source: "vortex".to_owned(),
            id: "Citizens 1".to_owned(),
            name: "Citizens".to_owned(),
        }
    }

    /// Vortex with the mod "Citizens" switched off. With `deployed`, Vortex
    /// has not deployed since and its files are still in the game; without,
    /// only the staging folder has them.
    struct Manager {
        deployed: bool,
    }

    impl ModSource for Manager {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            if path.as_str() == "data/town.esp" {
                return Some(ModRef {
                    source: "vortex".to_owned(),
                    id: "Town 2".to_owned(),
                    name: "Town".to_owned(),
                });
            }
            (self.deployed && path.as_str().starts_with("data/citizens.")).then(citizens)
        }

        fn switched_off_with(&self, path: &GamePath) -> Vec<ModRef> {
            if path.as_str().starts_with("data/citizens") {
                vec![citizens()]
            } else {
                Vec::new()
            }
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

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn house(plugin: &str, local_id: u32) -> PlacedObject {
        PlacedObject {
            id: id(plugin, local_id),
            plugin: plugin.to_owned(),
            base: id("skyrim.esm", 1),
            place: Place::Exterior {
                worldspace: id("skyrim.esm", 0x3C),
            },
            position: [0, 0, 0],
            rotation: [0, 0, 0],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        }
    }

    /// Two mods that place a house in the same spot; the plugin of
    /// "Citizens" is loaded or not, while the mod is switched off.
    fn facts(loaded: bool) -> Facts {
        let mut facts = Facts {
            plugins: vec![
                plugin("Skyrim.esm", &[]),
                plugin("Town.esp", &["Skyrim.esm"]),
                plugin("Citizens.esp", &["Skyrim.esm"]),
                plugin("Citizens Patch.esp", &["Citizens.esp"]),
                plugin("Lights.esp", &["Skyrim.esm", "Citizens Patch.esp"]),
            ],
            archives: vec![
                ArchiveFacts {
                    path: GamePath::new("data/Citizens.bsa"),
                    files: Vec::new(),
                },
                ArchiveFacts {
                    path: GamePath::new("data/Town.bsa"),
                    files: Vec::new(),
                },
            ],
            switched_off_plugins: vec![SwitchedOffPlugin {
                plugin: "Citizens.esp".to_owned(),
                mod_ref: citizens(),
                loaded,
                left_behind: vec!["Citizens.bsa".to_owned(), "Citizens.esp".to_owned()],
            }],
            ..Facts::default()
        };
        facts.objects.bases.push(BaseObject {
            name: None,
            model: None,
            id: id("skyrim.esm", 1),
            editor_id: Some("House".to_owned()),
            kind: BaseKind::Static,
            solid: true,
            natural: false,
            bounds: Bounds {
                min: [-500, -500, 0],
                max: [500, 500, 600],
            },
        });
        facts.objects.placed = vec![house("Town.esp", 1), house("Citizens.esp", 1)];
        if !loaded {
            facts.plugins.retain(|plugin| plugin.name != "Citizens.esp");
            facts
                .objects
                .placed
                .retain(|object| object.id.plugin != "citizens.esp");
        }
        facts
    }

    fn reported(facts: &Facts, manager: &Manager) -> Vec<Finding> {
        crate::run_all(&Context::new(facts, manager))
            .into_iter()
            .filter(|finding| finding.kind == kinds::PLUGINS_SWITCHED_OFF_LOADED.id)
            .collect()
    }

    #[test]
    fn a_copy_left_behind_keeps_the_mod_in_the_game_and_its_collisions_stay() {
        let facts = facts(true);
        let manager = Manager { deployed: false };
        // The game loads the plugin: what it collides with is real.
        assert_eq!(Context::new(&facts, &manager).collisions().spots.len(), 1);
        let [finding] = reported(&facts, &manager).try_into().unwrap();
        assert_eq!(finding.args.get("deployed"), Some(&0.into()));
        assert_eq!(
            finding.args.get("files"),
            Some(&"Citizens.bsa, Citizens.esp".into())
        );
        // What builds on it, also through another plugin, goes with it.
        assert_eq!(
            finding.args.get("dependentPlugins"),
            Some(&"Citizens Patch.esp\nLights.esp".into())
        );
        assert_eq!(finding.args.get("mod"), Some(&"Citizens".into()));
        assert_eq!(
            finding.actions,
            [Action::DisablePlugins {
                plugins: vec![
                    "Citizens.esp".to_owned(),
                    "Citizens Patch.esp".to_owned(),
                    "Lights.esp".to_owned(),
                ],
            }]
        );
    }

    #[test]
    fn a_mod_switched_off_but_not_deployed_since_is_still_in_the_game() {
        let facts = facts(true);
        let [finding] = reported(&facts, &Manager { deployed: true })
            .try_into()
            .unwrap();
        assert_eq!(finding.args.get("deployed"), Some(&1.into()));
        // What the mod manager still has in the game goes when it deploys.
        assert_eq!(
            finding.args.get("files"),
            Some(&"Citizens.esp, citizens.bsa".into())
        );
        assert_eq!(finding.actions, []);
    }

    #[test]
    fn a_switched_off_mod_the_game_does_not_load_is_not_reported() {
        let facts = facts(false);
        let manager = Manager { deployed: false };
        assert_eq!(reported(&facts, &manager), []);
        assert_eq!(Context::new(&facts, &manager).collisions().spots, []);
    }

    #[test]
    fn files_left_behind_by_a_mod_the_game_no_longer_loads_are_a_note() {
        let facts = facts(false);
        let manager = Manager { deployed: false };
        let [finding] = crate::run_all(&Context::new(&facts, &manager))
            .into_iter()
            .filter(|finding| finding.kind == kinds::PLUGINS_SWITCHED_OFF_LEFT_BEHIND.id)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        assert_eq!(
            finding.args.get("fileList"),
            Some(&"Citizens.bsa\nCitizens.esp".into())
        );
        assert_eq!(finding.args.get("mod"), Some(&"Citizens".into()));
    }
}
