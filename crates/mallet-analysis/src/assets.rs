//! Which file the game actually uses when several sources provide it.
//!
//! The game looks for an asset first among the loose files and then in the
//! archives, the one loaded last first. A mod manager shows conflicts
//! between loose files. It cannot show the two cases reported here, because
//! one side is hidden inside an archive:
//!
//! * loose files that replace files in another mod's archive, and
//! * an archive that replaces files in another mod's archive.
//!
//! Either can be exactly what the user wants, for example a texture pack
//! over an older mod. It can also break a mod quietly, when only some of
//! the files that belong together are replaced. Mod Mallet cannot tell
//! which, so these are notes, not warnings.

use std::collections::{BTreeMap, HashMap, HashSet};

use mallet_core::facts::ArchiveFacts;
use mallet_core::{Facts, Finding, GamePath, ModRef, Party, Subject, kinds};

use crate::{Analysis, Context, examples};

/// How many file names a finding quotes.
const EXAMPLE_LIMIT: usize = 3;

/// Where the game takes an asset from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetSource {
    /// A loose file in the data directory.
    Loose,
    /// The archive at this position in [`Facts::archives`].
    Archive(usize),
}

/// For every asset, the source the game uses.
pub struct AssetIndex<'a> {
    loose: HashSet<&'a GamePath>,
    winning_archive: HashMap<&'a GamePath, usize>,
}

impl<'a> AssetIndex<'a> {
    /// Builds the index from the loose files and archives of `facts`.
    pub fn new(facts: &'a Facts) -> Self {
        let mut winning_archive = HashMap::new();
        // Archives are ordered from lowest to highest precedence, so a later
        // archive simply replaces the entry of an earlier one.
        for (index, archive) in facts.archives.iter().enumerate() {
            for file in &archive.files {
                winning_archive.insert(file, index);
            }
        }
        Self {
            loose: facts.loose_files.iter().collect(),
            winning_archive,
        }
    }

    /// Whether `path` exists as a loose file.
    pub fn is_loose(&self, path: &GamePath) -> bool {
        self.loose.contains(path)
    }

    /// The archive that would provide `path` if there were no loose file.
    pub fn winning_archive(&self, path: &GamePath) -> Option<usize> {
        self.winning_archive.get(path).copied()
    }

    /// Where the game takes `path` from, or `None` if nothing provides it.
    pub fn source(&self, path: &GamePath) -> Option<AssetSource> {
        if self.is_loose(path) {
            Some(AssetSource::Loose)
        } else {
            self.winning_archive(path).map(AssetSource::Archive)
        }
    }
}

/// Reports files in a mod's archive that the game takes from somewhere
/// else.
///
/// Only archives that a mod manager attributes to a mod are examined:
/// replacing the game's own files is what mods are for.
pub struct ArchiveOverrides;

/// How many textures of a finding are offered for looking at.
const PREVIEW_LIMIT: usize = 40;

/// How many files of one archive one other source replaces.
#[derive(Default)]
struct Tally<'a> {
    count: i64,
    files: Vec<&'a GamePath>,
}

impl<'a> Tally<'a> {
    fn add(&mut self, file: &'a GamePath) {
        self.count += 1;
        self.files.push(file);
    }

    /// The textures and meshes among the files, to look at, up to a limit.
    fn textures(&self) -> Vec<GamePath> {
        self.files
            .iter()
            .filter(|file| {
                std::path::Path::new(file.as_str())
                    .extension()
                    .is_some_and(|extension| {
                        extension.eq_ignore_ascii_case("dds")
                            || extension.eq_ignore_ascii_case("nif")
                    })
            })
            .take(PREVIEW_LIMIT)
            .map(|file| (*file).clone())
            .collect()
    }

    /// A few of the files, as paths inside the data directory.
    fn examples(&self) -> String {
        examples(
            self.files.iter().map(|file| {
                let path = file.as_str();
                path.split_once('/').map_or(path, |(_, inside)| inside)
            }),
            EXAMPLE_LIMIT,
        )
    }
}

impl Analysis for ArchiveOverrides {
    fn id(&self) -> &'static str {
        "archive-overrides"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let archives = &context.facts.archives;
        let owners: Vec<Option<ModRef>> = archives
            .iter()
            .map(|archive| context.mods.provider_of(&archive.path))
            .collect();

