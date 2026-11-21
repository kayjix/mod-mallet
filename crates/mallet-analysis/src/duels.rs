//! Gathers the findings about two mods into duels, and suggests a side.
//!
//! Two rules decide the suggestion, in this order:
//!
//! 1. **Built on.** If a plugin of one side names a plugin of the other as
//!    its master, it was made to change that mod: it wins.
//! 2. **Specialist.** For the topic the pair disagrees about most, compare
//!    how much of each side's disagreements, with any mod, are about that
//!    topic. A mod whose disagreements are mostly faces is a face mod; a
//!    mod whose disagreements spread over everything touches faces in
//!    passing. The specialist wins its topic, if the difference is clear.
//!
//! Otherwise there is no suggestion: it is a matter of taste, which the
//! pictures help with.

use std::collections::{BTreeMap, HashMap};

use mallet_core::duel::{
    DuelFile, DuelResolution, DuelSide, DuelSuggestion, DuelTopic, ModDuel, SuggestionReason, Topic,
};
use mallet_core::{Action, Finding, GamePath, ModRef, Party, Subject};

use crate::Context;

/// How much larger one side's share of a topic must be than the other's
/// for it to count as the specialist.
const SPECIALIST_MARGIN: f64 = 0.25;

/// The topic of a finding, if it is about two mods disagreeing.
fn topic_of(finding: &Finding) -> Option<Topic> {
    let kind = finding.kind.as_str();
    match kind {
        "npcs.face-mismatch" => Some(Topic::Faces),
        "meshes.retexture-bypassed" => Some(Topic::Meshes),
        "plugins.contained" => Some(Topic::Contained),
        _ if kind.starts_with("assets.") => Some(Topic::Textures),
        _ if kind.starts_with("objects.") => Some(Topic::Objects),
        _ => None,
    }
}

/// One side as found in a finding: its key and how to show it.
#[derive(Clone)]
struct Found {
    key: String,
    side: DuelSide,
}

fn side_of(party: &Party, context: &Context<'_>) -> Option<Found> {
    let mut found = side_without_plugins(party, context)?;
    if let Party::File { path } = party {
        let name = path.file_name();
        let plugin = std::path::Path::new(name)
            .extension()
            .is_some_and(|extension| {
                ["esp", "esm", "esl"]
                    .iter()
                    .any(|known| extension.eq_ignore_ascii_case(known))
            });
        if plugin {
            found.side.plugins.push(name.to_owned());
        }
    }
    Some(found)
}

fn side_without_plugins(party: &Party, context: &Context<'_>) -> Option<Found> {
    match party {
        Party::File { path } => Some(match context.mods.provider_of(path) {
            Some(mod_ref) => from_mod(mod_ref),
            None => Found {
                key: format!("file:{path}"),
                side: DuelSide {
                    mod_ref: None,
                    name: path.file_name().to_owned(),
                    plugins: Vec::new(),
                },
            },
        }),
        Party::Mod { source, id } => Some(from_mod(ModRef {
            source: source.clone(),
            id: id.clone(),
            name: id.clone(),
        })),
        Party::Unmanaged => None,
    }
}

fn from_mod(mod_ref: ModRef) -> Found {
    Found {
        key: format!("mod:{}:{}", mod_ref.source, mod_ref.id),
        side: DuelSide {
            name: mod_ref.name.clone(),
            mod_ref: Some(mod_ref),
            plugins: Vec::new(),
        },
    }
}

/// The two sides of a finding: the winner first where there is one.
fn sides_of(finding: &Finding, context: &Context<'_>) -> Option<(Found, Found, bool)> {
    match &finding.subject {
        Subject::Override { winner, loser } => {
            Some((side_of(winner, context)?, side_of(loser, context)?, true))
        }
        Subject::Pair { first, second } => {
            Some((side_of(first, context)?, side_of(second, context)?, false))
        }
        _ => None,
    }
}

