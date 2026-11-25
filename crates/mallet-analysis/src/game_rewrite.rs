//! Plugins the game switched off when it last started (ADR 0089).
//!
//! The mod manager writes the list of active plugins before the game
//! starts. A plugin the game cannot load, most often one whose master is
//! missing, the game switches off, and it writes the list again itself.
//! The mod manager then finds a list it did not write and says another
//! program changed the load order. This finding says which plugins, why,
//! and what settles it: switching on the mod that has the missing master,
//! or switching the plugins off in the mod manager too.

use mallet_core::facts::plugin_list::{OffReason, SwitchedOffByGame};
use mallet_core::{Action, Finding, ModRef, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many plugins or masters a finding names in its sentences.
const EXAMPLE_LIMIT: usize = 5;

/// Reports the plugins the game switched off at its last start.
pub struct GameRewroteLoadOrder;

impl Analysis for GameRewroteLoadOrder {
    fn id(&self) -> &'static str {
        "game-rewrote-load-order"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let Some(rewrite) = &context.facts.game_rewrite else {
            return Vec::new();
        };
        let off = &rewrite.switched_off;
        if off.is_empty() {
            return Vec::new();
        }
        let mut masters: Vec<&str> = Vec::new();
        let mut master_mods: Vec<&ModRef> = Vec::new();
        for plugin in off {
            if let OffReason::MissingMasters(missing) = &plugin.reason {
                for master in missing {
                    if !masters
                        .iter()
                        .any(|known| known.eq_ignore_ascii_case(&master.plugin))
                    {
                        masters.push(&master.plugin);
                    }
                    if let Some(found) = &master.switched_off_mod
                        && !master_mods.contains(&found)
                    {
                        master_mods.push(found);
                    }
                }
            }
        }
        let names: Vec<&str> = off.iter().map(|plugin| plugin.plugin.as_str()).collect();
        let needs: Vec<String> = off.iter().map(needs_of).collect();
        let mut finding = Finding::new(&kinds::PLUGINS_GAME_REWROTE_LOAD_ORDER, Subject::Setup)
            .with_arg("game", rewrite.game.clone())
            .with_arg(
                "manager",
                rewrite.manager.clone().unwrap_or_else(|| "none".to_owned()),
            )
            .with_arg("count", number(off.len()))
            .with_arg("plugins", examples(names.iter().copied(), EXAMPLE_LIMIT))
            .with_arg("pluginList", names.join("\n"))
            .with_arg("needs", needs.join("\n"))
            .with_arg("cause", cause_of(off))
            .with_arg("masters", examples(masters.iter().copied(), EXAMPLE_LIMIT))
            .with_arg("masterCount", number(masters.len()))
            .with_arg(
                "masterMods",
                examples(
                    master_mods.iter().map(|found| found.name.as_str()),
                    EXAMPLE_LIMIT,
                ),
            )
            .with_arg("masterModCount", number(master_mods.len()));
        // Either settles it, on the user's click only (ADR 0020): the mods
        // with the missing masters on, or the plugins off in the manager.
        if rewrite.manager.is_some() {
            for found in &master_mods {
                finding = finding.with_action(Action::EnableMod {
                    mod_ref: (*found).clone(),
                });
            }
            finding = finding.with_action(Action::DisablePlugins {
                plugins: names.iter().map(|name| (*name).to_owned()).collect(),
            });
        }
        vec![finding]
    }
}

/// What one plugin needs that is not loaded, for the window: its missing
/// masters, joined.
fn needs_of(plugin: &SwitchedOffByGame) -> String {
    match &plugin.reason {
        OffReason::MissingMasters(missing) => missing
            .iter()
            .map(|master| master.plugin.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

/// Why the plugins were switched off, if it is the same for all of them.
fn cause_of(off: &[SwitchedOffByGame]) -> &'static str {
    let cause = |plugin: &SwitchedOffByGame| match plugin.reason {
        OffReason::MissingMasters(_) => "missing-master",
        OffReason::NotFound => "not-found",
        OffReason::Unreadable => "unreadable",
        OffReason::TooMany => "too-many",
        OffReason::Unknown => "unknown",
    };
    let mut causes = off.iter().map(cause);
    let Some(first) = causes.next() else {
        return "unknown";
    };
    if causes.all(|other| other == first) {
        first
    } else {
        "mixed"
    }
}

/// A count as a finding's argument.
fn number(count: usize) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::plugin_list::{GameRewrite, MissingMaster};
    use mallet_core::{ArgValue, Facts, NoModSource};

    use super::*;

    fn forge() -> ModRef {
        ModRef {
            source: "vortex".to_owned(),
            id: "forge-1".to_owned(),
            name: "Forge".to_owned(),
        }
    }

    fn missing(plugin: &str, master: &str, from: Option<ModRef>) -> SwitchedOffByGame {
        SwitchedOffByGame {
            plugin: plugin.to_owned(),
            mod_ref: None,
            reason: OffReason::MissingMasters(vec![MissingMaster {
                plugin: master.to_owned(),
                present: false,
                switched_off_mod: from,
            }]),
        }
    }

    fn run(rewrite: Option<GameRewrite>) -> Vec<Finding> {
        let facts = Facts {
            game_rewrite: rewrite,
            ..Facts::default()
        };
        GameRewroteLoadOrder.run(&Context::new(&facts, &NoModSource))
    }

    #[test]
    fn patches_whose_masters_are_off_are_one_finding_with_both_ways_out() {
        let found = run(Some(GameRewrite {
            game: "Skyrim Special Edition".to_owned(),
            manager: Some("vortex".to_owned()),
            switched_off: vec![
                missing("Forge Patch.esp", "Forge.esp", Some(forge())),
                missing("Roof Patch.esp", "Roof.esp", None),
            ],
        }));
        assert_eq!(found.len(), 1);
        let finding = &found[0];
        assert_eq!(finding.kind, "plugins.game-rewrote-load-order");
        assert_eq!(
            finding.id.to_string(),
            "plugins.game-rewrote-load-order:setup"
        );
        assert_eq!(finding.args["count"], ArgValue::from(2_i64));
        assert_eq!(finding.args["cause"], ArgValue::from("missing-master"));
        assert_eq!(
            finding.args["masters"],
            ArgValue::from("Forge.esp, Roof.esp")
        );
        assert_eq!(finding.args["masterMods"], ArgValue::from("Forge"));
        assert_eq!(finding.args["masterModCount"], ArgValue::from(1_i64));
        assert_eq!(finding.args["needs"], ArgValue::from("Forge.esp\nRoof.esp"));
        assert_eq!(
            finding.actions,
            vec![
                Action::EnableMod { mod_ref: forge() },
                Action::DisablePlugins {
                    plugins: vec!["Forge Patch.esp".to_owned(), "Roof Patch.esp".to_owned()],
                },
            ]
        );
    }

    #[test]
    fn mixed_causes_are_told_as_mixed_and_nothing_without_a_rewrite() {
        let found = run(Some(GameRewrite {
            game: "Skyrim Special Edition".to_owned(),
            manager: None,
            switched_off: vec![
                missing("Forge Patch.esp", "Forge.esp", None),
                SwitchedOffByGame {
                    plugin: "Gone.esp".to_owned(),
                    mod_ref: None,
                    reason: OffReason::NotFound,
                },
            ],
        }));
        assert_eq!(found[0].args["cause"], ArgValue::from("mixed"));
        assert_eq!(found[0].args["manager"], ArgValue::from("none"));
        assert_eq!(found[0].actions, Vec::new());
        assert_eq!(run(None), Vec::new());
    }
}
