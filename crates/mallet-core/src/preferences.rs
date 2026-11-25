//! The user's preferences between mods, as the window keeps them in the
//! settings of a game (ADR 0086).
//!
//! The window learns preferences from the user's decisions; what is kept
//! here is only what the user said: the preferences they set themselves,
//! their answers to the preferences offered, whether learning is on, and
//! which decisions were made by applying a preference, so that those are
//! not learned from again. The identities are the window's; the engine
//! keeps them as they are.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What the user said about preferences between their mods, for one game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchPreferences {
    /// Whether the user switched learning off: then only the preferences
    /// they set themselves count, and nothing is offered.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub learning_off: bool,
    /// The preferences the user set themselves.
    #[serde(default)]
    pub rules: Vec<PreferenceRule>,
    /// The user's answers to preferences learned and offered, by the
    /// preference's identity.
    #[serde(default)]
    pub answers: BTreeMap<String, PreferenceAnswer>,
    /// The decisions made by applying a preference, by their key among the
    /// decisions: the identity of the preference that made each.
    #[serde(default)]
    pub applied: BTreeMap<String, String>,
}

/// A preference the user set: one mod kept over others.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRule {
    /// The mod preferred, by the name the mod manager shows.
    #[serde(rename = "mod")]
    pub mod_name: String,
    /// The one mod it is preferred over; `None` for every other mod.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub over: Option<String>,
    /// What it is preferred for (`meshes`, `textures`, `faces`, `files`,
    /// `places`); `None` for everything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
}

/// The user's answer to a preference offered.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceAnswer {
    /// What they answered.
    pub answer: PreferenceAnswerKind,
    /// How many decisions the preference rested on then, so that "not
    /// now" can be asked again once there are more.
    #[serde(default)]
    pub at: u32,
}

/// The answers to a preference offered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PreferenceAnswerKind {
    /// Applied to the open conflicts; it keeps counting.
    #[default]
    Applied,
    /// Not now: asked again once more decisions speak for it.
    Later,
    /// Never: neither offered again nor used.
    Never,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_it_wrote_and_an_empty_object() {
        let mut preferences = PatchPreferences::default();
        preferences.rules.push(PreferenceRule {
            mod_name: "Static Mesh Improvement Mod".to_owned(),
            over: None,
            topic: Some("meshes".to_owned()),
        });
        preferences.answers.insert(
            "favourite|smim|meshes".to_owned(),
            PreferenceAnswer {
                answer: PreferenceAnswerKind::Later,
                at: 15,
            },
        );
        let text = serde_json::to_string(&preferences).unwrap();
        assert!(text.contains("\"mod\":\"Static Mesh Improvement Mod\""));
        assert!(text.contains("\"answer\":\"later\""));
        assert_eq!(
            serde_json::from_str::<PatchPreferences>(&text).unwrap(),
            preferences
        );
        assert_eq!(
            serde_json::from_str::<PatchPreferences>("{}").unwrap(),
            PatchPreferences::default()
        );
    }
}
