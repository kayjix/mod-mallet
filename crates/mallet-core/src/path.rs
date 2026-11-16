//! Normalized file locations inside a game installation.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};

/// A file location relative to the game's installation directory.
///
/// The games Mod Mallet targets treat paths case-insensitively, and mods mix
/// `/` and `\` freely. `GamePath` normalizes both, so two spellings of the
/// same file compare equal and hash alike:
///
/// ```
/// use mallet_core::GamePath;
///
/// assert_eq!(
///     GamePath::new(r"Data\SKSE\Plugins\Example.dll"),
///     GamePath::new("data/skse/plugins/example.dll"),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, JsonSchema)]
#[serde(transparent)]
pub struct GamePath(String);

impl GamePath {
    /// Normalizes `raw`: lower case, `/` as the only separator, and no empty
    /// or `.` segments.
    pub fn new(raw: &str) -> Self {
        let mut normalized = String::with_capacity(raw.len());
        let segments = raw
            .split(['/', '\\'])
            .filter(|segment| !segment.is_empty() && *segment != ".");
        for segment in segments {
            if !normalized.is_empty() {
                normalized.push('/');
            }
            normalized.push_str(&segment.to_lowercase());
        }
        Self(normalized)
    }

    /// The normalized path, for example `data/skse/plugins/example.dll`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last segment of the path.
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }

    /// The path of `child` below this path.
    #[must_use]
    pub fn join(&self, child: &str) -> Self {
        Self::new(&format!("{}/{child}", self.0))
    }

    /// The part of this path below `base`, or `None` if it lies elsewhere.
    ///
    /// A path is not considered to be below itself.
    pub fn strip_base(&self, base: &Self) -> Option<Self> {
        if base.0.is_empty() {
            return Some(self.clone());
        }
        let rest = self.0.strip_prefix(base.as_str())?;
        let rest = rest.strip_prefix('/')?;
        Some(Self(rest.to_owned()))
    }
}

impl fmt::Display for GamePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for GamePath {
    /// Normalizes on the way in, so a hand-written or older file can never
    /// introduce a second spelling of the same path.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(|raw| Self::new(&raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_separators() {
        let path = GamePath::new(r"Data\SKSE//Plugins\.\Example.DLL");
        assert_eq!(path.as_str(), "data/skse/plugins/example.dll");
    }

    #[test]
    fn leading_and_trailing_separators_are_dropped() {
        assert_eq!(GamePath::new(r"\Data\").as_str(), "data");
        assert_eq!(GamePath::new("").as_str(), "");
    }

    #[test]
    fn file_name_is_the_last_segment() {
        assert_eq!(GamePath::new("data/a/b.esp").file_name(), "b.esp");
        assert_eq!(GamePath::new("b.esp").file_name(), "b.esp");
    }

    #[test]
    fn join_normalizes_the_child() {
        let dir = GamePath::new("data/skse/plugins");
        assert_eq!(
            dir.join("Example.dll").as_str(),
            "data/skse/plugins/example.dll"
        );
        assert_eq!(GamePath::new("").join("Data").as_str(), "data");
    }

    #[test]
    fn strip_base_requires_a_whole_segment() {
        let path = GamePath::new("game/data/x.esp");
        assert_eq!(
            path.strip_base(&GamePath::new("game")),
            Some(GamePath::new("data/x.esp"))
        );
        assert_eq!(path.strip_base(&GamePath::new("ga")), None);
        assert_eq!(path.strip_base(&GamePath::new("other")), None);
        assert_eq!(path.strip_base(&path), None);
    }

    #[test]
    fn deserializing_normalizes() {
        let path: GamePath = serde_json::from_str(r#""Data\\Example.ESP""#).unwrap();
        assert_eq!(path.as_str(), "data/example.esp");
    }
}
