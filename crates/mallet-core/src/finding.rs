//! Findings: the problems Mod Mallet reports.

use std::collections::BTreeMap;
use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Action, GamePath, KindInfo, ModRef};

/// How much a finding matters to a working game.
///
/// The order is from most to least important, so sorting by severity puts
/// errors first.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Something does not work.
    Error,
    /// Something probably looks or behaves wrong.
    Warning,
    /// Worth knowing, nothing to fix.
    Info,
}

/// How sure Mod Mallet is that a finding describes a real problem
/// (ADR 0007).
///
/// Only [`Confidence::Certain`] findings may ever be fixed without asking.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// A fact read from the setup, such as a DLL the game refused to load.
    Certain,
    /// A heuristic that is usually right, such as two overlapping buildings.
    Likely,
    /// Knowledge from a rule or a text that may not apply to this setup.
    Hint,
}

/// What a finding is about.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Subject {
    /// A file inside the game installation.
    File {
        /// Where the file lies.
        path: GamePath,
    },
    /// A mod as a whole, as a mod manager knows it.
    Mod {
        /// The manager that knows the mod, for example `vortex`.
        source: String,
        /// The manager's own identifier of the mod.
        id: String,
    },
    /// One thing taking precedence over another, for example the loose
    /// files of a mod over the archive of a second mod.
    Override {
        /// What the game ends up using.
        winner: Party,
        /// What is replaced.
        loser: Party,
    },
    /// Two things that clash, neither of which takes precedence, such as
    /// two mods that place objects in the same spot. The two are in a fixed
    /// order that does not depend on the load order.
    Pair {
        /// The first of the two.
        first: Party,
        /// The second of the two.
        second: Party,
    },
    /// The setup as a whole.
    Setup,
}

impl Subject {
    /// A textual identity that does not depend on the load order, the
    /// machine or the language (ADR 0007).
    pub fn identity(&self) -> String {
        match self {
            Self::File { path } => format!("file:{path}"),
            Self::Mod { source, id } => format!("mod:{source}:{id}"),
            Self::Override { winner, loser } => {
                format!("override:{}>{}", winner.identity(), loser.identity())
            }
            Self::Pair { first, second } => {
                format!("pair:{}+{}", first.identity(), second.identity())
            }
            Self::Setup => "setup".to_owned(),
        }
    }
}

/// One side of a [`Subject::Override`] or a [`Subject::Pair`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Party {
    /// A file inside the game installation, such as a plugin or an archive.
    File {
        /// Where the file lies.
        path: GamePath,
    },
    /// The files a mod manager deployed for one mod.
    Mod {
        /// The manager that knows the mod, for example `vortex`.
        source: String,
        /// The manager's own identifier of the mod.
        id: String,
    },
    /// Files in the game that no known mod manager put there: the game's
    /// own files and anything installed by hand.
    Unmanaged,
}

impl Party {
    /// The party a mod, or the absence of one, stands for.
    pub fn from_provider(provider: Option<&ModRef>) -> Self {
        provider.map_or(Self::Unmanaged, |provider| Self::Mod {
            source: provider.source.clone(),
            id: provider.id.clone(),
        })
    }

    fn identity(&self) -> String {
        match self {
            Self::File { path } => format!("file:{path}"),
            Self::Mod { source, id } => format!("mod:{source}:{id}"),
            Self::Unmanaged => "unmanaged".to_owned(),
        }
    }
}

/// A value substituted into the text of a finding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ArgValue {
    /// A piece of text such as a file name.
    Text(String),
    /// A whole number such as a count.
    Number(i64),
}

impl From<&str> for ArgValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for ArgValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<i64> for ArgValue {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

/// The identity of a finding across scans.
///
/// It is derived from the kind and the subject only, so a decision such as
/// "ignore this" still applies after the load order changed or the mods were
/// re-deployed (ADR 0007).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct FindingId(String);

impl FindingId {
    /// Derives the identity of a finding of `kind` about `subject`.
    pub fn derive(kind: &str, subject: &Subject) -> Self {
        Self(format!("{kind}:{}", subject.identity()))
    }

