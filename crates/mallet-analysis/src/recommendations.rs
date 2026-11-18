//! Suggestions about whole mods (ADR 0043).
//!
//! The other analyses report what goes wrong between two files or two
//! plugins. These look at a mod as a whole and suggest, never require, a
//! decision about it:
//!
//! * a mod whose placed objects collide with many spots of other mods, a
//!   candidate to remove or to let lose against those mods everywhere;
//! * a mod most of whose content does not reach the game, because mods
//!   loaded later replace it, a candidate to remove;
//! * a mod that places largely the same objects as another, at the same
//!   places: keep one of the two.
//!
//! Only mods a mod manager knows are suggested: the game's own files and
//! files nobody claims are never candidates.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::objects::Place;
use mallet_core::{Finding, ModRef, ObjectRef, Subject, kinds};

use crate::{Analysis, AssetSource, Context, offer_disabling};

/// How many other mods a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// A mod collides widely from this many collision spots on, counting only
/// those with mods that place more objects than it does: about one town's
/// worth, more than a patch would usually leave.
const MIN_SPOTS: usize = 20;

/// A mod is mostly overridden only if Mod Mallet compared at least this
/// many of its parts: below that, a few overrides say little.
const MIN_PARTS: usize = 20;

/// ... and at most this share of them, in percent, reaches the game.
const MAX_SHARE_SURVIVING: usize = 10;

/// Suggests mods whose objects collide with many spots of other mods.
///
/// Of two mods that collide, the one that places fewer objects is the
/// candidate, as for which copy of a duplicate to hide: it is the one whose
/// own content the collisions are about, while the other is a larger mod
/// that merely shares a few places with it.
pub struct WideCollisions;

impl Analysis for WideCollisions {
    fn id(&self) -> &'static str {
        "mods-collide-widely"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mods = PluginMods::new(context);
        let mut placed: HashMap<Owner, usize> = HashMap::new();
        for object in &context.facts.objects.placed {
            *placed.entry(mods.owner(object.owner())).or_default() += 1;
        }
        let placed_by = |owner: &Owner| placed.get(owner).copied().unwrap_or_default();

        // Two mods one of which duplicates the other are suggested as such.
        let duplicates: HashSet<(Owner, Owner)> = twin_pairs(context, &mods)
            .into_iter()
            .flat_map(|pair| {
                let owner = Owner::Mod(pair.owner);
                [(owner.clone(), pair.other.clone()), (pair.other, owner)]
            })
            .collect();
        let mut tallies: BTreeMap<ModRef, Tally> = BTreeMap::new();
        for spot in &context.collisions().spots {
            let [first, second] = spot.parts.as_slice() else {
                continue;
            };
            let first_owner = mods.owner(&first.object.plugin);
            let second_owner = mods.owner(&second.object.plugin);
            if first_owner == second_owner
                || duplicates.contains(&(first_owner.clone(), second_owner.clone()))
            {
                continue;
            }
            for (own, own_owner, other_owner) in [
                (first, &first_owner, &second_owner),
                (second, &second_owner, &first_owner),
            ] {
                let Owner::Mod(own_mod) = own_owner else {
                    continue;
                };
                if placed_by(own_owner) > placed_by(other_owner) {
                    continue;
                }
                let tally = tallies.entry(own_mod.clone()).or_default();
                tally.spots += 1;
                tally.objects.insert(own.object.clone());
                *tally.others.entry(other_owner.name()).or_default() += 1;
            }
        }

        tallies
            .into_iter()
            .filter(|(_, tally)| tally.spots >= MIN_SPOTS)
            .map(|(owner, tally)| {
                let total = placed_by(&Owner::Mod(owner.clone()));
                let needed = mods.needed_by_others(&owner);
                let finding = Finding::new(&kinds::MODS_COLLIDES_WIDELY, mod_subject(&owner))
                    .with_arg("mod", owner.name.clone())
                    .with_arg("count", number(tally.spots))
                    .with_arg("modCount", number(tally.others.len()))
                    .with_arg("others", ranked(&tally.others))
                    .with_arg("objects", number(tally.objects.len()))
                    .with_arg("total", number(total))
                    .with_provider(Some(owner));
                // Switching off a mod other mods build on would stop the
                // game from starting.
                if needed {
                    finding
                } else {
                    offer_disabling(finding)
                }
            })
            .collect()
    }
}