        let mut findings = Vec::new();
        for (index, (archive, owner)) in archives.iter().zip(&owners).enumerate() {
            let Some(owner) = owner else {
                continue;
            };

            let mut by_loose: BTreeMap<Option<ModRef>, Tally<'_>> = BTreeMap::new();
            let mut by_archive: BTreeMap<usize, Tally<'_>> = BTreeMap::new();
            for file in &archive.files {
                if context.assets.is_loose(file) {
                    let provider = context.mods.provider_of(file);
                    if provider.as_ref() != Some(owner) {
                        by_loose.entry(provider).or_default().add(file);
                    }
                } else if let Some(winner) = context.assets.winning_archive(file)
                    && winner != index
                    && owners.get(winner).and_then(Option::as_ref) != Some(owner)
                {
                    by_archive.entry(winner).or_default().add(file);
                }
            }

            for (provider, tally) in by_loose {
                findings.push(loose_over_archive(
                    archive,
                    owner,
                    provider.as_ref(),
                    &tally,
                ));
            }
            for (winner, tally) in by_archive {
                if let Some(winning) = archives.get(winner) {
                    let winner_owner = owners.get(winner).and_then(Option::as_ref);
                    findings.push(archive_over_archive(
                        archive,
                        owner,
                        winning,
                        winner_owner,
                        &tally,
                    ));
                }
            }
        }
        findings
    }
}

/// `mod` or `unmanaged`, for choosing the wording of a message.
fn origin(provider: Option<&ModRef>) -> &'static str {
    if provider.is_some() {
        "mod"
    } else {
        "unmanaged"
    }
}

fn name(provider: Option<&ModRef>) -> &str {
    provider.map_or("", |provider| provider.name.as_str())
}

fn loose_over_archive(
    archive: &ArchiveFacts,
    owner: &ModRef,
    provider: Option<&ModRef>,
    tally: &Tally<'_>,
) -> Finding {
    Finding::new(
        &kinds::ASSETS_LOOSE_OVER_ARCHIVE,
        Subject::Override {
            winner: Party::from_provider(provider),
            loser: Party::File {
                path: archive.path.clone(),
            },
        },
    )
    .with_arg("count", tally.count)
    .with_arg("examples", tally.examples())
    .with_arg("winnerOrigin", origin(provider))
    .with_arg("winnerMod", name(provider))
    .with_arg("archive", archive.path.file_name())
    .with_arg("archiveMod", owner.name.as_str())
    .with_previews(tally.textures())
}

fn archive_over_archive(
    archive: &ArchiveFacts,
    owner: &ModRef,
    winning: &ArchiveFacts,
    winner_owner: Option<&ModRef>,
    tally: &Tally<'_>,
) -> Finding {
    Finding::new(
        &kinds::ASSETS_ARCHIVE_OVER_ARCHIVE,
        Subject::Override {
            winner: Party::File {
                path: winning.path.clone(),
            },
            loser: Party::File {
                path: archive.path.clone(),
            },
        },
    )
    .with_arg("count", tally.count)
    .with_arg("examples", tally.examples())
    .with_arg("winnerArchive", winning.path.file_name())
    .with_arg("winnerOrigin", origin(winner_owner))
    .with_arg("winnerMod", name(winner_owner))
    .with_arg("archive", archive.path.file_name())
    .with_arg("archiveMod", owner.name.as_str())
    .with_previews(tally.textures())
}

#[cfg(test)]
pub(crate) mod testing {
    //! A mod manager made of a list, for the tests of several analyses.

    use mallet_core::{GamePath, ModRef, ModSource};

    /// Attributes each listed path to the mod named next to it.
    pub(crate) struct Mods(pub(crate) Vec<(&'static str, &'static str)>);

