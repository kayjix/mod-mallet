//! Mod Mallet's texts.
//!
//! Findings carry no prose, only a kind and arguments (ADR 0007). This crate
//! turns them into sentences in the reader's language. Both the command line
//! and the desktop app go through it, so a finding reads the same everywhere.
//!
//! The texts are [Fluent](https://projectfluent.org) files under `locales/`,
//! one directory per language, compiled into the binary. English is the
//! source language. A test fails if any language lacks a message that
//! another one has, or if a kind declared in `mallet-core` has no texts.

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use mallet_core::duel::{ModDuel, SuggestionReason, Topic};
use mallet_core::{
    Action, ActionOutcome, ActionStatus, ArgValue, Confidence, Finding, GameInfo, InstalledPatch,
    KindInfo, LocationOrigin, LocationRole, ModInfo, OptionToTick, ScanLocation, ScanReport,
    Severity, Tool, ToolNeed, ToolState, WorldMap,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use unic_langid::LanguageIdentifier;

/// A language Mod Mallet can speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    /// English, the source language.
    #[default]
    En,
    /// German.
    De,
}

impl Locale {
    /// Every supported language.
    pub const ALL: [Self; 2] = [Self::En, Self::De];

    /// The language code, for example `de`.
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }

    /// Finds the supported language for a tag such as `de`, `de-DE` or
    /// `en_US.UTF-8`.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let language = tag
            .split(['-', '_', '.'])
            .next()?
            .trim()
            .to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|locale| locale.code() == language)
    }

    /// The Fluent sources of this language.
    fn sources(self) -> [&'static str; 2] {
        match self {
            Self::En => [
                include_str!("../locales/en/findings.ftl"),
                include_str!("../locales/en/ui.ftl"),
            ],
            Self::De => [
                include_str!("../locales/de/findings.ftl"),
                include_str!("../locales/de/ui.ftl"),
            ],
        }
    }
}

/// The texts that describe a kind of finding in general.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindText {
    /// One line naming the problem.
    pub title: String,
    /// What the problem is and why it matters.
    pub explanation: String,
    /// What to do about it.
    pub remedy: String,
}

/// A finding together with its texts in one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedFinding {
    /// The finding itself.
    pub finding: Finding,
    /// One line naming the problem.
    pub title: String,
    /// This finding in a few words, for a list in which each finding has
    /// one line.
    pub headline: String,
    /// What exactly is wrong in this setup.
    pub summary: String,
    /// What the problem is and why it matters.
    pub explanation: String,
    /// What to do about it.
    pub remedy: String,
    /// The severity in words.
    pub severity_label: String,
    /// The confidence in words.
    pub confidence_label: String,
    /// What can be done about the finding, in words.
    pub actions: Vec<RenderedAction>,
}

/// An action together with its texts in one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedAction {
    /// The action itself, to hand back when the user chooses it.
    pub action: Action,
    /// A few words for a button.
    pub label: String,
    /// What will happen, in a sentence or two.
    pub description: String,
    /// The options to tick, each with the page it is on in words.
    pub guide: Vec<RenderedTick>,
}

/// An option to tick in an installer, with its page in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedTick {
    /// The option and where it is.
    pub tick: OptionToTick,
    /// The page, such as "Page 5 of 7".
    pub page_label: String,
}

/// What became of an action, in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedOutcome {
    /// The outcome itself.
    pub outcome: ActionOutcome,
    /// What happened and what to do next.
    pub message: String,
}

/// How things stand between Mod Mallet and the mod manager, in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedLink {
    /// Whether the link is installed in the mod manager.
    pub installed: bool,
    /// Whether the mod manager is answering through it.
    pub running: bool,
    /// Whether the mod manager has to be restarted to load the installed
    /// version of the link.
    pub restart_needed: bool,
    /// The state in a sentence.
    pub message: String,
    /// Why the link exists and what it does, for asking the user before
    /// installing it.
    pub explanation: String,
}

/// A duel between two mods, in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedDuel {
    /// The duel itself.
    pub duel: ModDuel,
    /// Each topic as a few words, such as "34 faces", in the duel's order.
    pub topic_labels: Vec<String>,
    /// Why the suggested side is suggested, if one is.
    pub suggestion: Option<String>,
}