/// Whose a plugin's content is: a mod's, or, where no mod manager claims
/// the plugin, the plugin's own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Owner {
    Mod(ModRef),
    /// The plugin's file name.
    Plugin(String),
}

impl Owner {
    /// The name to show.
    fn name(&self) -> String {
        match self {
            Self::Mod(owner) => owner.name.clone(),
            Self::Plugin(file) => file.clone(),
        }
    }
}

/// Two objects of the same model this close (in game units, about 45 cm)
/// are the same object placed twice, as for duplicates (ADR 0023).
const TWIN_DISTANCE: i32 = 32;

/// A mod duplicates another from this many of its objects on...
const MIN_TWINS: usize = 20;

/// ... if at least this share of its placed objects, in percent, has a
/// twin in the other mod.
const MIN_SHARE_TWINS: usize = 50;

/// Suggests keeping only one of two mods that place largely the same
/// objects at the same places, such as a collection and a mod it contains.
pub struct DuplicateMods;

impl Analysis for DuplicateMods {
    fn id(&self) -> &'static str {
        "mods-duplicate-another"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mods = PluginMods::new(context);
        twin_pairs(context, &mods)
            .into_iter()
            .map(|pair| {
                let patches = mods.patches_of(&pair.owner, &pair.other);
                let other_patches = match &pair.other {
                    Owner::Mod(other) => mods.patches_of(other, &Owner::Mod(pair.owner.clone())),
                    Owner::Plugin(_) => Vec::new(),
                };
                let names = |plugins: &[String]| {
                    let mut plugins = plugins.to_vec();
                    plugins.truncate(EXAMPLE_LIMIT);
                    plugins.join(", ")
                };
                Finding::new(&kinds::MODS_DUPLICATES_ANOTHER, mod_subject(&pair.owner))
                    .with_arg("mod", pair.owner.name.clone())
                    .with_arg("other", pair.other.name())
                    .with_arg("count", number(pair.twins))
                    .with_arg("total", number(pair.total))
                    .with_arg("patchCount", number(patches.len()))
                    .with_arg("patches", names(&patches))
                    .with_arg("otherPatchCount", number(other_patches.len()))
                    .with_arg("otherPatches", names(&other_patches))
                    .with_provider(Some(pair.owner))
            })
            .collect()
    }
}

/// A mod most of whose objects another mod places as well.
struct TwinPair {
    /// The mod, the one with fewer objects of the two.
    owner: ModRef,
    /// The mod or plugin that places them as well.
    other: Owner,
    /// How many of the mod's objects have a twin in the other.
    twins: usize,
    /// How many objects the mod places.
    total: usize,
}

/// Where an object stands, for finding its twins: place, model and square
/// of a grid of [`TWIN_DISTANCE`].
type GridKey<'a> = (&'a Place, String, [i32; 3]);

