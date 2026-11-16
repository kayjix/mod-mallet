//! What the LOOT masterlist says about the active plugins.
//!
//! The adapter has already kept only the rules whose conditions hold for
//! this setup (ADR 0008). This analysis decides which of them describe a
//! problem: a required file that is not there, an incompatible file that
//! is, and a plugin version known to need cleaning.
//!
//! A required or incompatible plugin counts only while it is active: a
//! plugin that is installed but switched off does nothing in the game.

use std::collections::HashSet;

use mallet_core::facts::knowledge::{KnownFile, PluginKnowledge};
use mallet_core::{Action, Finding, GamePath, Party, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many missing files a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Reports what the masterlist's rules find in the setup.
pub struct KnownRules;

impl Analysis for KnownRules {
    fn id(&self) -> &'static str {
        "known-rules"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let active: HashSet<String> = context
            .facts
            .plugins
            .iter()
            .map(|plugin| plugin.name.to_lowercase())
            .collect();
        let is_active = |file: &KnownFile| active.contains(&file.name.to_lowercase());

        let mut findings = Vec::new();
        let mut seen = HashSet::new();
        for plugin in &context.facts.knowledge.plugins {
            if let Some(finding) = missing_requirements(context, plugin, &is_active) {
                findings.push(finding);
            }
            for finding in incompatibilities(context, plugin, &is_active) {
                // Two plugins often name each other; one finding is enough.
                if seen.insert(finding.id.clone()) {
                    findings.push(finding);
                }
            }
            if let Some(finding) = needs_cleaning(context, plugin) {
                findings.push(finding);
            }
        }
        findings
    }
}

fn missing_requirements(
    context: &Context<'_>,
    plugin: &PluginKnowledge,
    is_active: &dyn Fn(&KnownFile) -> bool,
) -> Option<Finding> {
    let missing: Vec<&str> = plugin
        .requirements
        .iter()
        .filter(|file| !file.present || (file.is_plugin() && !is_active(file)))
        .map(KnownFile::label)
        .collect();
    if missing.is_empty() {
        return None;
    }
    Some(crate::offer_disabling(
        Finding::new(
            &kinds::PLUGINS_REQUIREMENT_MISSING,
            Subject::File {
                path: plugin.path.clone(),
            },
        )
        .with_arg("plugin", plugin.plugin.as_str())
        .with_arg("count", i64::try_from(missing.len()).unwrap_or(i64::MAX))
        .with_arg("files", examples(missing, EXAMPLE_LIMIT))
        .with_provider(context.mods.provider_of(&plugin.path)),
    ))
}

fn incompatibilities<'a>(
    context: &'a Context<'_>,
    plugin: &'a PluginKnowledge,
    is_active: &'a dyn Fn(&KnownFile) -> bool,
) -> impl Iterator<Item = Finding> + 'a {
    plugin
        .incompatibilities
        .iter()
        .filter(move |file| file.present && (!file.is_plugin() || is_active(file)))
        .map(move |file| {
            let other_path = GamePath::new("data").join(&file.name);
            let mut parties = [
                Party::File {
                    path: plugin.path.clone(),
                },
                Party::File {
                    path: other_path.clone(),
                },
            ];
            parties.sort_by_key(|party| match party {
                Party::File { path } => path.to_string(),
                _ => String::new(),
            });
            let [first, second] = parties;
            let own_mod = context.mods.provider_of(&plugin.path);
            let other_mod = context.mods.provider_of(&other_path);
            let mut finding = Finding::new(
                &kinds::PLUGINS_INCOMPATIBLE,
                Subject::Pair { first, second },
            )
            .with_arg("plugin", plugin.plugin.as_str())
            .with_arg("other", file.label())
            .with_provider(own_mod.clone());
            finding = crate::offer_disabling(finding);
            if let Some(other_mod) = other_mod.filter(|other| Some(other) != own_mod.as_ref()) {
                finding = finding.with_action(Action::DisableMod { mod_ref: other_mod });
            }
            finding
        })
}