/// A program or file the setup needs, with its name and purpose in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedTool {
    /// The tool itself.
    pub tool: Tool,
    /// Its name.
    pub name: String,
    /// What it is for, in a sentence or two.
    pub purpose: String,
    /// Its state in a word or two.
    pub state_label: String,
    /// How much the setup depends on it, in a word.
    pub need_label: String,
}

/// Everything the setup needs besides its mods, in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedToolbox {
    /// The tools, in the order they were looked at.
    pub tools: Vec<RenderedTool>,
    /// How many of them want the user's attention.
    pub attention: u32,
}

/// A place the scan looked at, with its role and origin in words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedLocation {
    /// The place itself.
    pub location: ScanLocation,
    /// What the place is for.
    pub role_label: String,
    /// How Mod Mallet came to look there.
    pub origin_label: String,
}

/// The places a scan reads, in words, for showing them before anything is
/// read and for changing them later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPlaces {
    /// Whether the user has agreed to these places before.
    pub confirmed: bool,
    /// The places, each with its role and origin in words. A place with an
    /// empty path could not be found and has to be named by the user.
    pub places: Vec<RenderedLocation>,
}

/// A scan report together with its texts in one language.
///
/// This is what the desktop app receives. Its JSON form is described by
/// `schemas/rendered-report.schema.json`, which is generated from this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderedReport {
    /// The language of the texts.
    pub locale: Locale,
    /// When the scan ran, in seconds since 1970-01-01 UTC.
    pub scanned_at: u64,
    /// The game that was scanned.
    pub game: GameInfo,
    /// How many script extender plugins were examined.
    pub extender_plugins_checked: u32,
    /// How many active plugins were read.
    pub plugins_checked: u32,
    /// How many archives were read.
    pub archives_checked: u32,
    /// How many mod installers were read.
    pub installers_checked: u32,
    /// How many replacement meshes were compared with the mesh they
    /// replace.
    pub meshes_checked: u32,
    /// The places the scan looked at.
    pub locations: Vec<RenderedLocation>,
    /// What the mod manager knows about the installed mods.
    pub mods: Vec<ModInfo>,
    /// Where mods place objects, and where they collide.
    pub map: WorldMap,
    /// Whether Base Object Swapper, which patches need, is installed.
    pub object_swapper: bool,
    /// Mod Mallet's patch as it is in the game now, if there is one.
    pub installed_patch: Option<InstalledPatch>,
    /// What two mods disagree about, pair by pair.
    pub duels: Vec<RenderedDuel>,
    /// The findings with their texts, most severe first.
    pub findings: Vec<RenderedFinding>,
}

/// The texts of one language.
pub struct Messages {
    locale: Locale,
    bundle: FluentBundle<FluentResource>,
}

impl Messages {
    /// Loads the texts of `locale`.
    pub fn new(locale: Locale) -> Self {
        Self::load(locale).0
    }

    /// Loads the texts and also returns everything that is wrong with them.
    /// Production code ignores the problems; the tests insist on none.
    fn load(locale: Locale) -> (Self, Vec<String>) {
        let language: LanguageIdentifier = locale.code().parse().unwrap_or_default();
        let mut bundle = FluentBundle::new(vec![language]);
        // Fluent wraps arguments in invisible direction marks by default.
        // They help in mixed left-to-right and right-to-left text but break
        // copying a file name out of a terminal.
        bundle.set_use_isolating(false);

        let mut problems = Vec::new();
        for source in locale.sources() {
            let resource = match FluentResource::try_new(source.to_owned()) {
                Ok(resource) => resource,
                Err((resource, errors)) => {
                    problems.extend(errors.iter().map(|error| format!("{error:?}")));
                    resource
                }
            };
            if let Err(errors) = bundle.add_resource(resource) {
                problems.extend(errors.iter().map(|error| format!("{error:?}")));
            }
        }
        (Self { locale, bundle }, problems)
    }

    /// The language of these texts.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// The text of message `id` with `args` filled in.
    ///
    /// A message that does not exist yields its own id, so a gap shows up as
    /// an odd word in the output instead of a crash.
    pub fn text(&self, id: &str, args: &[(&str, ArgValue)]) -> String {
        let args: Vec<_> = args.iter().map(|(name, value)| (*name, value)).collect();
        self.format(id, None, &args).0
    }