/// The pairs of mods one of which places largely the same objects as the
/// other, at the same places: the same model, or the same base object
/// where there is no model, within [`TWIN_DISTANCE`].
fn twin_pairs(context: &Context<'_>, mods: &PluginMods) -> Vec<TwinPair> {
    let objects = &context.facts.objects;
    let models: HashMap<&RecordId, &str> = objects
        .bases
        .iter()
        .filter_map(|base| Some((&base.id, base.model.as_deref()?)))
        .collect();
    let mut owners: Vec<Owner> = Vec::new();
    let mut owner_index: HashMap<Owner, usize> = HashMap::new();
    let mut totals: Vec<usize> = Vec::new();
    // What stands where: by place, model and square of the grid, the
    // owner and position of each object.
    let mut grid: HashMap<GridKey<'_>, Vec<(usize, [i32; 3])>> = HashMap::new();
    let mut entries: Vec<(usize, GridKey<'_>, [i32; 3])> = Vec::new();
    for object in &objects.placed {
        let owner = mods.owner(object.owner());
        let index = *owner_index.entry(owner.clone()).or_insert_with(|| {
            owners.push(owner);
            totals.push(0);
            owners.len() - 1
        });
        if let Some(total) = totals.get_mut(index) {
            *total += 1;
        }
        let asset = models.get(&object.base).map_or_else(
            || format!("{}:{}", object.base.plugin, object.base.local_id),
            |model| model.to_lowercase().replace('/', "\\"),
        );
        let square = object.position.map(|axis| axis.div_euclid(TWIN_DISTANCE));
        let key = (&object.place, asset, square);
        grid.entry(key.clone())
            .or_default()
            .push((index, object.position));
        entries.push((index, key, object.position));
    }

    let mut twins: HashMap<(usize, usize), usize> = HashMap::new();
    for (index, (place, asset, square), position) in &entries {
        let mut found: HashSet<usize> = HashSet::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    let near = [square[0] + dx, square[1] + dy, square[2] + dz];
                    let Some(others) = grid.get(&(*place, asset.clone(), near)) else {
                        continue;
                    };
                    for (other, other_position) in others {
                        if other != index && close(*position, *other_position) {
                            found.insert(*other);
                        }
                    }
                }
            }
        }
        for other in found {
            *twins.entry((*index, other)).or_default() += 1;
        }
    }

    let total_of = |index: usize| totals.get(index).copied().unwrap_or_default();
    let qualifies = |own: usize, other: usize| {
        twins.get(&(own, other)).is_some_and(|&count| {
            count >= MIN_TWINS && count * 100 >= total_of(own) * MIN_SHARE_TWINS
        })
    };
    // Of two mods that duplicate each other, the smaller is named; at
    // equal size, the one first by name.
    let rank = |index: usize| (total_of(index), owners.get(index).map(Owner::name));
    let mut pairs: Vec<TwinPair> = twins
        .iter()
        .filter(|&(&(own, other), _)| {
            qualifies(own, other) && !(qualifies(other, own) && rank(other) < rank(own))
        })
        .filter_map(|(&(own, other), &count)| {
            let Some(Owner::Mod(owner)) = owners.get(own).cloned() else {
                return None;
            };
            Some(TwinPair {
                owner,
                other: owners.get(other).cloned()?,
                twins: count,
                total: total_of(own),
            })
        })
        .collect();
    pairs.sort_by(|a, b| {
        a.owner
            .cmp(&b.owner)
            .then_with(|| a.other.name().cmp(&b.other.name()))
    });
    pairs
}

/// Whether two positions are within [`TWIN_DISTANCE`] of each other.
fn close(a: [i32; 3], b: [i32; 3]) -> bool {
    let square = |axis: usize| {
        let delta = i64::from(a.get(axis).copied().unwrap_or_default())
            - i64::from(b.get(axis).copied().unwrap_or_default());
        delta * delta
    };
    square(0) + square(1) + square(2) <= i64::from(TWIN_DISTANCE) * i64::from(TWIN_DISTANCE)
}

/// What one mod's objects collide with.
#[derive(Default)]
struct Tally {
    /// The collision spots the mod is part of.
    spots: usize,
    /// The mod's own objects among them.
    objects: BTreeSet<ObjectRef>,
    /// How many spots it shares with each other mod, by name.
    others: BTreeMap<String, usize>,
}

/// Suggests mods most of whose content does not reach the game.
pub struct MostlyOverridden;

impl Analysis for MostlyOverridden {
    fn id(&self) -> &'static str {
        "mods-mostly-overridden"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let mods = PluginMods::new(context);
        let mut parts: BTreeMap<ModRef, Parts> = BTreeMap::new();
        let mut count = |owner: Option<ModRef>, winner: Option<String>| {
            if let Some(owner) = owner {
                parts.entry(owner).or_default().add(winner);
            }
        };

        // Objects a mod places, in the version of the plugin that wins.
        for object in &context.facts.objects.placed {
            if object.moved.is_some() {
                continue;
            }
            let placer = object.id.plugin.as_str();
            let winner = object.plugin.to_lowercase();
            let survives = winner == placer || mods.builds_on(&winner, placer);
            count(mods.of(placer), (!survives).then(|| mods.name(&winner)));
        }

