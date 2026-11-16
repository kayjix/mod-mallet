//! Script extender plugins that did not load.

use mallet_core::facts::{ExtenderPlugin, ExtenderPluginStatus, ExtenderReport};
use mallet_core::{Finding, Subject, kinds};

use crate::{Analysis, Context};

/// Reports every script extender plugin the game refused to load.
///
/// A plugin that is switched off takes its whole mod's features with it
/// while the game still starts normally, which makes this one of the
/// hardest problems to notice by playing.
pub struct ExtenderPlugins;

impl Analysis for ExtenderPlugins {
    fn id(&self) -> &'static str {
        "extender-plugins"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let Some(report) = &context.facts.extender else {
            return Vec::new();
        };
        report
            .plugins
            .iter()
            .filter_map(|plugin| finding_for(report, plugin, context))
            .collect()
    }
}

fn finding_for(
    report: &ExtenderReport,
    plugin: &ExtenderPlugin,
    context: &Context<'_>,
) -> Option<Finding> {
    let path = report.plugin_dir.join(&plugin.file_name);
    let provider = context.mods.provider_of(&path);
    let subject = Subject::File { path };
    let file = plugin.file_name.as_str();

    let finding = match &plugin.status {
        ExtenderPluginStatus::Loaded => return None,
        ExtenderPluginStatus::IncompatibleGameVersion => {
            Finding::new(&kinds::EXTENDER_PLUGIN_INCOMPATIBLE, subject)
                .with_arg("file", file)
                .with_arg("gameVersion", report.game_version.as_deref().unwrap_or("?"))
        }
        ExtenderPluginStatus::NoVersionData => {
            Finding::new(&kinds::EXTENDER_PLUGIN_NO_VERSION_DATA, subject).with_arg("file", file)
        }
        ExtenderPluginStatus::NotLoaded { reason } => {
            Finding::new(&kinds::EXTENDER_PLUGIN_NOT_LOADED, subject)
                .with_arg("file", file)
                .with_arg("reason", reason.as_str())
        }
    };
    Some(crate::offer_disabling(finding.with_provider(provider)))
}

#[cfg(test)]
mod tests {
    use mallet_core::{ArgValue, Facts, GamePath, ModRef, ModSource, NoModSource, Severity};

    use super::*;
    use crate::run_all;

    /// A mod manager that deployed exactly one file.
    struct OneFile {
        path: GamePath,
        name: &'static str,
    }

    impl ModSource for OneFile {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            (*path == self.path).then(|| ModRef {
                source: "test".to_owned(),
                id: self.name.to_owned(),
                name: self.name.to_owned(),
            })
        }
    }

    fn plugin(file_name: &str, status: ExtenderPluginStatus) -> ExtenderPlugin {
        ExtenderPlugin {
            file_name: file_name.to_owned(),
            status,
        }
    }

    fn facts(plugins: Vec<ExtenderPlugin>) -> Facts {
        Facts {
            extender: Some(ExtenderReport {
                extender_version: Some("2.3.1".to_owned()),
                game_version: Some("1.7.104".to_owned()),
                plugin_dir: GamePath::new("data/skse/plugins"),
                plugins,
            }),
            ..Facts::default()
        }
    }

    #[test]
    fn loaded_plugins_are_not_reported() {
        let facts = facts(vec![plugin("Fixes.dll", ExtenderPluginStatus::Loaded)]);
        let findings = run_all(&Context::new(&facts, &NoModSource));
        assert_eq!(findings, Vec::<Finding>::new());
    }

    #[test]
    fn without_an_extender_report_nothing_is_reported() {
        let findings = run_all(&Context::new(&Facts::default(), &NoModSource));
        assert_eq!(findings, Vec::<Finding>::new());
    }

    #[test]
    fn an_incompatible_plugin_is_an_error_naming_file_version_and_mod() {
        let facts = facts(vec![plugin(
            "Old.dll",
            ExtenderPluginStatus::IncompatibleGameVersion,
        )]);
        let mods = OneFile {
            path: GamePath::new("data/skse/plugins/old.dll"),
            name: "Old Mod",
        };
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "extender.plugin-incompatible");
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(
            finding.id.as_str(),
            "extender.plugin-incompatible:file:data/skse/plugins/old.dll"
        );
        assert_eq!(finding.args["file"], ArgValue::from("Old.dll"));
        assert_eq!(finding.args["gameVersion"], ArgValue::from("1.7.104"));
        assert_eq!(
            finding.provider.as_ref().map(|m| m.name.as_str()),
            Some("Old Mod")
        );
    }

    #[test]
    fn other_reasons_are_passed_on_in_the_extenders_words() {
        let facts = facts(vec![plugin(
            "Broken.dll",
            ExtenderPluginStatus::NotLoaded {
                reason: "disabled, fatal error occurred while loading plugin".to_owned(),
            },
        )]);
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "extender.plugin-not-loaded");
        assert_eq!(
            finding.args["reason"],
            ArgValue::from("disabled, fatal error occurred while loading plugin")
        );
        assert_eq!(finding.provider, None);
    }

    #[test]
    fn findings_are_ordered_errors_first_then_by_identity() {
        let facts = facts(vec![
            plugin("helper.dll", ExtenderPluginStatus::NoVersionData),
            plugin("Zeta.dll", ExtenderPluginStatus::IncompatibleGameVersion),
            plugin("Alpha.dll", ExtenderPluginStatus::IncompatibleGameVersion),
        ]);
        let findings = run_all(&Context::new(&facts, &NoModSource));

        let order: Vec<_> = findings
            .iter()
            .map(|finding| finding.subject.identity())
            .collect();
        assert_eq!(
            order,
            [
                "file:data/skse/plugins/alpha.dll",
                "file:data/skse/plugins/zeta.dll",
                "file:data/skse/plugins/helper.dll",
            ]
        );
    }

    #[test]
    fn every_kind_declares_the_arguments_its_findings_carry() {
        let facts = facts(vec![
            plugin("a.dll", ExtenderPluginStatus::IncompatibleGameVersion),
            plugin("b.dll", ExtenderPluginStatus::NoVersionData),
            plugin(
                "c.dll",
                ExtenderPluginStatus::NotLoaded {
                    reason: "x".to_owned(),
                },
            ),
        ]);
        let findings = run_all(&Context::new(&facts, &NoModSource));
        assert_eq!(findings.len(), 3);

        for finding in findings {
            let kind = kinds::find(&finding.kind).unwrap();
            let mut declared: Vec<_> = kind.args.to_vec();
            declared.sort_unstable();
            let carried: Vec<_> = finding.args.keys().map(String::as_str).collect();
            assert_eq!(carried, declared, "arguments of {}", kind.id);
        }
    }
}