    impl ModSource for Mods {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            self.0
                .iter()
                .find(|(owned, _)| GamePath::new(owned) == *path)
                .map(|(_, name)| mod_ref(name))
        }
    }

    pub(crate) fn mod_ref(name: &str) -> ModRef {
        ModRef {
            source: "test".to_owned(),
            id: name.to_owned(),
            name: name.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::{ArgValue, NoModSource, Severity};

    use super::testing::Mods;
    use super::*;
    use crate::run_all;

    fn archive(path: &str, files: &[&str]) -> ArchiveFacts {
        ArchiveFacts {
            path: GamePath::new(path),
            files: files.iter().map(|file| GamePath::new(file)).collect(),
        }
    }

    fn paths(files: &[&str]) -> Vec<GamePath> {
        files.iter().map(|file| GamePath::new(file)).collect()
    }

    /// A grass mod packed in an archive, and a flora mod whose loose files
    /// replace two of its three files.
    fn grass_setup() -> (Facts, Mods) {
        let facts = Facts {
            loose_files: paths(&[
                "data/meshes/grass/fern.nif",
                "data/textures/grass/fern.dds",
                "data/textures/sky.dds",
            ]),
            archives: vec![
                archive("data/skyrim - meshes0.bsa", &["data/meshes/grass/fern.nif"]),
                archive(
                    "data/grass.bsa",
                    &[
                        "data/meshes/grass/fern.nif",
                        "data/textures/grass/fern.dds",
                        "data/textures/grass/moss.dds",
                    ],
                ),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            ("data/grass.bsa", "Grass Mod"),
            ("data/meshes/grass/fern.nif", "Flora Mod"),
            ("data/textures/grass/fern.dds", "Flora Mod"),
            ("data/textures/sky.dds", "Sky Mod"),
        ]);
        (facts, mods)
    }

    #[test]
    fn the_index_prefers_loose_files_then_the_last_archive() {
        let (facts, _) = grass_setup();
        let index = AssetIndex::new(&facts);
        assert_eq!(
            index.source(&GamePath::new("data/meshes/grass/fern.nif")),
            Some(AssetSource::Loose)
        );
        assert_eq!(
            index.source(&GamePath::new("data/textures/grass/moss.dds")),
            Some(AssetSource::Archive(1))
        );
        assert_eq!(
            index.winning_archive(&GamePath::new("data/meshes/grass/fern.nif")),
            Some(1)
        );
        assert_eq!(index.source(&GamePath::new("data/nothing.dds")), None);
    }

    #[test]
    fn loose_files_over_another_mods_archive_are_reported_once_per_mod() {
        let (facts, mods) = grass_setup();
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "assets.loose-over-archive");
        assert_eq!(finding.severity, Severity::Info);
        assert_eq!(
            finding.id.as_str(),
            "assets.loose-over-archive:override:mod:test:Flora Mod>file:data/grass.bsa"
        );
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("meshes/grass/fern.nif, textures/grass/fern.dds")
        );
        assert_eq!(finding.args["winnerOrigin"], ArgValue::from("mod"));
        assert_eq!(finding.args["winnerMod"], ArgValue::from("Flora Mod"));
        assert_eq!(finding.args["archive"], ArgValue::from("grass.bsa"));
        assert_eq!(finding.args["archiveMod"], ArgValue::from("Grass Mod"));
    }

    #[test]
    fn a_mods_own_loose_files_over_its_archive_are_not_reported() {
        let (facts, _) = grass_setup();
        let mods = Mods(vec![
            ("data/grass.bsa", "Grass Mod"),
            ("data/meshes/grass/fern.nif", "Grass Mod"),
            ("data/textures/grass/fern.dds", "Grass Mod"),
        ]);
        assert_eq!(run_all(&Context::new(&facts, &mods)), Vec::<Finding>::new());
    }

    #[test]
    fn loose_files_nobody_manages_are_reported_as_unmanaged() {
        let (facts, _) = grass_setup();
        let mods = Mods(vec![("data/grass.bsa", "Grass Mod")]);
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.args["winnerOrigin"], ArgValue::from("unmanaged"));
        assert_eq!(finding.args["winnerMod"], ArgValue::from(""));
        assert_eq!(
            finding.id.as_str(),
            "assets.loose-over-archive:override:unmanaged>file:data/grass.bsa"
        );
    }

    #[test]
    fn the_games_own_archives_being_replaced_is_not_reported() {
        let (facts, _) = grass_setup();
        // Nothing is attributed to a mod: every archive is the game's own.
        assert_eq!(
            run_all(&Context::new(&facts, &NoModSource)),
            Vec::<Finding>::new()
        );
    }

    #[test]
    fn a_later_archive_over_another_mods_archive_is_reported() {
        let facts = Facts {
            archives: vec![
                archive(
                    "data/province.bsa",
                    &[
                        "data/meshes/shared/wall.nif",
                        "data/meshes/province/gate.nif",
                    ],
                ),
                archive("data/older.bsa", &["data/meshes/shared/wall.nif"]),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            ("data/province.bsa", "Province"),
            ("data/older.bsa", "Older Mod"),
        ]);
        let findings = run_all(&Context::new(&facts, &mods));

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "assets.archive-over-archive");
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["winnerArchive"], ArgValue::from("older.bsa"));
        assert_eq!(finding.args["winnerMod"], ArgValue::from("Older Mod"));
        assert_eq!(finding.args["archive"], ArgValue::from("province.bsa"));
        assert_eq!(finding.args["archiveMod"], ArgValue::from("Province"));
    }

    #[test]
    fn two_archives_of_one_mod_do_not_conflict() {
        let facts = Facts {
            archives: vec![
                archive("data/mod.bsa", &["data/textures/a.dds"]),
                archive("data/mod - textures.bsa", &["data/textures/a.dds"]),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            ("data/mod.bsa", "One Mod"),
            ("data/mod - textures.bsa", "One Mod"),
        ]);
        assert_eq!(run_all(&Context::new(&facts, &mods)), Vec::<Finding>::new());
    }

    #[test]
    fn a_file_replaced_by_a_loose_file_is_not_also_counted_against_an_archive() {
        let facts = Facts {
            loose_files: paths(&["data/textures/a.dds"]),
            archives: vec![
                archive("data/first.bsa", &["data/textures/a.dds"]),
                archive("data/second.bsa", &["data/textures/a.dds"]),
            ],
            ..Facts::default()
        };
        let mods = Mods(vec![
            ("data/first.bsa", "First"),
            ("data/second.bsa", "Second"),
            ("data/textures/a.dds", "Loose"),
        ]);
        let kinds: Vec<_> = run_all(&Context::new(&facts, &mods))
            .into_iter()
            .map(|finding| finding.kind)
            .collect();
        assert_eq!(
            kinds,
            ["assets.loose-over-archive", "assets.loose-over-archive"]
        );
    }
}
