//! Installer options that fit the setup now but were not installed.
//!
//! Many mods come with an installer that offers patches for other mods and
//! ticks the ones that fit what is installed at that moment. The dialog is
//! shown once. When a mod that needs such a patch is installed later,
//! nobody offers the patch again.
//!
//! The analysis replays each installer's own rules against the current
//! setup. It reports an option when all of this is true:
//!
//! * the option was not chosen,
//! * a rule of the installer that looks at an installed file now marks it
//!   as recommended or required,
//! * its step is shown, given the options that were chosen,
//! * its group allows adding it without replacing another choice.
//!
//! An option is named with its place in the installer, as
//! `step › group › option`: large installers have a dozen pages, and the
//! name of an option alone does not say on which one it is.
//!
//! Options that are recommended to everyone are left alone: not choosing
//! one of those was the user's decision, not an accident of timing.

use std::collections::{HashMap, HashSet};

use mallet_core::facts::installer::{
    Condition, FileState, InstallerFacts, InstallerOption, OptionStatus, Selection,
};
use mallet_core::{Action, Finding, GamePath, OptionToTick, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many option names a finding quotes.
const EXAMPLE_LIMIT: usize = 5;

/// File extensions of plugins, which can be installed without being active.
const PLUGIN_EXTENSIONS: [&str; 3] = [".esp", ".esm", ".esl"];

/// Reports mods whose installer would now tick options that are not
/// installed.
pub struct InstallerOptions;

impl Analysis for InstallerOptions {
    fn id(&self) -> &'static str {
        "installer-options"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        // Without a load order nothing can be said about what is active.
        if context.facts.plugins.is_empty() {
            return Vec::new();
        }
        let setup = Setup {
            context,
            active_plugins: context
                .facts
                .plugins
                .iter()
                .map(|plugin| plugin.name.to_lowercase())
                .collect(),
        };

        context
            .facts
            .installers
            .iter()
            .filter_map(|installer| {
                let missing = setup.missing_options(installer);
                (!missing.is_empty()).then(|| finding(installer, &missing))
            })
            .collect()
    }
}

/// The parts of the setup an installer can ask about.
struct Setup<'a> {
    context: &'a Context<'a>,
    /// File names of the active plugins, in lower case.
    active_plugins: HashSet<String>,
}

impl Setup<'_> {
    /// The options the installer would tick now that were not chosen, each
    /// with its place in the installer.
    fn missing_options(&self, installer: &InstallerFacts) -> Vec<OptionToTick> {
        let chosen: HashSet<(&str, &str, &str)> = installer
            .chosen
            .iter()
            .map(|choice| {
                (
                    choice.step.as_str(),
                    choice.group.as_str(),
                    choice.option.as_str(),
                )
            })
            .collect();
        let is_chosen = |step: &str, group: &str, option: &InstallerOption| {
            chosen.contains(&(step, group, option.name.as_str()))
        };

        // The flags as they stand after all chosen options.
        let mut flags: HashMap<&str, &str> = HashMap::new();
        for step in &installer.steps {
            for group in &step.groups {
                for option in &group.options {
                    if is_chosen(&step.name, &group.name, option) {
                        for (name, value) in &option.sets_flags {
                            flags.insert(name, value);
                        }
                    }
                }
            }
        }

        let shown: Vec<_> = installer
            .steps
            .iter()
            .filter(|step| {
                step.visible
                    .as_ref()
                    .is_none_or(|condition| self.holds(condition, &flags))
            })
            .collect();
        let pages = u32::try_from(shown.len()).unwrap_or(u32::MAX);

        let mut missing = Vec::new();
        for (page, step) in (1..).zip(shown) {
            for group in &step.groups {
                // In these groups an option cannot simply be added: it
                // would replace the user's choice, or there is no choice.
                if matches!(
                    group.selection,
                    Selection::ExactlyOne | Selection::AtMostOne | Selection::All
                ) {
                    continue;
                }
                for (index, option) in (0..).zip(&group.options) {
                    if !is_chosen(&step.name, &group.name, option)
                        && self.is_ticked_because_of_the_setup(option, &flags)
                    {
                        missing.push(OptionToTick {
                            page,
                            pages,
                            step: step.name.clone(),
                            group: group.name.clone(),
                            option: option.name.clone(),
                            index,
                            description: option.description.clone(),
                        });
                    }
                }
            }
        }
        missing
    }

    /// Whether the first rule that applies marks the option as recommended
    /// or required, and does so because of an installed file.
    fn is_ticked_because_of_the_setup(
        &self,
        option: &InstallerOption,
        flags: &HashMap<&str, &str>,
    ) -> bool {
        option
            .rules
            .iter()
            .find(|(condition, _)| self.holds(condition, flags))
            .is_some_and(|(condition, status)| {
                matches!(status, OptionStatus::Recommended | OptionStatus::Required)
                    && asks_about_a_file(condition)
            })
    }

    fn holds(&self, condition: &Condition, flags: &HashMap<&str, &str>) -> bool {
        match condition {
            Condition::All(parts) => parts.iter().all(|part| self.holds(part, flags)),
            Condition::Any(parts) => parts.iter().any(|part| self.holds(part, flags)),
            Condition::File { file, state } => {
                let (installed, active) = self.file_state(file);
                match state {
                    FileState::Active => active,
                    FileState::Inactive => installed && !active,
                    FileState::Missing => !installed,
                }
            }
            Condition::Flag { name, value } => {
                flags.get(name.as_str()).copied().unwrap_or_default() == value
            }
            Condition::Unknown => true,
        }
    }

    /// Whether `file` (relative to the data directory) is installed, and
    /// whether it is active. Only a plugin can be installed and inactive.
    fn file_state(&self, file: &str) -> (bool, bool) {
        let path = GamePath::new("data").join(file);
        let installed = self.context.assets.is_loose(&path);
        let is_plugin = PLUGIN_EXTENSIONS
            .iter()
            .any(|extension| path.as_str().ends_with(extension));
        let active = if is_plugin {
            self.active_plugins.contains(path.file_name())
        } else {
            installed
        };
        (installed || active, active)
    }
}