#[derive(Default)]
struct Gathered {
    sides: [Option<DuelSide>; 2],
    keys: [String; 2],
    topics: BTreeMap<Topic, (u32, Option<u8>)>,
    findings: Vec<mallet_core::FindingId>,
    resolutions: Vec<DuelResolution>,
    files: Vec<DuelFile>,
}

/// The duels among `findings`, most disputed first.
pub fn duels(findings: &[Finding], context: &Context<'_>) -> Vec<ModDuel> {
    let mut gathered: BTreeMap<(String, String), Gathered> = BTreeMap::new();
    // How many things each side disagrees about, per topic, with any mod.
    let mut footprint: HashMap<String, BTreeMap<Topic, u32>> = HashMap::new();
    let objects = objects_by_model(context);

    for finding in findings {
        let Some(topic) = topic_of(finding) else {
            continue;
        };
        let Some((a, b, a_wins)) = sides_of(finding, context) else {
            continue;
        };
        if a.key == b.key {
            continue;
        }
        let parts = parts_of(finding, topic);
        for found in [&a, &b] {
            let mine = footprint.entry(found.key.clone()).or_default();
            for (part, count) in &parts {
                *mine.entry(*part).or_default() += count;
            }
        }

        // Sides in a fixed order, by key, so that the duel's identity does
        // not depend on who wins now.
        let (first, second, winner) = if a.key <= b.key {
            (a, b, a_wins.then_some(0u8))
        } else {
            (b, a, a_wins.then_some(1u8))
        };
        let entry = gathered
            .entry((first.key.clone(), second.key.clone()))
            .or_default();
        entry.keys = [first.key.clone(), second.key.clone()];
        for (slot, found) in entry.sides.iter_mut().zip([&first, &second]) {
            let side = slot.get_or_insert_with(|| found.side.clone());
            for plugin in &found.side.plugins {
                if !side.plugins.contains(plugin) {
                    side.plugins.push(plugin.clone());
                }
            }
        }
        for (part, count) in &parts {
            let topic_entry = entry.topics.entry(*part).or_insert((0, winner));
            topic_entry.0 += count;
        }
        entry.findings.push(finding.id.clone());
        add_files(&mut entry.files, finding, topic, &objects);

        // Actions that hand the win to one side.
        let resolutions = resolutions_of(finding, &entry.keys[0], context);
        entry.resolutions.extend(resolutions);
    }

    let built_on = masters_by_side(context);
    let mut duels: Vec<ModDuel> = gathered
        .into_iter()
        .filter_map(|((first_key, second_key), gathered)| {
            let [Some(first), Some(second)] = gathered.sides else {
                return None;
            };
            let mut topics: Vec<DuelTopic> = gathered
                .topics
                .iter()
                .map(|(topic, (count, winning))| DuelTopic {
                    topic: *topic,
                    count: *count,
                    winning: *winning,
                })
                .collect();
            topics.sort_by(|a, b| b.count.cmp(&a.count).then(a.topic.cmp(&b.topic)));
            let suggestion = suggest(
                [&first_key, &second_key],
                topics.first().map(|topic| topic.topic),
                &footprint,
                &built_on,
            );
            Some(ModDuel {
                id: format!("duel:{first_key}|{second_key}"),
                sides: [first, second],
                topics,
                findings: gathered.findings,
                files: gathered.files,
                suggestion,
                resolutions: gathered.resolutions,
            })
        })
        .collect();
    duels.sort_by(|a, b| {
        let total = |duel: &ModDuel| duel.topics.iter().map(|topic| topic.count).sum::<u32>();
        total(b).cmp(&total(a)).then(a.id.cmp(&b.id))
    });
    duels
}

