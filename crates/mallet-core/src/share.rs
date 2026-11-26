//! Reports meant for sharing: the private parts of paths replaced by
//! placeholders, so that a report can be handed to someone who helps
//! without handing over who and where the user is (ADR 0014, ADR 0091).

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ScanReport;

/// What a placeholder in a shared report stands for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Placeholder {
    /// The directory the game is installed in.
    Game,
    /// The folder the mod manager keeps the installed mods' files in.
    Staging,
    /// The folder of a Mod Organizer 2 instance.
    Instance,
    /// The folder the mod manager keeps downloaded mod archives in.
    Downloads,
    /// The user's documents folder, where the games keep saves and logs.
    Documents,
    /// The user's local application data, `%LOCALAPPDATA%` on Windows.
    LocalAppData,
    /// The user's roaming application data, `%APPDATA%` on Windows.
    AppData,
    /// The user's home directory, `%USERPROFILE%` on Windows.
    UserProfile,
    /// A user name.
    User,
    /// The name of the computer.
    Machine,
    /// A profile of the mod manager, by its name or identifier.
    Profile,
}

impl Placeholder {
    /// Every placeholder, in the order a legend lists them.
    pub const ALL: [Self; 11] = [
        Self::Game,
        Self::Staging,
        Self::Instance,
        Self::Downloads,
        Self::Documents,
        Self::LocalAppData,
        Self::AppData,
        Self::UserProfile,
        Self::User,
        Self::Machine,
        Self::Profile,
    ];

    /// The text that takes the private part's place.
    pub fn token(self) -> &'static str {
        match self {
            Self::Game => "<game>",
            Self::Staging => "<staging>",
            Self::Instance => "<instance>",
            Self::Downloads => "<downloads>",
            Self::Documents => "<documents>",
            Self::LocalAppData => "%LOCALAPPDATA%",
            Self::AppData => "%APPDATA%",
            Self::UserProfile => "%USERPROFILE%",
            Self::User => "<user>",
            Self::Machine => "<machine>",
            Self::Profile => "<profile>",
        }
    }
}

/// A report made fit for sharing: every path in it shortened, and the
/// placeholders that took the place of what was removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SharedReport {
    /// The report, with the private parts of its paths replaced.
    pub report: ScanReport,
    /// The placeholders the report contains, in the order of
    /// [`Placeholder::ALL`].
    pub placeholders: Vec<Placeholder>,
}

/// One character of a known folder, or a run of separators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Part {
    Separator,
    Char(char),
}

/// A folder of this machine whose path is private, such as the home
/// directory.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Root {
    parts: Vec<Part>,
    placeholder: Placeholder,
}

/// A name that is private wherever it is a folder of a path, such as the
/// user name.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Name {
    chars: Vec<char>,
    placeholder: Placeholder,
}

/// Replaces the private parts of the paths in a text by placeholders.
///
/// It knows the private folders of one machine, and replaces each of them
/// wherever a text names it, in either kind of slash and in any case. It
/// also replaces the user name and the computer's name where they are a
/// folder of a path, the folder below `C:\Users` or `/home` whoever it
/// belongs to, and the profile folder of a mod manager. Everything else
/// stays as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shortener {
    roots: Vec<Root>,
    names: Vec<Name>,
}

impl Shortener {
    /// A shortener that knows no private folder yet. It still replaces the
    /// folder below `C:\Users` or `/home`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a private folder. A path of fewer than two parts, such as `C:\`
    /// or `/home`, is ignored: replacing it would hide what is not private.
    #[must_use]
    pub fn with_root(mut self, path: &str, placeholder: Placeholder) -> Self {
        let mut parts = Vec::new();
        let mut components = 0;
        let mut in_component = false;
        for c in path.trim().chars() {
            if is_separator(c) {
                if parts.last() != Some(&Part::Separator) {
                    parts.push(Part::Separator);
                }
                in_component = false;
            } else {
                if !in_component {
                    components += 1;
                    in_component = true;
                }
                parts.push(Part::Char(c));
            }
        }
        while parts.last() == Some(&Part::Separator) {
            parts.pop();
        }
        if components < 2 || self.roots.iter().any(|root| root.parts == parts) {
            return self;
        }
        self.roots.push(Root { parts, placeholder });
        // The longest folder wins: the game directory below the home
        // directory is the game directory.
        self.roots
            .sort_by_key(|root| std::cmp::Reverse(root.parts.len()));
        self
    }