fn needs_cleaning(context: &Context<'_>, plugin: &PluginKnowledge) -> Option<Finding> {
    // The game's updates replace its own plugins, so cleaning them does
    // not last, and whether they should be cleaned at all is disputed.
    if plugin.shipped_with_game {
        return None;
    }
    let cleaning = plugin.dirty.first()?;
    let count = |value: u32| i64::from(value);
    let subject = Subject::File {
        path: plugin.path.clone(),
    };
    let finding = if cleaning.deleted_references > 0 || cleaning.deleted_navmeshes > 0 {
        Finding::new(&kinds::PLUGINS_NEEDS_CLEANING, subject)
            .with_arg("identical", count(cleaning.identical_records))
            .with_arg("deleted", count(cleaning.deleted_references))
            .with_arg("navmeshes", count(cleaning.deleted_navmeshes))
    } else if cleaning.identical_records > 0 {
        Finding::new(&kinds::PLUGINS_IDENTICAL_RECORDS, subject)
            .with_arg("count", count(cleaning.identical_records))
    } else {
        return None;
    };
    Some(
        finding
            .with_arg("plugin", plugin.plugin.as_str())
            .with_arg("utility", cleaning.utility.as_str())
            .with_provider(context.mods.provider_of(&plugin.path)),
    )
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::knowledge::{Cleaning, KnowledgeFacts};
    use mallet_core::{ArgValue, Confidence, Facts, NoModSource, Severity};

    use super::*;
    use crate::run_all;

    fn plugin(name: &str) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: Vec::new(),
            npcs: Vec::new(),
        }
    }

    fn file(name: &str, present: bool) -> KnownFile {
        KnownFile {
            name: name.to_owned(),
            display: None,
            page: None,
            present,
        }
    }

    fn rules(name: &str) -> PluginKnowledge {
        PluginKnowledge {
            plugin: name.to_owned(),
            path: GamePath::new("data").join(name),
            shipped_with_game: false,
            requirements: Vec::new(),
            incompatibilities: Vec::new(),
            dirty: Vec::new(),
        }
    }

    fn findings(plugins: &[&str], knowledge: Vec<PluginKnowledge>) -> Vec<Finding> {
        let facts = Facts {
            plugins: plugins.iter().map(|name| plugin(name)).collect(),
            knowledge: KnowledgeFacts { plugins: knowledge },
            ..Facts::default()
        };
        run_all(&Context::new(&facts, &NoModSource))
    }

    #[test]
    fn a_missing_file_and_an_inactive_plugin_are_missing_requirements() {
        let mut city = rules("City.esp");
        city.requirements = vec![
            KnownFile {
                display: Some("Helper".to_owned()),
                ..file("SKSE/Plugins/Helper.dll", false)
            },
            file("Installed But Off.esp", true),
            file("Fine.esp", true),
            file("SKSE/Plugins/There.dll", true),
        ];
        let findings = findings(&["City.esp", "Fine.esp"], vec![city]);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.requirement-missing");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(
            finding.args["files"],
            ArgValue::from("Helper, Installed But Off.esp")
        );
    }

    #[test]
    fn plugins_that_name_each_other_as_incompatible_give_one_finding() {
        let mut city = rules("City.esp");
        city.incompatibilities = vec![file("Other City.esp", true), file("Off.esp", true)];
        let mut other = rules("Other City.esp");
        other.incompatibilities = vec![file("City.esp", true)];
        let findings = findings(&["City.esp", "Other City.esp"], vec![city, other]);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.incompatible");
        assert_eq!(
            finding.id.as_str(),
            "plugins.incompatible:pair:file:data/city.esp+file:data/other city.esp"
        );
        assert_eq!(finding.args["plugin"], ArgValue::from("City.esp"));
        assert_eq!(finding.args["other"], ArgValue::from("Other City.esp"));
    }

    #[test]
    fn an_incompatible_file_that_is_not_a_plugin_counts_when_present() {
        let mut city = rules("City.esp");
        city.incompatibilities = vec![
            file("SKSE/Plugins/Clash.dll", true),
            file("SKSE/Plugins/Absent.dll", false),
        ];
        let findings = findings(&["City.esp"], vec![city]);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(
            finding.args["other"],
            ArgValue::from("SKSE/Plugins/Clash.dll")
        );
    }

    #[test]
    fn a_dirty_plugin_is_reported_with_the_counts() {
        let mut city = rules("City.esp");
        city.dirty = vec![Cleaning {
            identical_records: 12,
            deleted_references: 3,
            deleted_navmeshes: 0,
            utility: "SSEEdit v4.1.5".to_owned(),
        }];
        let findings = findings(&["City.esp"], vec![city]);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.needs-cleaning");
        assert_eq!(finding.args["identical"], ArgValue::Number(12));
        assert_eq!(finding.args["deleted"], ArgValue::Number(3));
        assert_eq!(finding.args["navmeshes"], ArgValue::Number(0));
        assert_eq!(finding.args["utility"], ArgValue::from("SSEEdit v4.1.5"));
    }

    fn copies_only(count: u32) -> Cleaning {
        Cleaning {
            identical_records: count,
            deleted_references: 0,
            deleted_navmeshes: 0,
            utility: "SSEEdit".to_owned(),
        }
    }

    #[test]
    fn a_plugin_with_only_unchanged_copies_is_a_note() {
        let mut city = rules("City.esp");
        city.dirty = vec![copies_only(7)];
        let findings = findings(&["City.esp"], vec![city]);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.identical-records");
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(finding.args["count"], ArgValue::Number(7));
    }

    #[test]
    fn the_games_own_plugins_are_not_reported_for_cleaning() {
        let mut update = rules("Update.esm");
        update.shipped_with_game = true;
        update.dirty = vec![Cleaning {
            deleted_references: 90,
            deleted_navmeshes: 6,
            ..copies_only(400)
        }];
        assert_eq!(
            findings(&["Update.esm"], vec![update]),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn no_rules_no_findings() {
        assert_eq!(
            findings(&["City.esp"], vec![rules("City.esp")]),
            Vec::<Finding>::new()
        );
    }
}