    /// The general texts of a kind of finding.
    pub fn kind(&self, kind: &KindInfo) -> KindText {
        self.kind_by_id(kind.id)
    }

    /// The severity in words.
    pub fn severity(&self, severity: Severity) -> String {
        let id = match severity {
            Severity::Error => "severity-error",
            Severity::Warning => "severity-warning",
            Severity::Info => "severity-info",
        };
        self.text(id, &[])
    }

    /// The confidence in words.
    pub fn confidence(&self, confidence: Confidence) -> String {
        let id = match confidence {
            Confidence::Certain => "confidence-certain",
            Confidence::Likely => "confidence-likely",
            Confidence::Hint => "confidence-hint",
        };
        self.text(id, &[])
    }

    /// Puts a finding into words.
    pub fn render(&self, finding: &Finding) -> RenderedFinding {
        let KindText {
            title,
            explanation,
            remedy,
        } = self.kind_by_id(&finding.kind);
        let args: Vec<_> = finding
            .args
            .iter()
            .map(|(name, value)| (name.as_str(), value))
            .collect();
        let id = message_id(&finding.kind);
        let (summary, _) = self.format(&id, Some("summary"), &args);
        let (headline, _) = self.format(&id, Some("headline"), &args);

        RenderedFinding {
            finding: finding.clone(),
            title,
            headline,
            summary,
            explanation,
            remedy,
            severity_label: self.severity(finding.severity),
            confidence_label: self.confidence(finding.confidence),
            actions: finding
                .actions
                .iter()
                .map(|action| self.render_action(action))
                .collect(),
        }
    }

    /// Puts an action into words.
    pub fn render_action(&self, action: &Action) -> RenderedAction {
        let no_ticks = Vec::new();
        let (id, args, ticks) = match action {
            Action::ReinstallMod { mod_ref, tick } => (
                "action-reinstall-mod",
                vec![("mod", ArgValue::from(mod_ref.name.as_str()))],
                tick,
            ),
            Action::HideObjects { objects } => (
                "action-hide-objects",
                vec![(
                    "count",
                    ArgValue::Number(i64::try_from(objects.len()).unwrap_or(i64::MAX)),
                )],
                &no_ticks,
            ),
            Action::LoadAfter { plugin, after } => (
                "action-load-after",
                vec![
                    ("plugin", ArgValue::from(plugin.as_str())),
                    ("after", ArgValue::from(after.as_str())),
                ],
                &no_ticks,
            ),
            Action::DisableMod { mod_ref } => (
                "action-disable-mod",
                vec![("mod", ArgValue::from(mod_ref.name.as_str()))],
                &no_ticks,
            ),
        };
        let args: Vec<_> = args.iter().map(|(name, value)| (*name, value)).collect();
        RenderedAction {
            action: action.clone(),
            label: self.format(id, None, &args).0,
            description: self.format(id, Some("description"), &args).0,
            guide: ticks
                .iter()
                .map(|tick| RenderedTick {
                    tick: tick.clone(),
                    page_label: self.text(
                        "guide-page",
                        &[
                            ("page", ArgValue::Number(i64::from(tick.page))),
                            ("pages", ArgValue::Number(i64::from(tick.pages))),
                        ],
                    ),
                })
                .collect(),
        }
    }

    /// Puts the outcome of an action into words.
    pub fn render_outcome(&self, outcome: &ActionOutcome) -> RenderedOutcome {
        let id = match outcome.status {
            ActionStatus::Started => "outcome-started",
            ActionStatus::Installing => "outcome-installing",
            ActionStatus::Applied => "outcome-applied",
            ActionStatus::LinkMissing => "outcome-link-missing",
            ActionStatus::ManagerNotRunning => "outcome-manager-not-running",
            ActionStatus::NoAnswer => "outcome-no-answer",
            ActionStatus::WrongGame => "outcome-wrong-game",
            ActionStatus::UnknownMod => "outcome-unknown-mod",
            ActionStatus::ArchiveMissing => "outcome-archive-missing",
            ActionStatus::Unsupported => "outcome-unsupported",
            ActionStatus::Failed => "outcome-failed",
        };
        let detail = ArgValue::from(outcome.detail.as_deref().unwrap_or("?"));
        RenderedOutcome {
            outcome: outcome.clone(),
            message: self.text(id, &[("detail", detail)]),
        }
    }