    /// Adds a name that is private wherever it is a folder of a path. An
    /// empty name is ignored.
    #[must_use]
    pub fn with_name(mut self, name: &str, placeholder: Placeholder) -> Self {
        let chars: Vec<char> = name.trim().chars().collect();
        if chars.is_empty()
            || chars.iter().any(|c| is_separator(*c))
            || self.names.iter().any(|known| known.chars == chars)
        {
            return self;
        }
        self.names.push(Name { chars, placeholder });
        self.names
            .sort_by_key(|name| std::cmp::Reverse(name.chars.len()));
        self
    }

    /// Returns `text` with the private parts of its paths replaced, and
    /// adds the placeholders it used to `used`.
    pub fn shorten(&self, text: &str, used: &mut BTreeSet<Placeholder>) -> String {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut at = 0;
        while let Some(&c) = chars.get(at) {
            let previous = at.checked_sub(1).and_then(|before| chars.get(before));
            if previous.is_none_or(|p| !is_word(*p))
                && let Some((end, placeholder)) = self.root_at(&chars, at)
            {
                out.push_str(placeholder.token());
                used.insert(placeholder);
                at = end;
                continue;
            }
            if previous.is_some_and(|p| is_separator(*p))
                && let Some((end, placeholder)) = self.folder_at(&chars, at, &out)
            {
                out.push_str(placeholder.token());
                used.insert(placeholder);
                at = end;
                continue;
            }
            out.push(c);
            at += 1;
        }
        out
    }

    /// The known folder that starts at `start`, and where it ends.
    fn root_at(&self, text: &[char], start: usize) -> Option<(usize, Placeholder)> {
        self.roots.iter().find_map(|root| {
            let mut at = start;
            for part in &root.parts {
                match part {
                    Part::Separator => {
                        let from = at;
                        while text.get(at).is_some_and(|c| is_separator(*c)) {
                            at += 1;
                        }
                        if at == from {
                            return None;
                        }
                    }
                    Part::Char(expected) => {
                        if !same_letter(*expected, *text.get(at)?) {
                            return None;
                        }
                        at += 1;
                    }
                }
            }
            (ends_word(text, at) && !continues_folder(text, at)).then_some((at, root.placeholder))
        })
    }

    /// The private folder of a path that starts at `start`, right after a
    /// separator, and where it ends. `out` is what came before, already
    /// shortened.
    fn folder_at(&self, text: &[char], start: usize, out: &str) -> Option<(usize, Placeholder)> {
        for name in &self.names {
            let matches = name.chars.iter().enumerate().all(|(offset, expected)| {
                text.get(start + offset)
                    .is_some_and(|c| same_letter(*expected, *c))
            });
            let end = start + name.chars.len();
            if matches && ends_word(text, end) {
                return Some((end, name.placeholder));
            }
        }

        let before = components_before(out);
        let first = before.first().map(String::as_str);
        let second = before.get(1).map(String::as_str);
        let third = before.get(2).map(String::as_str);
        let placeholder = match (first, second) {
            // `C:\Users\someone`, `/Users/someone`.
            (Some("users"), Some(drive)) if drive.is_empty() || is_drive(drive) => {
                Placeholder::User
            }
            // `/home/someone`.
            (Some("home"), Some("")) => Placeholder::User,
            // `<instance>\profiles\name`, `Vortex\skyrimse\profiles\id`.
            (Some("profiles"), Some(owner))
                if owner == Placeholder::Instance.token() || third == Some("vortex") =>
            {
                Placeholder::Profile
            }
            _ => return None,
        };
        let end = folder_end(text, start);
        let folder: String = text
            .get(start..end)?
            .iter()
            .flat_map(|c| c.to_lowercase())
            .collect();
        let shared = ["public", "default", "default user", "all users"];
        if folder.is_empty()
            || (placeholder == Placeholder::User && shared.contains(&folder.as_str()))
        {
            return None;
        }
        Some((end, placeholder))
    }
}