/// The actions of `finding` that make one side win, with that side: 0 if
/// it is the side with the key `first_key`.
fn resolutions_of(
    finding: &Finding,
    first_key: &str,
    context: &Context<'_>,
) -> Vec<DuelResolution> {
    finding
        .actions
        .iter()
        .filter_map(|action| {
            let Action::LoadAfter { plugin, .. } = action else {
                return None;
            };
            let owner = context
                .mods
                .provider_of(&GamePath::new(&format!("data/{plugin}")));
            let key = owner.map_or_else(
                || format!("file:data/{}", plugin.to_lowercase()),
                |mod_ref| format!("mod:{}:{}", mod_ref.source, mod_ref.id),
            );
            Some(DuelResolution {
                side: u8::from(key != first_key),
                finding: finding.id.clone(),
                action: action.clone(),
            })
        })
        .collect()
}

/// Which object uses which model, to name the files of a duel.
fn objects_by_model<'a>(context: &'a Context<'_>) -> HashMap<&'a str, (String, &'static str)> {
    context
        .facts
        .objects
        .bases
        .iter()
        .filter_map(|base| {
            let name = base.name.clone().or_else(|| base.editor_id.clone())?;
            Some((base.model.as_deref()?, (name, kind_label(base.kind))))
        })
        .collect()
}

/// How many things of each topic `finding`, about `topic`, is about. A
/// finding about the files of archives counts its meshes, textures and
/// other files apart, so that a mesh is never counted as a texture.
fn parts_of(finding: &Finding, topic: Topic) -> Vec<(Topic, u32)> {
    let number = |name: &str| {
        finding.args.get(name).and_then(|value| match value {
            mallet_core::ArgValue::Number(number) => u32::try_from(*number).ok(),
            mallet_core::ArgValue::Text(_) => None,
        })
    };
    let count = number("count").unwrap_or(1).max(1);
    if !finding.kind.as_str().starts_with("assets.") {
        return vec![(topic, count)];
    }
    // Findings kept from before the files were counted by kind.
    let (Some(meshes), Some(textures)) = (number("meshes"), number("textures")) else {
        return vec![(topic, count)];
    };
    let other = count.saturating_sub(meshes).saturating_sub(textures);
    let parts: Vec<(Topic, u32)> = [
        (Topic::Meshes, meshes),
        (Topic::Textures, textures),
        (Topic::Files, other),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .collect();
    if parts.is_empty() {
        vec![(Topic::Files, count)]
    } else {
        parts
    }
}

/// The topic of one file of a finding about `topic`: for the files of
/// archives, by the file's kind.
fn file_topic(path: &str, topic: Topic) -> Topic {
    if topic != Topic::Textures {
        return topic;
    }
    let extension = std::path::Path::new(path)
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase());
    match extension.as_deref() {
        Some("nif") => Topic::Meshes,
        Some("dds") => Topic::Textures,
        _ => Topic::Files,
    }
}

/// Adds the files `finding` names to `files`, each once.
fn add_files(
    files: &mut Vec<DuelFile>,
    finding: &Finding,
    topic: Topic,
    objects: &HashMap<&str, (String, &'static str)>,
) {
    for path in &finding.previews {
        let path = path.as_str().trim_start_matches("data/").to_owned();
        if files.iter().any(|file| file.path == path) {
            continue;
        }
        let object = objects.get(path.as_str());
        files.push(DuelFile {
            topic: file_topic(&path, topic),
            object: object.map(|(name, _)| name.clone()),
            object_kind: object.map(|(_, kind)| (*kind).to_owned()),
            path,
        });
    }
}

/// A short fixed word for a kind of object.
fn kind_label(kind: mallet_core::facts::objects::BaseKind) -> &'static str {
    use mallet_core::facts::objects::BaseKind;
    match kind {
        BaseKind::Static => "static",
        BaseKind::MovableStatic => "movable-static",
        BaseKind::StaticCollection => "static-collection",
        BaseKind::Tree => "tree",
        BaseKind::Furniture => "furniture",
        BaseKind::Container => "container",
        BaseKind::Door => "door",
        BaseKind::Activator => "activator",
    }
}