    /// Puts the state of the link to the mod manager into words.
    pub fn render_link(
        &self,
        installed: bool,
        running: bool,
        restart_needed: bool,
    ) -> RenderedLink {
        let id = match (installed, running) {
            (true, true) if restart_needed => "link-restart-needed",
            (true, true) => "link-ready",
            (true, false) => "link-not-running",
            (false, _) => "link-missing",
        };
        RenderedLink {
            installed,
            running,
            restart_needed,
            message: self.text(id, &[]),
            explanation: self.text("link-explanation", &[]),
        }
    }

    /// Puts a duel between two mods into words.
    pub fn render_duel(&self, duel: &ModDuel) -> RenderedDuel {
        let topic_id = |topic: Topic| match topic {
            Topic::Faces => "faces",
            Topic::Textures => "textures",
            Topic::Meshes => "meshes",
            Topic::Objects => "objects",
            Topic::Contained => "contained",
        };
        let topic_labels = duel
            .topics
            .iter()
            .map(|topic| {
                self.text(
                    &format!("duel-topic-{}", topic_id(topic.topic)),
                    &[("count", ArgValue::Number(i64::from(topic.count)))],
                )
            })
            .collect();
        let suggestion = duel.suggestion.as_ref().map(|suggestion| {
            let [first, second] = &duel.sides;
            let (winner, other) = if suggestion.side == 0 {
                (first, second)
            } else {
                (second, first)
            };
            let names = [
                ("winner", ArgValue::from(winner.name.as_str())),
                ("other", ArgValue::from(other.name.as_str())),
            ];
            match suggestion.reason {
                SuggestionReason::BuiltOn => self.text("duel-suggest-built-on", &names),
                SuggestionReason::Specialist => {
                    let subject = self.text(
                        &format!(
                            "duel-subject-{}",
                            topic_id(suggestion.topic.unwrap_or(Topic::Objects))
                        ),
                        &[],
                    );
                    let [winner, other] = names;
                    self.text(
                        "duel-suggest-specialist",
                        &[winner, other, ("subject", ArgValue::from(subject.as_str()))],
                    )
                }
            }
        });
        RenderedDuel {
            duel: duel.clone(),
            topic_labels,
            suggestion,
        }
    }

    /// Puts what the setup needs besides its mods into words.
    pub fn render_toolbox(&self, tools: &[Tool]) -> RenderedToolbox {
        let rendered: Vec<RenderedTool> = tools
            .iter()
            .map(|tool| RenderedTool {
                tool: tool.clone(),
                name: self.text(&format!("tool-{}-name", tool.id), &[]),
                purpose: self.text(&format!("tool-{}-purpose", tool.id), &[]),
                state_label: self.text(
                    match tool.state {
                        ToolState::Ready => "tool-state-ready",
                        ToolState::Missing => "tool-state-missing",
                        ToolState::Outdated => "tool-state-outdated",
                        ToolState::Stopped => "tool-state-stopped",
                    },
                    &[],
                ),
                need_label: self.text(
                    match tool.need {
                        ToolNeed::Required => "tool-need-required",
                        ToolNeed::Recommended => "tool-need-recommended",
                        ToolNeed::Optional => "tool-need-optional",
                    },
                    &[],
                ),
            })
            .collect();
        let attention = tools.iter().filter(|tool| tool.wants_attention()).count();
        RenderedToolbox {
            tools: rendered,
            attention: u32::try_from(attention).unwrap_or(u32::MAX),
        }
    }