        // Characters a plugin defines or changes, in the version of the
        // plugin loaded last.
        let mut last_version: HashMap<&RecordId, usize> = HashMap::new();
        for (index, plugin) in context.facts.plugins.iter().enumerate() {
            for npc in &plugin.npcs {
                last_version.insert(&npc.id, index);
            }
        }
        for (index, plugin) in context.facts.plugins.iter().enumerate() {
            let name = plugin.name.to_lowercase();
            for npc in &plugin.npcs {
                let winner = last_version
                    .get(&npc.id)
                    .filter(|&&winner| winner != index)
                    .and_then(|&winner| context.facts.plugins.get(winner))
                    .map(|winner| winner.name.to_lowercase());
                let lost = winner.filter(|winner| !mods.builds_on(winner, &name));
                count(mods.of(&name), lost.map(|winner| mods.name(&winner)));
            }
        }

        // Files in a mod's archives, where the game takes them from.
        let archive_mods: Vec<Option<ModRef>> = context
            .facts
            .archives
            .iter()
            .map(|archive| context.mods.provider_of(&archive.path))
            .collect();
        for (index, archive) in context.facts.archives.iter().enumerate() {
            let Some(owner) = archive_mods.get(index).cloned().flatten() else {
                continue;
            };
            for file in &archive.files {
                let source_mod = match context.assets.source(file) {
                    Some(AssetSource::Archive(winner)) if winner == index => Some(owner.clone()),
                    Some(AssetSource::Archive(winner)) => {
                        archive_mods.get(winner).cloned().flatten()
                    }
                    Some(AssetSource::Loose) => context.mods.provider_of(file),
                    None => None,
                };
                let lost = (source_mod.as_ref() != Some(&owner))
                    .then(|| source_mod.map_or_else(|| file.as_str().to_owned(), |m| m.name));
                count(Some(owner.clone()), lost);
            }
        }

        // Loose files reach the game: the mod manager put them there.
        for file in &context.facts.loose_files {
            count(context.mods.provider_of(file), None);
        }

        parts
            .into_iter()
            .filter(|(owner, parts)| {
                !mods.needed_by_others(owner)
                    && parts.total >= MIN_PARTS
                    && parts.surviving() * 100 <= parts.total * MAX_SHARE_SURVIVING
            })
            .map(|(owner, parts)| {
                offer_disabling(
                    Finding::new(&kinds::MODS_MOSTLY_OVERRIDDEN, mod_subject(&owner))
                        .with_arg("mod", owner.name.clone())
                        .with_arg("total", number(parts.total))
                        .with_arg("surviving", number(parts.surviving()))
                        .with_arg("percent", number(parts.surviving() * 100 / parts.total))
                        .with_arg("overriders", ranked(&parts.lost_to))
                        .with_provider(Some(owner)),
                )
            })
            .collect()
    }
}

/// The parts of one mod Mod Mallet compared, and who replaces those that
/// do not reach the game.
#[derive(Default)]
struct Parts {
    total: usize,
    /// How many parts each other mod or plugin replaces, by name.
    lost_to: BTreeMap<String, usize>,
}

impl Parts {
    /// Counts one part: replaced by `winner`, or reaching the game.
    fn add(&mut self, winner: Option<String>) {
        self.total += 1;
        if let Some(winner) = winner {
            *self.lost_to.entry(winner).or_default() += 1;
        }
    }

    fn surviving(&self) -> usize {
        self.total - self.lost_to.values().sum::<usize>()
    }
}

/// The active plugins with the mods they belong to and what they build on.
struct PluginMods {
    /// By plugin name in lower case: the file name, the mod, the masters
    /// in lower case.
    plugins: HashMap<String, (String, Option<ModRef>, HashSet<String>)>,
}

impl PluginMods {
    fn new(context: &Context<'_>) -> Self {
        let plugins = context
            .facts
            .plugins
            .iter()
            .map(|plugin| {
                (
                    plugin.name.to_lowercase(),
                    (
                        plugin.name.clone(),
                        context.mods.provider_of(&plugin.path),
                        plugin
                            .masters
                            .iter()
                            .map(|master| master.to_lowercase())
                            .collect(),
                    ),
                )
            })
            .collect();
        Self { plugins }
    }

