//! Active plugins that build on a plugin which is not installed.
//!
//! A plugin names the plugins it builds on, its masters. The game loads
//! them first and closes while starting if one of them is not there. This
//! happens when a patch for two mods is installed and one of the two is
//! not, or when a mod was removed and its patches stayed.
//!
//! Only a master whose file is gone is reported. A master that is
//! installed but switched off is left to the mod manager, which shows it.

use std::collections::HashSet;

use mallet_core::{Finding, GamePath, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many missing masters a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Reports active plugins with a master that is not installed.
pub struct MissingMasters;

impl Analysis for MissingMasters {
    fn id(&self) -> &'static str {
        "missing-masters"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let active: HashSet<String> = context
            .facts
            .plugins
            .iter()
            .map(|plugin| plugin.name.to_lowercase())
            .collect();
        let is_there = |master: &str| {
            active.contains(&master.to_lowercase())
                || context.assets.is_loose(&GamePath::new("data").join(master))
        };

        context
            .facts
            .plugins
            .iter()
            .filter_map(|plugin| {
                let missing: Vec<&str> = plugin
                    .masters
                    .iter()
                    .map(String::as_str)
                    .filter(|master| !is_there(master))
                    .collect();
                if missing.is_empty() {
                    return None;
                }
                Some(
                    Finding::new(
                        &kinds::PLUGINS_MASTER_MISSING,
                        Subject::File {
                            path: plugin.path.clone(),
                        },
                    )
                    .with_arg("plugin", plugin.name.as_str())
                    .with_arg("count", i64::try_from(missing.len()).unwrap_or(i64::MAX))
                    .with_arg("masters", examples(missing, EXAMPLE_LIMIT))
                    .with_provider(context.mods.provider_of(&plugin.path)),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::{ArgValue, Confidence, Facts, NoModSource, Severity};

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

    #[test]
    fn a_patch_whose_mod_is_gone_is_reported() {
        let facts = Facts {
            plugins: vec![
                plugin("Skyrim.esm", &[]),
                plugin("City.esp", &["Skyrim.esm"]),
                plugin(
                    "City - Dungeons Patch.esp",
                    &["Skyrim.esm", "City.esp", "Dungeons.esp"],
                ),
            ],
            ..Facts::default()
        };
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.master-missing");
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(finding.confidence, Confidence::Certain);
        assert_eq!(
            finding.id.as_str(),
            "plugins.master-missing:file:data/city - dungeons patch.esp"
        );
        assert_eq!(
            finding.args["plugin"],
            ArgValue::from("City - Dungeons Patch.esp")
        );
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["masters"], ArgValue::from("Dungeons.esp"));
    }

    #[test]
    fn masters_are_matched_without_regard_to_case() {
        let facts = Facts {
            plugins: vec![
                plugin("skyrim.ESM", &[]),
                plugin("Mine.esp", &["Skyrim.esm"]),
            ],
            ..Facts::default()
        };
        assert_eq!(run_all(&Context::new(&facts, &NoModSource)), Vec::new());
    }

    #[test]
    fn a_master_that_is_installed_but_switched_off_is_not_reported() {
        let facts = Facts {
            loose_files: vec![GamePath::new("data/Resting.esp")],
            plugins: vec![plugin("Mine.esp", &["Resting.esp"])],
            ..Facts::default()
        };
        assert_eq!(run_all(&Context::new(&facts, &NoModSource)), Vec::new());
    }
}