    /// Puts a place the scan looked at into words.
    pub fn render_location(&self, location: &ScanLocation) -> RenderedLocation {
        let role = match location.role {
            LocationRole::GameDir => "location-game-dir",
            LocationRole::PluginList => "location-plugin-list",
            LocationRole::ExtenderLog => "location-extender-log",
            LocationRole::ManagerState => "location-manager-state",
            LocationRole::Downloads => "location-downloads",
            LocationRole::Unpacker => "location-unpacker",
            LocationRole::DeploymentManifest => "location-deployment-manifest",
            LocationRole::Masterlist => "location-masterlist",
            LocationRole::CrashLogs => "location-crash-logs",
        };
        let origin = match location.origin {
            LocationOrigin::Given => "origin-given",
            LocationOrigin::Registry => "origin-registry",
            LocationOrigin::Standard => "origin-standard",
            LocationOrigin::Steam => "origin-steam",
            LocationOrigin::Gog => "origin-gog",
        };
        RenderedLocation {
            location: location.clone(),
            role_label: self.text(role, &[]),
            origin_label: self.text(origin, &[]),
        }
    }

    /// Puts the places a scan would read into words.
    pub fn render_places(&self, confirmed: bool, places: &[ScanLocation]) -> RenderedPlaces {
        RenderedPlaces {
            confirmed,
            places: places
                .iter()
                .map(|place| self.render_location(place))
                .collect(),
        }
    }

    /// Puts a whole report into words.
    pub fn render_report(&self, report: &ScanReport) -> RenderedReport {
        RenderedReport {
            locale: self.locale,
            scanned_at: report.scanned_at,
            game: report.game.clone(),
            extender_plugins_checked: report.extender_plugins_checked,
            plugins_checked: report.plugins_checked,
            archives_checked: report.archives_checked,
            installers_checked: report.installers_checked,
            meshes_checked: report.meshes_checked,
            locations: report
                .locations
                .iter()
                .map(|location| self.render_location(location))
                .collect(),
            findings: report
                .findings
                .iter()
                .map(|finding| self.render(finding))
                .collect(),
            mods: report.mods.clone(),
            map: report.map.clone(),
            object_swapper: report.object_swapper,
            installed_patch: report.installed_patch.clone(),
            duels: {
                // The mod manager knows the proper names; deployments only
                // know folders.
                let names: std::collections::HashMap<&str, &str> = report
                    .mods
                    .iter()
                    .map(|info| (info.mod_ref.id.as_str(), info.mod_ref.name.as_str()))
                    .collect();
                report
                    .duels
                    .iter()
                    .map(|duel| {
                        let mut duel = duel.clone();
                        for side in &mut duel.sides {
                            if let Some(name) = side
                                .mod_ref
                                .as_ref()
                                .and_then(|mod_ref| names.get(mod_ref.id.as_str()))
                            {
                                side.name = (*name).to_owned();
                            }
                        }
                        self.render_duel(&duel)
                    })
                    .collect()
            },
        }
    }

    fn kind_by_id(&self, kind_id: &str) -> KindText {
        let id = message_id(kind_id);
        KindText {
            title: self.format(&id, None, &[]).0,
            explanation: self.format(&id, Some("explanation"), &[]).0,
            remedy: self.format(&id, Some("remedy"), &[]).0,
        }
    }

    /// Formats a message or one of its attributes. Returns the text and
    /// everything that went wrong while producing it.
    fn format(
        &self,
        id: &str,
        attribute: Option<&str>,
        args: &[(&str, &ArgValue)],
    ) -> (String, Vec<String>) {
        let name = attribute.map_or_else(|| id.to_owned(), |attribute| format!("{id}.{attribute}"));

        let Some(message) = self.bundle.get_message(id) else {
            return (name.clone(), vec![format!("message `{name}` is missing")]);
        };
        let pattern = match attribute {
            None => message.value(),
            Some(attribute) => message.get_attribute(attribute).map(|found| found.value()),
        };
        let Some(pattern) = pattern else {
            return (name.clone(), vec![format!("message `{name}` is missing")]);
        };

        let mut fluent_args = FluentArgs::new();
        for (arg_name, value) in args {
            let value = match value {
                ArgValue::Text(text) => FluentValue::from(text.as_str()),
                ArgValue::Number(number) => FluentValue::from(*number),
            };
            fluent_args.set(*arg_name, value);
        }

        let mut errors = Vec::new();
        let text = self
            .bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned();
        let problems = errors
            .iter()
            .map(|error| format!("message `{name}`: {error:?}"))
            .collect();
        (text, problems)
    }
}