/// Where the folder that starts at `start` ends: at the next separator,
/// or, in a path that ends in this folder, at the first space.
fn folder_end(text: &[char], start: usize) -> usize {
    let mut at = start;
    while let Some(&c) = text.get(at) {
        if is_separator(c) {
            return at;
        }
        if is_stop(c) {
            break;
        }
        at += 1;
    }
    (start..at)
        .find(|index| text.get(*index).is_some_and(|c| c.is_whitespace()))
        .unwrap_or(at)
}

/// The folders of the path that `out` ends in, the nearest first and in
/// lower case, up to three. An empty one means the path starts with a
/// separator there.
fn components_before(out: &str) -> Vec<String> {
    let mut components = Vec::new();
    let mut chars = out.chars().rev().peekable();
    while components.len() < 3 {
        let mut separators = 0;
        while chars.peek().is_some_and(|c| is_separator(*c)) {
            chars.next();
            separators += 1;
        }
        let mut component = Vec::new();
        while let Some(&c) = chars.peek() {
            if is_separator(c) || is_stop(c) || c.is_whitespace() {
                break;
            }
            component.push(c);
            chars.next();
        }
        if component.is_empty() {
            if separators > 0 {
                components.push(String::new());
            }
            break;
        }
        components.push(
            component
                .iter()
                .rev()
                .flat_map(|c| c.to_lowercase())
                .collect(),
        );
        if !chars.peek().is_some_and(|c| is_separator(*c)) {
            break;
        }
    }
    components
}

fn is_separator(c: char) -> bool {
    c == '\\' || c == '/'
}

/// Whether `c` may be part of a folder's name next to the letters of a
/// known one, so that the known one is not matched there.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '~' | '$')
}

/// Whether `c` ends a path in a text, such as a quotation mark.
fn is_stop(c: char) -> bool {
    matches!(
        c,
        '"' | '\'' | '`' | '\n' | '\r' | '\t' | '|' | '*' | '?' | '(' | ')' | '[' | ']' | ',' | ';'
    )
}

/// Whether a space at `at` continues the name of a folder rather than
/// ending the path: the path goes on with a separator before the line or
/// the quotation ends, as in `Skyrim Special Edition\Data`.
fn continues_folder(text: &[char], at: usize) -> bool {
    if text.get(at) != Some(&' ') {
        return false;
    }
    text.iter()
        .skip(at)
        .take_while(|c| !is_stop(**c) && **c != ':')
        .any(|c| is_separator(*c))
}

fn ends_word(text: &[char], at: usize) -> bool {
    !text.get(at).is_some_and(|c| is_word(*c))
}

fn is_drive(component: &str) -> bool {
    let mut chars = component.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(letter), Some(':'), None) if letter.is_ascii_alphabetic()
    )
}