/// Where an option is in its installer: the names of its step, its group
/// and the option itself. Installers may leave a step or a group unnamed,
/// and often give a step and its only group the same name, which is then
/// said once.
fn place(parts: &[&str]) -> String {
    let mut named: Vec<&str> = parts
        .iter()
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();
    named.dedup();
    named.join(" › ")
}

fn asks_about_a_file(condition: &Condition) -> bool {
    match condition {
        Condition::All(parts) | Condition::Any(parts) => parts.iter().any(asks_about_a_file),
        Condition::File { .. } => true,
        Condition::Flag { .. } | Condition::Unknown => false,
    }
}

fn finding(installer: &InstallerFacts, missing: &[OptionToTick]) -> Finding {
    let mod_ref = &installer.mod_ref;
    Finding::new(
        &kinds::INSTALLER_OPTIONS_NOW_RECOMMENDED,
        Subject::Mod {
            source: mod_ref.source.clone(),
            id: mod_ref.id.clone(),
        },
    )
    .with_arg("mod", mod_ref.name.as_str())
    .with_arg("count", i64::try_from(missing.len()).unwrap_or(i64::MAX))
    .with_arg(
        "examples",
        examples(
            missing
                .iter()
                .map(|tick| place(&[&tick.step, &tick.group, &tick.option]))
                .collect::<Vec<_>>()
                .iter()
                .map(String::as_str),
            EXAMPLE_LIMIT,
        ),
    )
    .with_provider(Some(mod_ref.clone()))
    // Running the installer again lets the user tick what is missing.
    .with_action(Action::ReinstallMod {
        mod_ref: mod_ref.clone(),
        tick: missing.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::installer::{ChosenOption, InstallerGroup, InstallerStep};
    use mallet_core::{ArgValue, Confidence, Facts, NoModSource, Severity};

    use super::*;
    use crate::assets::testing::mod_ref;
    use crate::run_all;

    fn recommended_if(file: &str, state: FileState) -> Vec<(Condition, OptionStatus)> {
        vec![(
            Condition::All(vec![Condition::File {
                file: file.to_owned(),
                state,
            }]),
            OptionStatus::Recommended,
        )]
    }

    fn option(name: &str, rules: Vec<(Condition, OptionStatus)>) -> InstallerOption {
        InstallerOption {
            name: name.to_owned(),
            description: format!("About {name}"),
            sets_flags: Vec::new(),
            default_status: OptionStatus::Optional,
            rules,
        }
    }

    fn step(name: &str, selection: Selection, options: Vec<InstallerOption>) -> InstallerStep {
        InstallerStep {
            name: name.to_owned(),
            visible: None,
            groups: vec![InstallerGroup {
                name: "Group".to_owned(),
                selection,
                options,
            }],
        }
    }

    fn chosen(step: &str, option: &str) -> ChosenOption {
        ChosenOption {
            step: step.to_owned(),
            group: "Group".to_owned(),
            option: option.to_owned(),
        }
    }

    /// A patch hub with a patch for a city mod and one for a dungeon mod,
    /// in a setup where only the given plugins are active.
    fn facts(steps: Vec<InstallerStep>, chosen: Vec<ChosenOption>, active: &[&str]) -> Facts {
        Facts {
            loose_files: active
                .iter()
                .chain(&["Inactive.esp"])
                .map(|name| GamePath::new("data").join(name))
                .collect(),
            plugins: active
                .iter()
                .map(|name| PluginFacts {
                    name: (*name).to_owned(),
                    path: GamePath::new("data").join(name),
                    masters: Vec::new(),
                    npcs: Vec::new(),
                })
                .collect(),
            installers: vec![InstallerFacts {
                mod_ref: mod_ref("Patch Hub"),
                steps,
                chosen,
            }],
            ..Facts::default()
        }
    }

    fn patch_step() -> InstallerStep {
        step(
            "Patches",
            Selection::Any,
            vec![
                option(
                    "City Overhaul",
                    recommended_if("City.esp", FileState::Active),
                ),
                option(
                    "Dungeons",
                    recommended_if("Dungeons.esp", FileState::Active),
                ),
                option("Extra", Vec::new()),
            ],
        )
    }

    #[test]
    fn a_place_leaves_out_what_the_installer_did_not_name() {
        assert_eq!(place(&["Step", "Group", "Option"]), "Step › Group › Option");
        assert_eq!(place(&["Step", " ", "Option"]), "Step › Option");
        assert_eq!(place(&["", "", "Option"]), "Option");
        assert_eq!(place(&["Patches", "Patches", "Option"]), "Patches › Option");
    }

    fn findings(facts: &Facts) -> Vec<Finding> {
        run_all(&Context::new(facts, &NoModSource))
    }

    #[test]
    fn a_patch_for_a_mod_that_was_installed_later_is_reported() {
        let facts = facts(
            vec![patch_step()],
            vec![chosen("Patches", "City Overhaul")],
            &["Skyrim.esm", "City.esp", "Dungeons.esp"],
        );
        let findings = findings(&facts);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "installer.options-now-recommended");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "installer.options-now-recommended:mod:test:Patch Hub"
        );
        assert_eq!(finding.args["mod"], ArgValue::from("Patch Hub"));
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        // The option is named with its step and group.
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("Patches › Group › Dungeons")
        );
        assert_eq!(
            finding.provider.as_ref().map(|m| m.name.as_str()),
            Some("Patch Hub")
        );
        let [
            Action::ReinstallMod {
                mod_ref: reinstalled,
                tick,
            },
        ] = finding.actions.as_slice()
        else {
            panic!("expected one reinstall, got {:?}", finding.actions);
        };
        assert_eq!(*reinstalled, mod_ref("Patch Hub"));
        let [tick] = tick.as_slice() else {
            panic!("expected one option to tick, got {tick:?}");
        };
        assert_eq!((tick.page, tick.pages), (1, 1));
        assert_eq!(
            (
                tick.step.as_str(),
                tick.group.as_str(),
                tick.option.as_str()
            ),
            ("Patches", "Group", "Dungeons")
        );
        assert_eq!(tick.description, "About Dungeons");
    }

    #[test]
    fn an_installer_whose_recommendations_were_all_followed_is_fine() {
        let facts = facts(
            vec![patch_step()],
            vec![chosen("Patches", "City Overhaul")],
            &["Skyrim.esm", "City.esp"],
        );
        assert_eq!(findings(&facts), Vec::<Finding>::new());
    }

    #[test]
    fn a_plugin_that_is_installed_but_inactive_does_not_count_as_active() {
        let facts = facts(
            vec![step(
                "Patches",
                Selection::Any,
                vec![
                    option(
                        "For active",
                        recommended_if("Inactive.esp", FileState::Active),
                    ),
                    option(
                        "For inactive",
                        recommended_if("Inactive.esp", FileState::Inactive),
                    ),
                    option(
                        "For missing",
                        recommended_if("Gone.esp", FileState::Missing),
                    ),
                ],
            )],
            Vec::new(),
            &["Skyrim.esm"],
        );
        let findings = findings(&facts);
        assert_eq!(
            findings[0].args["examples"],
            ArgValue::from("Patches › Group › For inactive, Patches › Group › For missing")
        );
    }

    #[test]
    fn options_recommended_to_everyone_are_left_alone() {
        let always = InstallerOption {
            default_status: OptionStatus::Recommended,
            ..option("Always", Vec::new())
        };
        let by_flag = option(
            "By flag",
            vec![(
                Condition::All(vec![Condition::Flag {
                    name: "unset".to_owned(),
                    value: String::new(),
                }]),
                OptionStatus::Recommended,
            )],
        );
        let facts = facts(
            vec![step("Patches", Selection::Any, vec![always, by_flag])],
            Vec::new(),
            &["Skyrim.esm"],
        );
        assert_eq!(findings(&facts), Vec::<Finding>::new());
    }

    #[test]
    fn a_choice_between_alternatives_is_not_second_guessed() {
        let facts = facts(
            vec![step(
                "Version",
                Selection::ExactlyOne,
                vec![
                    option("For City", recommended_if("City.esp", FileState::Active)),
                    option("Plain", Vec::new()),
                ],
            )],
            vec![chosen("Version", "Plain")],
            &["Skyrim.esm", "City.esp"],
        );
        assert_eq!(findings(&facts), Vec::<Finding>::new());
    }

    #[test]
    fn a_step_that_the_chosen_options_hide_is_not_examined() {
        let mut hidden = patch_step();
        hidden.visible = Some(Condition::All(vec![Condition::Flag {
            name: "patches".to_owned(),
            value: "on".to_owned(),
        }]));
        let mut switch = option("Install patches", Vec::new());
        switch.sets_flags = vec![("patches".to_owned(), "on".to_owned())];
        let steps = vec![step("Start", Selection::Any, vec![switch]), hidden];
        let active = ["Skyrim.esm", "City.esp", "Dungeons.esp"];

        // The switch was not chosen: the patch step was never shown.
        let off = facts(steps.clone(), Vec::new(), &active);
        assert_eq!(findings(&off), Vec::<Finding>::new());

        // With the switch chosen the step counts.
        let on = facts(steps, vec![chosen("Start", "Install patches")], &active);
        assert_eq!(findings(&on)[0].args["count"], ArgValue::Number(2));
    }

    #[test]
    fn the_first_rule_that_applies_decides() {
        let rules = vec![
            (
                Condition::All(vec![Condition::File {
                    file: "Conflicting.esp".to_owned(),
                    state: FileState::Active,
                }]),
                OptionStatus::NotUsable,
            ),
            (
                Condition::All(vec![Condition::File {
                    file: "City.esp".to_owned(),
                    state: FileState::Active,
                }]),
                OptionStatus::Recommended,
            ),
        ];
        let steps = vec![step("Patches", Selection::Any, vec![option("City", rules)])];

        let blocked = facts(
            steps.clone(),
            Vec::new(),
            &["Skyrim.esm", "City.esp", "Conflicting.esp"],
        );
        assert_eq!(findings(&blocked), Vec::<Finding>::new());

        let free = facts(steps, Vec::new(), &["Skyrim.esm", "City.esp"]);
        assert_eq!(findings(&free).len(), 1);
    }

    #[test]
    fn without_a_load_order_nothing_is_reported() {
        let mut facts = facts(
            vec![step(
                "Patches",
                Selection::Any,
                vec![option(
                    "For missing",
                    recommended_if("Gone.esp", FileState::Missing),
                )],
            )],
            Vec::new(),
            &["Skyrim.esm"],
        );
        facts.plugins.clear();
        assert_eq!(findings(&facts), Vec::<Finding>::new());
    }
}