/// The Fluent message id of a kind: Fluent ids cannot contain dots.
fn message_id(kind_id: &str) -> String {
    kind_id.replace('.', "-")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use mallet_core::{GamePath, Subject, kinds};

    use super::*;

    /// The ids of all messages and attributes in a Fluent source, as
    /// `message` and `message.attribute`. Good enough for comparing catalogs;
    /// it is not a Fluent parser.
    fn entries(source: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let mut current = String::new();
        for line in source.lines() {
            let Some((before_equals, _)) = line.split_once('=') else {
                continue;
            };
            let name = before_equals.trim();
            let is_identifier = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
            if !is_identifier {
                continue;
            }
            if line.starts_with(char::is_whitespace) {
                if let Some(attribute) = name.strip_prefix('.') {
                    found.insert(format!("{current}.{attribute}"));
                }
            } else {
                name.clone_into(&mut current);
                found.insert(current.clone());
            }
        }
        found
    }

    fn all_entries(locale: Locale) -> BTreeSet<String> {
        locale.sources().into_iter().flat_map(entries).collect()
    }

    fn sample_finding() -> Finding {
        Finding::new(
            &kinds::EXTENDER_PLUGIN_INCOMPATIBLE,
            Subject::File {
                path: GamePath::new("data/skse/plugins/old.dll"),
            },
        )
        .with_arg("file", "Old.dll")
        .with_arg("gameVersion", "1.7.104")
    }

    #[test]
    fn every_catalog_loads_without_problems() {
        for locale in Locale::ALL {
            let (_, problems) = Messages::load(locale);
            assert!(problems.is_empty(), "{}: {problems:?}", locale.code());
        }
    }

    #[test]
    fn every_language_has_the_same_messages() {
        let english = all_entries(Locale::En);
        assert_ne!(english, BTreeSet::new());
        for locale in Locale::ALL {
            let other = all_entries(locale);
            let missing: Vec<_> = english.difference(&other).collect();
            let extra: Vec<_> = other.difference(&english).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{}: missing {missing:?}, not in English {extra:?}",
                locale.code()
            );
        }
    }

    #[test]
    fn every_kind_has_complete_texts_in_every_language() {
        for locale in Locale::ALL {
            let messages = Messages::new(locale);
            for kind in kinds::ALL {
                let id = message_id(kind.id);
                let placeholder = ArgValue::from("x");
                let args: Vec<_> = kind.args.iter().map(|name| (*name, &placeholder)).collect();

                let checks = [
                    messages.format(&id, None, &[]),
                    messages.format(&id, Some("explanation"), &[]),
                    messages.format(&id, Some("remedy"), &[]),
                    messages.format(&id, Some("summary"), &args),
                    messages.format(&id, Some("headline"), &args),
                ];
                for (text, problems) in checks {
                    assert!(
                        problems.is_empty(),
                        "{} in {}: {problems:?}",
                        kind.id,
                        locale.code()
                    );
                    assert_ne!(text.trim(), "");
                }
            }
        }
    }

    #[test]
    fn a_finding_is_put_into_words() {
        let rendered = Messages::new(Locale::En).render(&sample_finding());
        assert_eq!(
            rendered.title,
            "Script extender plugin does not support this game version"
        );
        assert_eq!(
            rendered.summary,
            "Old.dll was switched off because it was not built for game version 1.7.104."
        );
        assert_eq!(
            rendered.headline,
            "Old.dll does not support game version 1.7.104"
        );
        assert_eq!(rendered.severity_label, "Error");
        assert_eq!(rendered.confidence_label, "certain");
        assert_ne!(rendered.explanation, "");
        assert_ne!(rendered.remedy, "");
    }

    #[test]
    fn the_same_finding_reads_differently_in_german() {
        let rendered = Messages::new(Locale::De).render(&sample_finding());
        assert_eq!(
            rendered.summary,
            "Old.dll wurde abgeschaltet, weil es nicht für die Spielversion 1.7.104 gebaut ist."
        );
        assert_eq!(rendered.severity_label, "Fehler");
    }

    #[test]
    fn actions_and_their_outcomes_are_put_into_words_in_every_language() {
        let action = Action::ReinstallMod {
            mod_ref: mallet_core::ModRef {
                source: "vortex".to_owned(),
                id: "Patch Hub 1.0".to_owned(),
                name: "Patch Hub".to_owned(),
            },
            tick: vec![OptionToTick {
                page: 5,
                pages: 7,
                step: "Patches".to_owned(),
                group: "Cities".to_owned(),
                option: "City Overhaul".to_owned(),
                index: 2,
                description: String::new(),
            }],
        };
        let statuses = [
            ActionStatus::Started,
            ActionStatus::LinkMissing,
            ActionStatus::ManagerNotRunning,
            ActionStatus::NoAnswer,
            ActionStatus::WrongGame,
            ActionStatus::UnknownMod,
            ActionStatus::ArchiveMissing,
            ActionStatus::Unsupported,
            ActionStatus::Failed,
        ];
        for locale in Locale::ALL {
            let messages = Messages::new(locale);

            let rendered = messages.render_action(&action);
            assert!(!rendered.label.contains("action-"), "{}", rendered.label);
            assert!(rendered.description.contains("Patch Hub"));

            for status in statuses {
                let outcome = ActionOutcome {
                    status,
                    detail: Some("disk full".to_owned()),
                };
                let message = messages.render_outcome(&outcome).message;
                assert!(!message.starts_with("outcome-"), "{message}");
                assert!(!message.contains('{'), "{message}");
            }
            assert!(
                messages
                    .render_outcome(&ActionOutcome {
                        status: ActionStatus::Failed,
                        detail: Some("disk full".to_owned()),
                    })
                    .message
                    .contains("disk full")
            );

            let page = &messages.render_action(&action).guide[0].page_label;
            assert!(page.contains('5') && page.contains('7'), "{page}");
            assert!(!page.starts_with("guide-"), "{page}");

            for (installed, running, restart) in [
                (true, true, false),
                (true, true, true),
                (true, false, false),
                (false, false, false),
            ] {
                let link = messages.render_link(installed, running, restart);
                assert!(!link.message.starts_with("link-"), "{}", link.message);
                assert!(!link.explanation.starts_with("link-"));
            }
        }
    }

    #[test]
    fn a_finding_brings_its_actions_in_words() {
        let finding = sample_finding().with_action(Action::ReinstallMod {
            mod_ref: mallet_core::ModRef {
                source: "vortex".to_owned(),
                id: "Patch Hub 1.0".to_owned(),
                name: "Patch Hub".to_owned(),
            },
            tick: Vec::new(),
        });
        let rendered = Messages::new(Locale::En).render(&finding);
        assert_eq!(rendered.actions.len(), 1);
        assert_eq!(rendered.actions[0].label, "Reinstall in Vortex");
        assert_eq!(rendered.actions[0].action, finding.actions[0]);
    }

    #[test]
    fn counts_choose_singular_or_plural() {
        let messages = Messages::new(Locale::En);
        assert_eq!(
            messages.text("report-plugins-checked", &[("count", ArgValue::Number(1))]),
            "1 script extender plugin checked"
        );
        assert_eq!(
            messages.text("report-plugins-checked", &[("count", ArgValue::Number(46))]),
            "46 script extender plugins checked"
        );
    }

    #[test]
    fn an_unknown_message_yields_its_id() {
        let messages = Messages::new(Locale::En);
        assert_eq!(messages.text("no-such-message", &[]), "no-such-message");
    }

    #[test]
    fn a_finding_of_an_unknown_kind_still_renders() {
        let mut finding = sample_finding();
        finding.kind = "retired.kind".to_owned();
        let rendered = Messages::new(Locale::En).render(&finding);
        assert_eq!(rendered.title, "retired-kind");
    }

    #[test]
    fn language_tags_are_matched_by_their_language_part() {
        assert_eq!(Locale::from_tag("de"), Some(Locale::De));
        assert_eq!(Locale::from_tag("de-DE"), Some(Locale::De));
        assert_eq!(Locale::from_tag("en_US.UTF-8"), Some(Locale::En));
        assert_eq!(Locale::from_tag("fr"), None);
        assert_eq!(Locale::from_tag(""), None);
    }

    #[test]
    fn a_report_serializes_with_its_locale_code() {
        let json = serde_json::to_value(Locale::De).unwrap();
        assert_eq!(json, "de");
    }
}