fn same_letter(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn alice() -> Shortener {
        Shortener::new()
            .with_root(r"C:\Users\Alice", Placeholder::UserProfile)
            .with_root(r"C:\Users\Alice\AppData\Local", Placeholder::LocalAppData)
            .with_root(r"D:\Steam\steamapps\common\Skyrim", Placeholder::Game)
            .with_name("Alice", Placeholder::User)
            .with_name("ALICE-PC", Placeholder::Machine)
    }

    fn short(text: &str) -> String {
        alice().shorten(text, &mut BTreeSet::new())
    }

    #[test]
    fn known_folders_are_replaced_in_any_spelling() {
        assert_eq!(
            short(r"C:\Users\Alice\AppData\Local\Skyrim Special Edition\plugins.txt"),
            r"%LOCALAPPDATA%\Skyrim Special Edition\plugins.txt"
        );
        assert_eq!(short("c:/users/alice/Documents"), "%USERPROFILE%/Documents");
        assert_eq!(
            short(r"C:\\Users\\Alice\\Documents"),
            r"%USERPROFILE%\\Documents"
        );
        assert_eq!(
            short(r"loaded from D:\Steam\steamapps\common\Skyrim\Data\x.esp"),
            r"loaded from <game>\Data\x.esp"
        );
        assert_eq!(short(r"C:\Users\Alice"), "%USERPROFILE%");
    }

    #[test]
    fn a_longer_folder_of_another_name_is_not_a_known_one() {
        // `Alice2` is someone else, but still a user's folder.
        assert_eq!(short(r"C:\Users\Alice2\x"), r"C:\Users\<user>\x");
        assert_eq!(
            short(r"D:\Steam\steamapps\common\Skyrim Special Edition\Data"),
            r"D:\Steam\steamapps\common\Skyrim Special Edition\Data"
        );
        // A space that ends the path ends the folder.
        assert_eq!(
            short(r"D:\Steam\steamapps\common\Skyrim is missing, see C:\x"),
            r"<game> is missing, see C:\x"
        );
    }

    #[test]
    fn names_are_replaced_where_they_are_folders() {
        assert_eq!(short(r"E:\Mods\Alice\Vortex"), r"E:\Mods\<user>\Vortex");
        assert_eq!(short(r"\\ALICE-PC\share\x"), r"\\<machine>\share\x");
        // Not a folder: a mod's name keeps the name.
        assert_eq!(short("Alice's Armour"), "Alice's Armour");
        assert_eq!(short(r"E:\Mods\Alicens\x"), r"E:\Mods\Alicens\x");
    }

    #[test]
    fn other_users_and_profiles_are_replaced() {
        assert_eq!(
            short(r"C:\Users\BOB~1\AppData\Local\Temp\x"),
            r"C:\Users\<user>\AppData\Local\Temp\x"
        );
        assert_eq!(short("/home/bob/.config"), "/home/<user>/.config");
        assert_eq!(
            short(r"C:\Users\Public\Documents"),
            r"C:\Users\Public\Documents"
        );
        assert_eq!(
            short(r"C:\Users\Alice\AppData\Roaming\Vortex\skyrimse\profiles\a1B2c3\x"),
            r"%USERPROFILE%\AppData\Roaming\Vortex\skyrimse\profiles\<profile>\x"
        );
        let instance = Shortener::new().with_root(r"E:\MO2\Skyrim", Placeholder::Instance);
        assert_eq!(
            instance.shorten(
                r"E:\MO2\Skyrim\profiles\Alice run\plugins.txt",
                &mut BTreeSet::new()
            ),
            r"<instance>\profiles\<profile>\plugins.txt"
        );
    }

    #[test]
    fn paths_inside_the_game_stay() {
        assert_eq!(
            short("data/skse/plugins/old.dll"),
            "data/skse/plugins/old.dll"
        );
        assert_eq!(
            short(r"meshes\home\profiles\x.nif"),
            r"meshes\home\profiles\x.nif"
        );
        assert_eq!(short(r"Users\x"), r"Users\x");
    }

    #[test]
    fn the_used_placeholders_are_told() {
        let mut used = BTreeSet::new();
        alice().shorten(r"C:\Users\Alice\x and \\ALICE-PC\y", &mut used);
        assert_eq!(
            used.into_iter().collect::<Vec<_>>(),
            [Placeholder::UserProfile, Placeholder::Machine]
        );
    }

    #[test]
    fn a_root_of_one_part_is_ignored() {
        let shortener = Shortener::new()
            .with_root(r"C:\", Placeholder::Game)
            .with_root("/home", Placeholder::UserProfile)
            .with_name("", Placeholder::User);
        assert_eq!(shortener, Shortener::new());
    }

    proptest! {
        #[test]
        fn shortening_never_panics(text in "\\PC*", root in "\\PC*", name in "\\PC*") {
            let shortener = Shortener::new()
                .with_root(&root, Placeholder::Game)
                .with_name(&name, Placeholder::User);
            let _ = shortener.shorten(&text, &mut BTreeSet::new());
        }

        #[test]
        fn shortening_paths_never_panics(text in r"([A-Za-z]:|[\\/]{1,2}|[A-Za-z ~.]{0,6}|\x22){0,12}") {
            let _ = short(&text);
        }

        #[test]
        fn no_home_path_survives(rest in r"([\\/][b-zB-Z0-9 ._-]{1,8}){0,4}", upper in any::<bool>(), slash in any::<bool>()) {
            let mut home = String::from(r"C:\Users\Alice");
            if upper { home = home.to_uppercase(); }
            if slash { home = home.replace('\\', "/"); }
            let text = format!("see {home}{rest}");
            let shortened = short(&text);
            prop_assert!(!shortened.to_lowercase().contains("alice"), "{shortened}");
        }
    }
}
