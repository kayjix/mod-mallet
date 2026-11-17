//! Plugins that a tool generated from the load order, made before the load
//! order last changed.
//!
//! Tools such as DynDOLOD, Synthesis or Wrye Bash read every active plugin
//! and write a plugin of their own from what wins. They list the plugins
//! they read as masters, in the load order of that moment. When the load
//! order changes afterwards, the generated plugin still carries the old
//! winners until the tool is run again. The masters then no longer appear
//! in the order in which the game loads them, which is what this analysis
//! looks for.
//!
//! Only plugins whose names these tools always use are looked at. A mod's
//! own plugin may list its masters in any order without harm.

use std::collections::HashMap;

use mallet_core::{Finding, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many moved plugins a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// The tool that made a plugin, known by the plugin's file name.
fn generator(plugin: &str) -> Option<&'static str> {
    let name = plugin.to_lowercase();
    match name.as_str() {
        "dyndolod.esp" => Some("DynDOLOD"),
        "occlusion.esp" => Some("DynDOLOD / xLODGen"),
        "synthesis.esp" => Some("Synthesis"),
        "smashed patch.esp" => Some("Mator Smash"),
        _ => {
            // Wrye Bash numbers its patches: `Bashed Patch, 0.esp`, ...
            let number = name.strip_prefix("bashed patch, ")?.strip_suffix(".esp")?;
            (!number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
                .then_some("Wrye Bash")
        }
    }
}

/// Reports generated plugins whose masters no longer load in the order the
/// tool saw.
pub struct GeneratedPlugins;

impl Analysis for GeneratedPlugins {
    fn id(&self) -> &'static str {
        "generated-plugins"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let positions: HashMap<String, usize> = context
            .facts
            .plugins
            .iter()
            .enumerate()
            .map(|(position, plugin)| (plugin.name.to_lowercase(), position))
            .collect();

        context
            .facts
            .plugins
            .iter()
            .filter_map(|plugin| {
                let tool = generator(&plugin.name)?;
                // Masters that are not active are another finding's matter.
                let masters: Vec<(&str, usize)> = plugin
                    .masters
                    .iter()
                    .filter_map(|master| {
                        let position = positions.get(&master.to_lowercase())?;
                        Some((master.as_str(), *position))
                    })
                    .collect();
                let moved = moved(&masters);
                if moved.is_empty() {
                    return None;
                }
                Some(
                    Finding::new(
                        &kinds::PLUGINS_GENERATED_OUTDATED,
                        Subject::File {
                            path: plugin.path.clone(),
                        },
                    )
                    .with_arg("plugin", plugin.name.as_str())
                    .with_arg("tool", tool)
                    .with_arg("count", i64::try_from(moved.len()).unwrap_or(i64::MAX))
                    .with_arg("examples", examples(moved, EXAMPLE_LIMIT))
                    .with_provider(context.mods.provider_of(&plugin.path)),
                )
            })
            .collect()
    }
}