    /// Whose the content of the plugin named `plugin` is, in any spelling.
    fn owner(&self, plugin: &str) -> Owner {
        match self.plugins.get(&plugin.to_lowercase()) {
            Some((_, Some(owner), _)) => Owner::Mod(owner.clone()),
            Some((file, None, _)) => Owner::Plugin(file.clone()),
            None => Owner::Plugin(plugin.to_owned()),
        }
    }

    /// The mod of the plugin named `plugin`, in any spelling.
    fn of(&self, plugin: &str) -> Option<ModRef> {
        self.plugins
            .get(&plugin.to_lowercase())
            .and_then(|(_, owner, _)| owner.clone())
    }

    /// The name to show for a plugin: its mod, else its file name.
    fn name(&self, plugin: &str) -> String {
        match self.plugins.get(plugin) {
            Some((_, Some(owner), _)) => owner.name.clone(),
            Some((file, None, _)) => file.clone(),
            None => plugin.to_owned(),
        }
    }

    /// The plugins of mods other than `owner` and `other` that build on a
    /// plugin of `owner` and on none of `other`: patches made for `owner`
    /// alone, by file name.
    fn patches_of(&self, owner: &ModRef, other: &Owner) -> Vec<String> {
        let belongs = |name: &str, to: &Owner| match (to, self.plugins.get(name)) {
            (Owner::Mod(to), Some((_, Some(plugin_mod), _))) => plugin_mod == to,
            (Owner::Plugin(file), _) => file.eq_ignore_ascii_case(name),
            _ => false,
        };
        let owner = Owner::Mod(owner.clone());
        let mut patches: Vec<String> = self
            .plugins
            .iter()
            .filter(|(name, _)| !belongs(name, &owner) && !belongs(name, other))
            .filter(|(_, (_, _, masters))| {
                masters.iter().any(|master| belongs(master, &owner))
                    && !masters.iter().any(|master| belongs(master, other))
            })
            .map(|(_, (file, _, _))| file.clone())
            .collect();
        patches.sort_unstable();
        patches
    }

    /// Whether a plugin of another mod builds on a plugin of `owner`.
    fn needed_by_others(&self, owner: &ModRef) -> bool {
        let own: HashSet<&String> = self
            .plugins
            .iter()
            .filter(|(_, (_, plugin_mod, _))| plugin_mod.as_ref() == Some(owner))
            .map(|(name, _)| name)
            .collect();
        self.plugins.values().any(|(_, plugin_mod, masters)| {
            plugin_mod.as_ref() != Some(owner) && masters.iter().any(|master| own.contains(master))
        })
    }

    /// Whether `plugin` names `master` among its masters, both in lower
    /// case: its author made it on top of the other, so what it replaces it
    /// carries on.
    fn builds_on(&self, plugin: &str, master: &str) -> bool {
        self.plugins
            .get(plugin)
            .is_some_and(|(_, _, masters)| masters.contains(master))
    }
}

/// The subject of a finding about a whole mod.
fn mod_subject(owner: &ModRef) -> Subject {
    Subject::Mod {
        source: owner.source.clone(),
        id: owner.id.clone(),
    }
}