    /// The identity as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FindingId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One problem found in a setup.
///
/// A finding carries no prose. It names its [kind](crate::kinds) and the
/// arguments for that kind's message; the text is produced later, in the
/// reader's language (ADR 0007).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Stable identity of this finding.
    pub id: FindingId,
    /// Identifier of the finding's kind, for example
    /// `extender.plugin-incompatible`.
    pub kind: String,
    /// How much the finding matters.
    pub severity: Severity,
    /// How sure Mod Mallet is.
    pub confidence: Confidence,
    /// What the finding is about.
    pub subject: Subject,
    /// Values for the placeholders of the kind's message.
    pub args: BTreeMap<String, ArgValue>,
    /// The mod the subject comes from, if a mod manager knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ModRef>,
    /// What can be done about the finding. Empty if Mod Mallet can only
    /// describe the problem.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

impl Finding {
    /// Starts a finding of `kind` about `subject`.
    pub fn new(kind: &KindInfo, subject: Subject) -> Self {
        Self {
            id: FindingId::derive(kind.id, &subject),
            kind: kind.id.to_owned(),
            severity: kind.severity,
            confidence: kind.confidence,
            subject,
            args: BTreeMap::new(),
            provider: None,
            actions: Vec::new(),
        }
    }

    /// Adds a value for the placeholder `name` of the kind's message.
    #[must_use]
    pub fn with_arg(mut self, name: &str, value: impl Into<ArgValue>) -> Self {
        self.args.insert(name.to_owned(), value.into());
        self
    }

    /// Offers an action that addresses the finding.
    #[must_use]
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Records which mod the subject comes from.
    #[must_use]
    pub fn with_provider(mut self, provider: Option<ModRef>) -> Self {
        self.provider = provider;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds;

    fn sample() -> Finding {
        Finding::new(
            &kinds::EXTENDER_PLUGIN_INCOMPATIBLE,
            Subject::File {
                path: GamePath::new(r"Data\SKSE\Plugins\Example.dll"),
            },
        )
        .with_arg("file", "Example.dll")
        .with_arg("gameVersion", "1.7.104")
    }

    #[test]
    fn id_depends_only_on_kind_and_subject() {
        let other_spelling = Finding::new(
            &kinds::EXTENDER_PLUGIN_INCOMPATIBLE,
            Subject::File {
                path: GamePath::new("data/skse/plugins/EXAMPLE.DLL"),
            },
        )
        .with_arg("file", "something else");

        assert_eq!(sample().id, other_spelling.id);
        assert_eq!(
            sample().id.as_str(),
            "extender.plugin-incompatible:file:data/skse/plugins/example.dll"
        );
    }

    #[test]
    fn severity_and_confidence_come_from_the_kind() {
        let finding = sample();
        assert_eq!(
            finding.severity,
            kinds::EXTENDER_PLUGIN_INCOMPATIBLE.severity
        );
        assert_eq!(
            finding.confidence,
            kinds::EXTENDER_PLUGIN_INCOMPATIBLE.confidence
        );
    }

    #[test]
    fn an_override_is_identified_by_both_sides() {
        let subject = Subject::Override {
            winner: Party::Mod {
                source: "vortex".to_owned(),
                id: "Flora 1.0".to_owned(),
            },
            loser: Party::File {
                path: GamePath::new(r"Data\Grass.bsa"),
            },
        };
        assert_eq!(
            subject.identity(),
            "override:mod:vortex:Flora 1.0>file:data/grass.bsa"
        );
        assert_eq!(Party::from_provider(None), Party::Unmanaged);
    }

    #[test]
    fn errors_sort_before_notes() {
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Info);
    }

    #[test]
    fn survives_a_json_round_trip() {
        let finding = sample();
        let json = serde_json::to_string(&finding).unwrap();
        assert_eq!(serde_json::from_str::<Finding>(&json).unwrap(), finding);
    }

    #[test]
    fn json_uses_the_documented_field_names() {
        let json = serde_json::to_value(sample()).unwrap();
        assert_eq!(json["kind"], "extender.plugin-incompatible");
        assert_eq!(json["severity"], "error");
        assert_eq!(json["confidence"], "certain");
        assert_eq!(json["subject"]["type"], "file");
        assert_eq!(json["args"]["gameVersion"], "1.7.104");
        assert!(json.get("provider").is_none());
    }
}