/// The fewest masters that have to have moved for the masters, listed in
/// the order the tool saw, to load at the given positions now: everything
/// outside the longest run that is still in order.
fn moved<'a>(masters: &[(&'a str, usize)]) -> Vec<&'a str> {
    // For each master, the length of the longest ordered run ending with
    // it, and the master before it in that run. A generated plugin has at
    // most a few hundred masters, so the quadratic search is quick.
    let mut runs: Vec<(usize, Option<usize>)> = Vec::with_capacity(masters.len());
    for (index, (_, position)) in masters.iter().enumerate() {
        let best = masters
            .iter()
            .zip(&runs)
            .enumerate()
            .take(index)
            .filter(|(_, ((_, earlier), _))| earlier < position)
            .max_by_key(|(_, (_, (length, _)))| *length)
            .map(|(before, (_, (length, _)))| (length + 1, Some(before)));
        runs.push(best.unwrap_or((1, None)));
    }

    let mut in_order = vec![false; masters.len()];
    let mut next = runs
        .iter()
        .enumerate()
        .max_by_key(|(_, (length, _))| *length)
        .map(|(index, _)| index);
    while let Some(index) = next {
        if let Some(flag) = in_order.get_mut(index) {
            *flag = true;
        }
        next = runs.get(index).and_then(|(_, before)| *before);
    }

    masters
        .iter()
        .zip(in_order)
        .filter(|(_, in_order)| !in_order)
        .map(|((name, _), _)| *name)
        .collect()
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::{ArgValue, Confidence, Facts, GamePath, NoModSource, Severity};

    use super::*;
    use crate::run_all;

    fn plugin(name: &str, masters: &[&str]) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: masters.iter().map(|master| (*master).to_owned()).collect(),
            npcs: Vec::new(),
        }
    }

    fn setup(order: &[&str], generated: PluginFacts) -> Facts {
        let mut plugins: Vec<PluginFacts> = order.iter().map(|name| plugin(name, &[])).collect();
        plugins.push(generated);
        Facts {
            plugins,
            ..Facts::default()
        }
    }

    #[test]
    fn a_plugin_moved_after_dyndolod_ran_is_reported() {
        let facts = setup(
            &["Skyrim.esm", "Town.esp", "Forest.esp", "Castle.esp"],
            plugin(
                "DynDOLOD.esp",
                &["Skyrim.esm", "Castle.esp", "Town.esp", "Forest.esp"],
            ),
        );
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.generated-outdated");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "plugins.generated-outdated:file:data/dyndolod.esp"
        );
        assert_eq!(finding.args["tool"], ArgValue::from("DynDOLOD"));
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["examples"], ArgValue::from("Castle.esp"));
    }

    #[test]
    fn masters_in_load_order_are_not_reported() {
        let facts = setup(
            &["Skyrim.esm", "Town.esp", "New.esp", "Forest.esp"],
            plugin("Synthesis.esp", &["Skyrim.esm", "Town.esp", "Forest.esp"]),
        );
        assert_eq!(run_all(&Context::new(&facts, &NoModSource)), Vec::new());
    }

    #[test]
    fn a_mods_own_plugin_may_list_its_masters_in_any_order() {
        let facts = setup(
            &["Skyrim.esm", "Town.esp", "Forest.esp"],
            plugin("Patch.esp", &["Forest.esp", "Town.esp"]),
        );
        assert_eq!(run_all(&Context::new(&facts, &NoModSource)), Vec::new());
    }

    #[test]
    fn numbered_bashed_patches_are_recognised() {
        assert_eq!(generator("Bashed Patch, 0.esp"), Some("Wrye Bash"));
        assert_eq!(generator("bashed patch, 12.ESP"), Some("Wrye Bash"));
        assert_eq!(generator("Bashed Patch, .esp"), None);
        assert_eq!(generator("Bashed Patch, x.esp"), None);
        assert_eq!(generator("Bashed Patch Extras.esp"), None);
    }

    #[test]
    fn the_fewest_plugins_are_named_as_moved() {
        // B moved to the end: only B is out of place, not C and D.
        let moved_b = moved(&[("A", 0), ("B", 3), ("C", 1), ("D", 2)]);
        assert_eq!(moved_b, vec!["B"]);
        assert_eq!(moved(&[]), Vec::<&str>::new());
        assert_eq!(moved(&[("A", 2), ("B", 1), ("C", 0)]).len(), 2);
    }

    #[test]
    fn inactive_masters_are_left_to_other_checks() {
        let facts = setup(
            &["Skyrim.esm", "Town.esp"],
            plugin("Occlusion.esp", &["Skyrim.esm", "Gone.esp", "Town.esp"]),
        );
        let findings: Vec<_> = run_all(&Context::new(&facts, &NoModSource))
            .into_iter()
            .filter(|finding| finding.kind == "plugins.generated-outdated")
            .collect();
        assert_eq!(findings, Vec::new());
    }
}