/// For each side, the sides whose plugins its plugins name as masters.
fn masters_by_side(context: &Context<'_>) -> HashMap<String, Vec<String>> {
    let key_of = |plugin: &str| {
        let path = GamePath::new(&format!("data/{plugin}"));
        context.mods.provider_of(&path).map_or_else(
            || format!("file:{path}"),
            |mod_ref| format!("mod:{}:{}", mod_ref.source, mod_ref.id),
        )
    };
    let mut built_on: HashMap<String, Vec<String>> = HashMap::new();
    for plugin in &context.facts.plugins {
        let own = key_of(&plugin.name);
        for master in &plugin.masters {
            let other = key_of(master);
            if other != own {
                built_on.entry(own.clone()).or_default().push(other);
            }
        }
    }
    built_on
}

fn suggest(
    keys: [&String; 2],
    main_topic: Option<Topic>,
    footprint: &HashMap<String, BTreeMap<Topic, u32>>,
    built_on: &HashMap<String, Vec<String>>,
) -> Option<DuelSuggestion> {
    let [first, second] = keys;
    let builds_on =
        |a: &String, b: &String| built_on.get(a).is_some_and(|masters| masters.contains(b));
    match (builds_on(first, second), builds_on(second, first)) {
        (true, false) => {
            return Some(DuelSuggestion {
                side: 0,
                reason: SuggestionReason::BuiltOn,
                topic: None,
            });
        }
        (false, true) => {
            return Some(DuelSuggestion {
                side: 1,
                reason: SuggestionReason::BuiltOn,
                topic: None,
            });
        }
        _ => {}
    }
    let topic = main_topic?;
    let share = |key: &String| {
        let topics = footprint.get(key)?;
        let total: u32 = topics.values().sum();
        let here = topics.get(&topic).copied().unwrap_or(0);
        (total > 0).then(|| f64::from(here) / f64::from(total))
    };
    let (a, b) = (share(first)?, share(second)?);
    if (a - b).abs() < SPECIALIST_MARGIN {
        return None;
    }
    Some(DuelSuggestion {
        side: u8::from(b > a),
        reason: SuggestionReason::Specialist,
        topic: Some(topic),
    })
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::{Facts, GamePath, ModSource, kinds};

    use super::*;

    /// Every plugin is its own mod, named like the plugin without extension.
    struct ByName;

    impl ModSource for ByName {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            let name = path.file_name();
            let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
            Some(ModRef {
                source: "test".to_owned(),
                id: stem.to_owned(),
                name: stem.to_owned(),
            })
        }
    }

    fn file(name: &str) -> Party {
        Party::File {
            path: GamePath::new(&format!("data/{name}")),
        }
    }

    fn faces(winner: &str, face: &str, count: i64) -> Finding {
        Finding::new(
            &kinds::NPCS_FACE_MISMATCH,
            Subject::Override {
                winner: file(winner),
                loser: file(face),
            },
        )
        .with_arg("count", count)
        .with_action(Action::LoadAfter {
            plugin: face.to_owned(),
            after: winner.to_owned(),
        })
    }

    fn overlap(first: &str, second: &str, count: i64) -> Finding {
        Finding::new(
            &kinds::OBJECTS_OVERLAPPING,
            Subject::Pair {
                first: file(first),
                second: file(second),
            },
        )
        .with_arg("count", count)
    }

    fn plugin(name: &str, masters: &[&str]) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new(&format!("data/{name}")),
            masters: masters.iter().map(|master| (*master).to_owned()).collect(),
            npcs: Vec::new(),
        }
    }

    #[test]
    fn the_findings_of_two_mods_become_one_duel() {
        let facts = Facts::default();
        let context = Context::new(&facts, &ByName);
        let found = duels(
            &[
                faces("Content.esp", "Faces.esp", 34),
                overlap("Content.esp", "Faces.esp", 2),
                overlap("Town.esp", "Trees.esp", 1),
            ],
            &context,
        );
        assert_eq!(found.len(), 2);
        let duel = &found[0];
        assert_eq!(duel.sides[0].name, "content");
        assert_eq!(duel.sides[1].name, "faces");
        assert_eq!(duel.findings.len(), 2);
        assert_eq!(duel.sides[1].plugins, ["faces.esp"]);
        assert_eq!(duel.topics[0].topic, Topic::Faces);
        assert_eq!(duel.topics[0].count, 34);
        // Content's plugin wins the faces now.
        assert_eq!(duel.topics[0].winning, Some(0));
        assert_eq!(duel.topics[1].winning, None);
        // Loading the face plugin last makes the face mod win.
        assert_eq!(duel.resolutions.len(), 1);
        assert_eq!(duel.resolutions[0].side, 1);
    }

    #[test]
    fn the_specialist_wins_its_topic() {
        let facts = Facts::default();
        let context = Context::new(&facts, &ByName);
        // Faces.esp disagrees about nothing but faces; Content.esp about
        // objects with others as well.
        let found = duels(
            &[
                faces("Content.esp", "Faces.esp", 10),
                overlap("Content.esp", "Town.esp", 40),
            ],
            &context,
        );
        let duel = found
            .iter()
            .find(|duel| duel.sides[1].name == "faces")
            .unwrap();
        let suggestion = duel.suggestion.as_ref().unwrap();
        assert_eq!(suggestion.side, 1);
        assert_eq!(suggestion.reason, SuggestionReason::Specialist);
        assert_eq!(suggestion.topic, Some(Topic::Faces));
    }

    #[test]
    fn a_plugin_built_on_the_other_wins() {
        let facts = Facts {
            plugins: vec![plugin("Base.esp", &[]), plugin("Patch.esp", &["Base.esp"])],
            ..Facts::default()
        };
        let context = Context::new(&facts, &ByName);
        let found = duels(&[overlap("Base.esp", "Patch.esp", 3)], &context);
        let suggestion = found[0].suggestion.as_ref().unwrap();
        assert_eq!(found[0].sides[usize::from(suggestion.side)].name, "patch");
        assert_eq!(suggestion.reason, SuggestionReason::BuiltOn);
    }

    #[test]
    fn files_of_archives_are_counted_by_their_kind() {
        let facts = Facts::default();
        let context = Context::new(&facts, &ByName);
        // Two skeletons and a script: meshes and another file, no texture.
        let finding = Finding::new(
            &kinds::ASSETS_ARCHIVE_OVER_ARCHIVE,
            Subject::Override {
                winner: file("Skeleton.bsa"),
                loser: file("Patch.bsa"),
            },
        )
        .with_arg("count", 3)
        .with_arg("meshes", 2)
        .with_arg("textures", 0)
        .with_previews(vec![
            GamePath::new("data/meshes/actors/werewolfbeast/character assets/skeleton.nif"),
            GamePath::new("data/meshes/actors/vampirelord/character assets/skeleton.nif"),
            GamePath::new("data/scripts/werewolf.pex"),
        ]);
        let found = duels(&[finding], &context);
        let topics: Vec<(Topic, u32)> = found[0]
            .topics
            .iter()
            .map(|topic| (topic.topic, topic.count))
            .collect();
        assert_eq!(topics, [(Topic::Meshes, 2), (Topic::Files, 1)]);
        let files: Vec<(&str, Topic)> = found[0]
            .files
            .iter()
            .map(|file| (file.path.as_str(), file.topic))
            .collect();
        assert_eq!(
            files,
            [
                (
                    "meshes/actors/werewolfbeast/character assets/skeleton.nif",
                    Topic::Meshes
                ),
                (
                    "meshes/actors/vampirelord/character assets/skeleton.nif",
                    Topic::Meshes
                ),
                ("scripts/werewolf.pex", Topic::Files),
            ]
        );
    }

    #[test]
    fn findings_about_one_mod_are_no_duel() {
        let facts = Facts::default();
        let context = Context::new(&facts, &ByName);
        let single = Finding::new(
            &kinds::EXTENDER_PLUGIN_INCOMPATIBLE,
            Subject::File {
                path: GamePath::new("data/skse/plugins/a.dll"),
            },
        );
        assert_eq!(duels(&[single], &context), Vec::new());
    }
}
