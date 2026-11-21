//! Plugins of mods the mod manager keeps switched off that the game loads
//! all the same (ADR 0061).
//!
//! Switching a mod off in Vortex takes its files out of the game when
//! Vortex deploys. Until then, the game still loads them. And a copy that
//! was in the data directory before Vortex deployed the mod, which Vortex
//! set aside and put back when it took its own away, stays for good; Vortex
//! keeps such a plugin active. In both cases the mod is off in the mod
//! manager and on in the game. Its collisions are real and stay listed;
//! this finding says why they are still there, and what takes the mod out.

use mallet_core::{Finding, GamePath, Subject, SwitchedOffPlugin, kinds};

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
    /// The plugin and the archives of its name the mod brings.
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
            let ours = |path: &GamePath| match context.mods.provider_of(path) {
                Some(provider) => provider == plugin.mod_ref,
                None => context
                    .mods
                    .switched_off_with(path)
                    .contains(&plugin.mod_ref),
            };
            let stem = plugin_stem(&facts.name).to_lowercase();
            let archives = context.facts.archives.iter().filter(|archive| {
                let archive_stem = plugin_stem(archive.path.file_name());
                (archive_stem == stem || archive_stem.starts_with(&format!("{stem} - ")))
                    && ours(&archive.path)
            });
            let files = std::iter::once(facts.name.clone())
                .chain(archives.map(|archive| archive.path.file_name().to_owned()))
                .collect();
            Some(StillLoaded {
                plugin,
                path: facts.path.clone(),
                deployed: context.mods.provider_of(&facts.path).as_ref() == Some(&plugin.mod_ref),
                files,
            })
        })
        .collect()
}

/// A file name without its ending.
fn plugin_stem(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// Reports plugins of switched-off mods that the game loads all the same.
pub struct SwitchedOffButLoaded;

impl Analysis for SwitchedOffButLoaded {
    fn id(&self) -> &'static str {
        "switched-off-but-loaded"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        still_loaded(context)
            .into_iter()
            .map(|found| {
                let lower = found.plugin.plugin.to_lowercase();
                let dependents: Vec<&str> = context
                    .facts
                    .plugins
                    .iter()
                    .filter(|other| {
                        other
                            .masters
                            .iter()
                            .any(|master| master.to_lowercase() == lower)
                    })
                    .map(|other| other.name.as_str())
                    .collect();
                let number = |count: usize| i64::try_from(count).unwrap_or(i64::MAX);
                Finding::new(
                    &kinds::PLUGINS_SWITCHED_OFF_LOADED,
                    Subject::File { path: found.path },
                )
                .with_arg("plugin", found.plugin.plugin.clone())
                .with_arg("mod", found.plugin.mod_ref.name.clone())
                .with_arg("deployed", i64::from(found.deployed))
                .with_arg("fileCount", number(found.files.len()))
                .with_arg("files", found.files.join(", "))
                .with_arg("dependentCount", number(dependents.len()))
                .with_arg(
                    "dependents",
                    examples(dependents.iter().copied(), EXAMPLE_LIMIT),
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
            Some(&"Citizens.esp, citizens.bsa".into())
        );
        assert_eq!(
            finding.args.get("dependents"),
            Some(&"Citizens Patch.esp".into())
        );
        assert_eq!(finding.args.get("mod"), Some(&"Citizens".into()));
    }

    #[test]
    fn a_mod_switched_off_but_not_deployed_since_is_still_in_the_game() {
        let facts = facts(true);
        let [finding] = reported(&facts, &Manager { deployed: true })
            .try_into()
            .unwrap();
        assert_eq!(finding.args.get("deployed"), Some(&1.into()));
    }

    #[test]
    fn a_switched_off_mod_the_game_does_not_load_is_not_reported() {
        let facts = facts(false);
        let manager = Manager { deployed: false };
        assert_eq!(reported(&facts, &manager), []);
        assert_eq!(Context::new(&facts, &manager).collisions().spots, []);
    }
}