/// The names with the most first, each with its number, such as
/// `JKs Skyrim (28), Lux Via (3)`.
fn ranked(counts: &BTreeMap<String, usize>) -> String {
    let mut ranked: Vec<(&String, &usize)> = counts.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    ranked
        .into_iter()
        .take(EXAMPLE_LIMIT)
        .map(|(name, count)| format!("{name} ({count})"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn number(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::objects::{
        BaseKind, BaseObject, Bounds, ObjectFacts, Place, PlacedObject,
    };
    use mallet_core::facts::{ArchiveFacts, NpcFacts, PluginFacts};
    use mallet_core::{Action, ArgValue, Facts, GamePath};

    use super::*;
    use crate::assets::testing::Mods;

    const HOUSE: u32 = 0x10;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn plugin(name: &str, masters: &[&str]) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: masters.iter().map(|master| (*master).to_owned()).collect(),
            npcs: Vec::new(),
        }
    }

    /// A house placed by `placer`, in the version of `winner`. Houses with
    /// the same `spot` stand in the same place.
    fn house(placer: &str, winner: &str, spot: u32) -> PlacedObject {
        PlacedObject {
            id: id(placer, 0x800 + spot),
            plugin: winner.to_owned(),
            base: id("Skyrim.esm", HOUSE),
            place: Place::Exterior {
                worldspace: id("Skyrim.esm", 0x3C),
            },
            position: [i32::try_from(spot).unwrap() * 10_000, 0, 0],
            rotation: [0, 0, 0],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        }
    }

    fn facts(plugins: Vec<PluginFacts>, placed: Vec<PlacedObject>) -> Facts {
        Facts {
            plugins,
            objects: ObjectFacts {
                placed,
                bases: vec![BaseObject {
                    id: id("Skyrim.esm", HOUSE),
                    editor_id: Some("House".to_owned()),
                    name: None,
                    model: None,
                    kind: BaseKind::Static,
                    solid: true,
                    natural: false,
                    bounds: Bounds {
                        min: [-400, -300, 0],
                        max: [400, 300, 600],
                    },
                }],
                place_names: Vec::new(),
            },
            ..Facts::default()
        }
    }

    fn mods() -> Mods {
        Mods(vec![
            ("data/small.esp", "Small Village"),
            ("data/large.esp", "Large Town"),
            ("data/patch.esp", "Patch"),
        ])
    }

    /// A small mod with `small` houses, each standing `shift` units from
    /// where a large mod has one of its 60.
    fn colliding(small: u32, shift: i32, extra: Vec<PluginFacts>) -> Facts {
        let mut placed: Vec<PlacedObject> = (0..small)
            .map(|spot| {
                let mut object = house("Small.esp", "Small.esp", spot);
                object.position[1] += shift;
                object
            })
            .collect();
        placed.extend((0..60).map(|spot| house("Large.esp", "Large.esp", spot)));
        let mut plugins = vec![
            plugin("Skyrim.esm", &[]),
            plugin("Small.esp", &["Skyrim.esm"]),
            plugin("Large.esp", &["Skyrim.esm"]),
        ];
        plugins.extend(extra);
        facts(plugins, placed)
    }

    #[test]
    fn the_smaller_of_two_mods_that_collide_widely_is_suggested() {
        let facts = colliding(25, 100, Vec::new());
        let mods = mods();
        let findings = WideCollisions.run(&Context::new(&facts, &mods));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.kind, "mods.collides-widely");
        assert_eq!(finding.args["mod"], ArgValue::from("Small Village"));
        assert_eq!(finding.args["count"], ArgValue::Number(25));
        assert_eq!(finding.args["modCount"], ArgValue::Number(1));
        assert_eq!(finding.args["others"], ArgValue::from("Large Town (25)"));
        assert_eq!(finding.args["objects"], ArgValue::Number(25));
        assert_eq!(finding.args["total"], ArgValue::Number(25));
        assert!(matches!(
            finding.actions.as_slice(),
            [Action::DisableMod { .. }]
        ));
    }

    #[test]
    fn a_few_collisions_are_left_to_the_list_of_collisions() {
        let facts = colliding(10, 100, Vec::new());
        let mods = mods();
        assert_eq!(WideCollisions.run(&Context::new(&facts, &mods)), []);
    }

    #[test]
    fn a_mod_others_build_on_is_not_offered_to_be_switched_off() {
        let facts = colliding(25, 100, vec![plugin("Patch.esp", &["Small.esp"])]);
        let mods = mods();
        let findings = WideCollisions.run(&Context::new(&facts, &mods));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.actions, []);
    }

    #[test]
    fn a_mod_whose_objects_another_places_as_well_is_suggested_once() {
        // The small mod's 25 houses stand exactly where the large mod has
        // 25 of its 60: the small mod is part of the large one. A patch is
        // made for the small mod alone.
        let facts = colliding(25, 0, vec![plugin("Patch.esp", &["Small.esp"])]);
        let mods = mods();
        let context = Context::new(&facts, &mods);
        let findings = DuplicateMods.run(&context);
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.kind, "mods.duplicates-another");
        assert_eq!(finding.args["mod"], ArgValue::from("Small Village"));
        assert_eq!(finding.args["other"], ArgValue::from("Large Town"));
        assert_eq!(finding.args["count"], ArgValue::Number(25));
        assert_eq!(finding.args["total"], ArgValue::Number(25));
        assert_eq!(finding.args["patchCount"], ArgValue::Number(1));
        assert_eq!(finding.args["patches"], ArgValue::from("Patch.esp"));
        assert_eq!(finding.args["otherPatchCount"], ArgValue::Number(0));
        // The same places are not suggested a second time as collisions.
        assert_eq!(WideCollisions.run(&context), []);
    }

    #[test]
    fn a_mod_only_some_of_whose_objects_another_places_as_well_is_not() {
        // 25 of the small mod's houses stand where the large mod has its
        // own, but 40 more stand elsewhere.
        let mut facts = colliding(25, 0, Vec::new());
        facts
            .objects
            .placed
            .extend((100..140).map(|spot| house("Small.esp", "Small.esp", spot)));
        let mods = mods();
        assert_eq!(DuplicateMods.run(&Context::new(&facts, &mods)), []);
    }

    /// A small mod with 25 houses, all of which a later mod changes but
    /// the first.
    fn overridden(later_masters: &[&str]) -> Facts {
        let placed = (0..25)
            .map(|spot| {
                let winner = if spot == 0 { "Small.esp" } else { "Large.esp" };
                house("Small.esp", winner, spot)
            })
            .collect();
        facts(
            vec![
                plugin("Skyrim.esm", &[]),
                plugin("Small.esp", &["Skyrim.esm"]),
                plugin("Large.esp", later_masters),
            ],
            placed,
        )
    }

    #[test]
    fn a_mod_hardly_anything_of_which_reaches_the_game_is_suggested() {
        let facts = overridden(&["Skyrim.esm"]);
        let mods = mods();
        let findings = MostlyOverridden.run(&Context::new(&facts, &mods));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.kind, "mods.mostly-overridden");
        assert_eq!(finding.args["mod"], ArgValue::from("Small Village"));
        assert_eq!(finding.args["total"], ArgValue::Number(25));
        assert_eq!(finding.args["surviving"], ArgValue::Number(1));
        assert_eq!(finding.args["percent"], ArgValue::Number(4));
        assert_eq!(
            finding.args["overriders"],
            ArgValue::from("Large Town (24)")
        );
    }

    #[test]
    fn what_a_plugin_made_for_the_mod_replaces_still_counts_as_the_mods() {
        // The later plugin builds on the small mod: its versions carry the
        // small mod's changes on, and it needs the small mod besides.
        let facts = overridden(&["Skyrim.esm", "Small.esp"]);
        let mods = mods();
        assert_eq!(MostlyOverridden.run(&Context::new(&facts, &mods)), []);
    }

    #[test]
    fn archive_files_and_characters_count_where_the_game_takes_them_from() {
        let file = |index: usize| GamePath::new(&format!("data/textures/house{index}.dds"));
        let mut facts = facts(
            vec![plugin("Small.esp", &[]), plugin("Large.esp", &[])],
            Vec::new(),
        );
        facts.archives = vec![
            ArchiveFacts {
                path: GamePath::new("data/small.bsa"),
                files: (0..30).map(file).collect(),
            },
            ArchiveFacts {
                path: GamePath::new("data/large.bsa"),
                files: (1..30).map(file).collect(),
            },
        ];
        let npc = |look: &str| NpcFacts {
            id: id("Skyrim.esm", 0x1_3BA1),
            editor_id: None,
            inherits_appearance: false,
            appearance: look.as_bytes().to_vec(),
        };
        facts.plugins[0].npcs.push(npc("small"));
        facts.plugins[1].npcs.push(npc("large"));
        let mods = Mods(vec![
            ("data/small.esp", "Small Village"),
            ("data/small.bsa", "Small Village"),
            ("data/large.esp", "Large Town"),
            ("data/large.bsa", "Large Town"),
        ]);
        let findings = MostlyOverridden.run(&Context::new(&facts, &mods));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.args["mod"], ArgValue::from("Small Village"));
        // 30 files and one character, of which only the first file is left.
        assert_eq!(finding.args["total"], ArgValue::Number(31));
        assert_eq!(finding.args["surviving"], ArgValue::Number(1));
        assert_eq!(
            finding.args["overriders"],
            ArgValue::from("Large Town (30)")
        );
    }
}
