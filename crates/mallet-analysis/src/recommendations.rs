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

/// Of two objects that collide, one that takes up at least this many times
/// the room of the other makes the decision clear: keep the big one. As
/// for the clear suggestions of the guided review (ADR 0050), measured here
/// by the objects' boxes.
const CLEAR_RATIO: f64 = 20.0;

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
        let room = rooms_of(context);

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
                if is_clear(&room, &first.object, &second.object) {
                    tally.clear += 1;
                }
                tally.objects.insert(own.object.clone());
                *tally.others.entry(other_owner.name()).or_default() += 1;
                if !tally.partners.contains(other_owner) {
                    tally.partners.push(other_owner.clone());
                }
            }
        }

        let rooms = Rooms::new(context, &mods);
        tallies
            .into_iter()
            .filter(|(_, tally)| tally.spots >= MIN_SPOTS)
            .map(|(owner, tally)| {
                let total = placed_by(&Owner::Mod(owner.clone()));
                let (interiors, interior_objects) = rooms.only(&owner, &tally.partners);
                let loot = loot_about(context, &mods, &owner, &tally.partners);
                // What relies on it, said before any advice to remove it.
                let dependents = mods.dependents_of(&owner);
                let dependent_plugins: usize = dependents.values().map(Vec::len).sum();
                let assets = asset_users(context, &mods, &owner);
                let finding = Finding::new(&kinds::MODS_COLLIDES_WIDELY, mod_subject(&owner))
                    .with_arg("mod", owner.name.clone())
                    .with_arg("count", number(tally.spots))
                    .with_arg("clearCount", number(tally.clear))
                    .with_arg("modCount", number(tally.others.len()))
                    .with_arg("others", ranked(&tally.others))
                    .with_arg("objects", number(tally.objects.len()))
                    .with_arg("total", number(total))
                    .with_arg("interiors", number(interiors))
                    .with_arg("interiorObjects", number(interior_objects))
                    .with_arg("lootCount", number(loot.len()))
                    .with_arg(
                        "loot",
                        loot.join(
                            "
",
                        ),
                    )
                    .with_arg("dependentCount", number(dependent_plugins))
                    .with_arg("dependentModCount", number(dependents.len()))
                    .with_arg("dependents", listed(&dependents))
                    .with_arg("assetCount", number(assets.values().sum()))
                    .with_arg("assetUsers", ranked(&assets))
                    .with_provider(Some(owner));
                // Switching it off stays the user's call, on a click: the
                // texts say first what relies on it and what it would take
                // away (ADR 0061).
                offer_disabling(finding)
            })
            .collect()
    }
}

/// The room each placed object takes up, by its box, in cubic game units,
/// as collision spots name them: by the plugin the object counts as, in
/// lower case, and its number.
fn rooms_of(context: &Context<'_>) -> HashMap<(String, u32), f64> {
    let bases: HashMap<&RecordId, &mallet_core::facts::objects::BaseObject> = context
        .facts
        .objects
        .bases
        .iter()
        .map(|base| (&base.id, base))
        .collect();
    context
        .facts
        .objects
        .placed
        .iter()
        .filter_map(|object| {
            let base = bases.get(&object.base)?;
            let scale = f64::from(object.scale) / 1000.0;
            let [x, y, z] = [0, 1, 2].map(|axis| {
                let min = base.bounds.min.get(axis).copied().unwrap_or_default();
                let max = base.bounds.max.get(axis).copied().unwrap_or_default();
                f64::from(max.saturating_sub(min).max(0)) * scale
            });
            Some((
                (object.owner().to_lowercase(), object.id.local_id),
                x * y * z,
            ))
        })
        .collect()
}

/// Whether one of two colliding objects is so much larger than the other
/// that keeping it is the clear decision.
fn is_clear(room: &HashMap<(String, u32), f64>, a: &ObjectRef, b: &ObjectRef) -> bool {
    let of = |object: &ObjectRef| {
        room.get(&(object.plugin.to_lowercase(), object.local_id))
            .copied()
    };
    match (of(a), of(b)) {
        (Some(a), Some(b)) => {
            let (small, large) = if a <= b { (a, b) } else { (b, a) };
            large >= CLEAR_RATIO * small
        }
        _ => false,
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
                let loot = loot_about(
                    context,
                    &mods,
                    &pair.owner,
                    std::slice::from_ref(&pair.other),
                );
                Finding::new(&kinds::MODS_DUPLICATES_ANOTHER, mod_subject(&pair.owner))
                    .with_arg("mod", pair.owner.name.clone())
                    .with_arg("other", pair.other.name())
                    .with_arg("count", number(pair.twins))
                    .with_arg("total", number(pair.total))
                    .with_arg("patchCount", number(patches.len()))
                    .with_arg("patches", names(&patches))
                    .with_arg("otherPatchCount", number(other_patches.len()))
                    .with_arg("otherPatches", names(&other_patches))
                    .with_arg("lootCount", number(loot.len()))
                    .with_arg(
                        "loot",
                        loot.join(
                            "
",
                        ),
                    )
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

/// How many of LOOT's messages a suggestion quotes.
const LOOT_LIMIT: usize = 3;

/// An interior counts as a mod's own room if it places at least this many
/// objects there...
const MIN_ROOM_OBJECTS: usize = 10;

/// ... and this many times as many as the mods it collides with together:
/// an AI overhaul's chair or a fix of the unofficial patch does not make
/// a room theirs.
const ROOM_SHARE: usize = 4;

/// Who furnishes which interior: by interior cell, how many objects each
/// mod or plugin places there.
struct Rooms<'a> {
    cells: HashMap<&'a RecordId, HashMap<Owner, usize>>,
}

impl<'a> Rooms<'a> {
    fn new(context: &'a Context<'_>, mods: &PluginMods) -> Self {
        let mut cells: HashMap<&RecordId, HashMap<Owner, usize>> = HashMap::new();
        for object in &context.facts.objects.placed {
            if let Place::Interior { cell } = &object.place {
                *cells
                    .entry(cell)
                    .or_default()
                    .entry(mods.owner(object.owner()))
                    .or_default() += 1;
            }
        }
        Self { cells }
    }

    /// The interiors `owner` furnishes and `others` hardly touch, and how
    /// many objects it places there: rooms of its own, which switching it
    /// off would take away.
    fn only(&self, owner: &ModRef, others: &[Owner]) -> (usize, usize) {
        let owner = Owner::Mod(owner.clone());
        self.cells
            .values()
            .filter_map(|furnishers| {
                let own = furnishers.get(&owner).copied().unwrap_or_default();
                let theirs: usize = others
                    .iter()
                    .filter_map(|other| furnishers.get(other))
                    .sum();
                (own >= MIN_ROOM_OBJECTS && own >= theirs * ROOM_SHARE).then_some(own)
            })
            .fold((0, 0), |(rooms, objects), count| {
                (rooms + 1, objects + count)
            })
    }
}

/// What LOOT says about `owner` and the mods it collides with: its general
/// messages that name a plugin of `owner`, and the messages on a plugin of
/// either side that name the other side. Each once, at most
/// [`LOOT_LIMIT`].
fn loot_about(
    context: &Context<'_>,
    mods: &PluginMods,
    owner: &ModRef,
    others: &[Owner],
) -> Vec<String> {
    let own = Owner::Mod(owner.clone());
    let knowledge = &context.facts.knowledge;
    let names_of = |side: &Owner| -> Vec<String> {
        let mut names: Vec<String> = mods
            .plugins_of(side)
            .iter()
            .map(|plugin| plugin_stem(plugin).to_lowercase())
            .collect();
        if let Owner::Mod(side) = side {
            names.push(side.name.to_lowercase());
        }
        names.retain(|name| name.len() >= 4);
        names
    };
    let own_names = names_of(&own);
    let other_names: Vec<String> = others.iter().flat_map(names_of).collect();
    let mentions = |text: &str, names: &[String]| {
        let text = text.to_lowercase();
        names.iter().any(|name| text.contains(name.as_str()))
    };

    let mut texts: Vec<String> = Vec::new();
    let mut add = |text: &str| {
        if texts.len() < LOOT_LIMIT && !texts.iter().any(|known| known == text) {
            texts.push(text.to_owned());
        }
    };
    for general in &knowledge.general {
        if general
            .plugins
            .iter()
            .any(|plugin| mods.owner(plugin) == own)
        {
            add(&general.message.text);
        }
    }
    for plugin in &knowledge.plugins {
        let side = mods.owner(&plugin.plugin);
        let about = if side == own {
            &other_names
        } else if others.contains(&side) {
            &own_names
        } else {
            continue;
        };
        for message in &plugin.messages {
            if mentions(&message.text, about) {
                add(&message.text);
            }
        }
    }
    texts
}

/// A plugin's file name without its extension.
fn plugin_stem(plugin: &str) -> &str {
    plugin.rsplit_once('.').map_or(plugin, |(stem, _)| stem)
}

/// What one mod's objects collide with.
#[derive(Default)]
struct Tally {
    /// The collision spots the mod is part of.
    spots: usize,
    /// Those of them where one object is far larger than the other, so
    /// that the decision is clear (see [`CLEAR_RATIO`]).
    clear: usize,
    /// The mod's own objects among them.
    objects: BTreeSet<ObjectRef>,
    /// How many spots it shares with each other mod, by name.
    others: BTreeMap<String, usize>,
    /// The mods and plugins it collides with.
    partners: Vec<Owner>,
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

        count_loose_files(context, &mut count);

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

/// Counts the loose files of each mod. Every file a mod brings, where the
/// mod manager lists them, counts by whether the manager deployed this
/// mod's copy or another mod's (ADR 0049). A file no mod deployed says
/// nothing, since the manager may have been told to leave it out. Where the
/// manager does not list them, the deployed files reach the game by being
/// there.
fn count_loose_files(
    context: &Context<'_>,
    count: &mut impl FnMut(Option<ModRef>, Option<String>),
) {
    let mut deployed: BTreeMap<ModRef, usize> = BTreeMap::new();
    for file in &context.facts.loose_files {
        if let Some(owner) = context.mods.provider_of(file) {
            *deployed.entry(owner).or_default() += 1;
        }
    }
    for (owner, deployed_count) in deployed {
        match context.mods.staged_files(&owner) {
            Some(files) => {
                for file in files {
                    match context.mods.provider_of(&file) {
                        Some(winner) if winner == owner => count(Some(owner.clone()), None),
                        Some(winner) => count(Some(owner.clone()), Some(winner.name)),
                        None => {}
                    }
                }
            }
            None => {
                for _ in 0..deployed_count {
                    count(Some(owner.clone()), None);
                }
            }
        }
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

    /// The file names of the active plugins whose content is `owner`'s.
    fn plugins_of(&self, owner: &Owner) -> Vec<String> {
        self.plugins
            .values()
            .filter(|(file, plugin_mod, _)| match owner {
                Owner::Mod(owner) => plugin_mod.as_ref() == Some(owner),
                Owner::Plugin(plugin) => file.eq_ignore_ascii_case(plugin),
            })
            .map(|(file, _, _)| file.clone())
            .collect()
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

    /// The plugins of other mods that build on a plugin of `owner`, by the
    /// name of their mod (or their file name, where no mod claims them).
    fn dependents_of(&self, owner: &ModRef) -> BTreeMap<String, Vec<String>> {
        let own: HashSet<&String> = self
            .plugins
            .iter()
            .filter(|(_, (_, plugin_mod, _))| plugin_mod.as_ref() == Some(owner))
            .map(|(name, _)| name)
            .collect();
        let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (file, plugin_mod, masters) in self.plugins.values() {
            if plugin_mod.as_ref() != Some(owner)
                && masters.iter().any(|master| own.contains(master))
            {
                let name = plugin_mod
                    .as_ref()
                    .map_or_else(|| file.clone(), |plugin_mod| plugin_mod.name.clone());
                found.entry(name).or_default().push(file.clone());
            }
        }
        for files in found.values_mut() {
            files.sort_unstable();
        }
        found
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

/// How many plugins of one mod are named before the rest are counted.
const PLUGINS_PER_MOD: usize = 2;

/// Mods with their plugins, such as `Patch Hub (A.esp, B.esp, +3), C.esp`:
/// a plugin no mod claims stands for itself.
fn listed(dependents: &BTreeMap<String, Vec<String>>) -> String {
    dependents
        .iter()
        .take(EXAMPLE_LIMIT)
        .map(|(name, files)| match files.as_slice() {
            [file] if file == name => name.clone(),
            _ => {
                let mut shown: Vec<String> = files.iter().take(PLUGINS_PER_MOD).cloned().collect();
                if files.len() > PLUGINS_PER_MOD {
                    shown.push(format!("+{}", files.len() - PLUGINS_PER_MOD));
                }
                format!("{name} ({})", shown.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The other mods whose objects show models that only `owner` brings,
/// with how many of their base objects do: without the mod they would
/// show nothing. A model another archive or loose file has as well is left
/// out, since the game would take that one instead.
fn asset_users(
    context: &Context<'_>,
    mods: &PluginMods,
    owner: &ModRef,
) -> BTreeMap<String, usize> {
    let own = Owner::Mod(owner.clone());
    let data = mallet_core::GamePath::new("data");
    let candidates: Vec<(mallet_core::GamePath, Owner)> = context
        .facts
        .objects
        .bases
        .iter()
        .filter_map(|base| {
            let model = base.model.as_deref()?;
            let user = mods.owner(&base.id.plugin);
            (user != own).then(|| (data.join(model), user))
        })
        .collect();
    if candidates.is_empty() {
        return BTreeMap::new();
    }
    let wanted: HashSet<&mallet_core::GamePath> = candidates.iter().map(|(path, _)| path).collect();
    // Every source of each wanted model: the mod that deployed it, if any.
    let mut sources: HashMap<&mallet_core::GamePath, Vec<Option<ModRef>>> = HashMap::new();
    for archive in &context.facts.archives {
        let provider = context.mods.provider_of(&archive.path);
        for file in &archive.files {
            if let Some(path) = wanted.get(file) {
                sources.entry(path).or_default().push(provider.clone());
            }
        }
    }
    for path in &wanted {
        if context.assets.is_loose(path) {
            sources
                .entry(path)
                .or_default()
                .push(context.mods.provider_of(path));
        }
    }
    let only_owner = |path: &mallet_core::GamePath| {
        sources.get(path).is_some_and(|found| {
            !found.is_empty() && found.iter().all(|source| source.as_ref() == Some(owner))
        })
    };
    let mut users: BTreeMap<String, usize> = BTreeMap::new();
    for (path, user) in &candidates {
        if only_owner(path) {
            *users.entry(user.name()).or_default() += 1;
        }
    }
    users
}

fn number(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::knowledge::{GeneralMessage, KnownMessage, MessageLevel};
    use mallet_core::facts::objects::{
        BaseKind, BaseObject, Bounds, ObjectFacts, Place, PlacedObject,
    };
    use mallet_core::facts::{ArchiveFacts, NpcFacts, PluginFacts};
    use mallet_core::{Action, ArgValue, Facts, GamePath, NoModSource};

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
        // Two houses of a size: none of the places is a clear decision.
        assert_eq!(finding.args["clearCount"], ArgValue::Number(0));
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
    fn a_lantern_against_a_house_is_a_clear_decision_two_houses_are_not() {
        let mut facts = colliding(1, 100, Vec::new());
        // The small mod's house is shrunk to a lantern's size.
        if let Some(lantern) = facts.objects.placed.first_mut() {
            lantern.scale = 300;
        }
        let room = rooms_of(&Context::new(&facts, &NoModSource));
        let object = |plugin: &str| ObjectRef {
            plugin: plugin.to_owned(),
            local_id: 0x800,
        };
        assert!(is_clear(&room, &object("Small.esp"), &object("Large.esp")));
        assert!(!is_clear(&room, &object("Large.esp"), &object("Large.esp")));
        assert!(!is_clear(
            &room,
            &object("Small.esp"),
            &object("Unknown.esp")
        ));
    }

    #[test]
    fn a_mod_with_rooms_of_its_own_is_kept_and_loot_is_quoted() {
        // Besides its houses outside, the small mod furnishes a room in
        // which the large mod places a single object.
        let mut facts = colliding(25, 100, Vec::new());
        let room = Place::Interior {
            cell: id("Small.esp", 0x900),
        };
        for spot in 0..12 {
            let mut object = house("Small.esp", "Small.esp", 200 + spot);
            object.place = room.clone();
            facts.objects.placed.push(object);
        }
        let mut theirs = house("Large.esp", "Large.esp", 300);
        theirs.place = room;
        facts.objects.placed.push(theirs);
        facts.knowledge.general.push(GeneralMessage {
            message: KnownMessage {
                level: MessageLevel::Note,
                text: "Use the Village Patch Collection.".to_owned(),
            },
            plugins: vec!["Small.esp".to_owned()],
        });
        let mods = mods();
        let findings = WideCollisions.run(&Context::new(&facts, &mods));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.args["interiors"], ArgValue::Number(1));
        assert_eq!(finding.args["interiorObjects"], ArgValue::Number(12));
        assert_eq!(finding.args["lootCount"], ArgValue::Number(1));
        assert_eq!(
            finding.args["loot"],
            ArgValue::from("Use the Village Patch Collection.")
        );
        // Switching it off would take the room away, which the texts say;
        // whether to is the user's call, on a click (ADR 0061).
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
    fn what_relies_on_a_mod_is_named_before_switching_it_off() {
        let mut facts = colliding(25, 100, vec![plugin("Patch.esp", &["Small.esp"])]);
        // The large mod shows a model only the small mod's archive has.
        facts.objects.bases.push(BaseObject {
            id: id("Large.esp", 0x700),
            editor_id: Some("Lantern".to_owned()),
            name: None,
            model: Some("meshes/village/lantern.nif".to_owned()),
            kind: BaseKind::Static,
            solid: false,
            natural: false,
            bounds: Bounds {
                min: [0, 0, 0],
                max: [10, 10, 10],
            },
        });
        facts.archives.push(ArchiveFacts {
            path: GamePath::new("data/small.bsa"),
            files: vec![GamePath::new("data/meshes/village/lantern.nif")],
        });
        let mods = Mods(vec![
            ("data/small.esp", "Small Village"),
            ("data/small.bsa", "Small Village"),
            ("data/large.esp", "Large Town"),
            ("data/patch.esp", "Patch"),
        ]);
        let findings = WideCollisions.run(&Context::new(&facts, &mods));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.args["dependentCount"], ArgValue::Number(1));
        assert_eq!(finding.args["dependentModCount"], ArgValue::Number(1));
        assert_eq!(
            finding.args["dependents"],
            ArgValue::from("Patch (Patch.esp)")
        );
        assert_eq!(finding.args["assetCount"], ArgValue::Number(1));
        assert_eq!(finding.args["assetUsers"], ArgValue::from("Large Town (1)"));
        // Offered, on a click, with the dependents named in the texts.
        assert!(matches!(
            finding.actions.as_slice(),
            [Action::DisableMod { .. }]
        ));
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

    /// A mod manager that also lists the files each mod brings.
    struct Staging {
        deployed: Mods,
        staged: Vec<(&'static str, Vec<GamePath>)>,
    }

    impl mallet_core::ModSource for Staging {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            self.deployed.provider_of(path)
        }

        fn staged_files(&self, mod_ref: &ModRef) -> Option<Vec<GamePath>> {
            self.staged
                .iter()
                .find(|(name, _)| *name == mod_ref.name)
                .map(|(_, files)| files.clone())
        }
    }

    #[test]
    fn loose_files_the_manager_kept_back_do_not_reach_the_game() {
        // An old texture pack brings 30 textures; a newer one wins for 29
        // of them, so the manager deployed only one of the old pack's.
        let texture = |index: usize| format!("data/textures/rock{index}.dds");
        let paths: Vec<String> = (0..30).map(texture).collect();
        let facts = Facts {
            loose_files: paths.iter().map(|path| GamePath::new(path)).collect(),
            ..Facts::default()
        };
        let deployed: Vec<(&'static str, &'static str)> = paths
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let path: &'static str = Box::leak(path.clone().into_boxed_str());
                (path, if index == 0 { "Old Rocks" } else { "New Rocks" })
            })
            .collect();
        let staging = Staging {
            deployed: Mods(deployed),
            staged: vec![
                (
                    "Old Rocks",
                    paths.iter().map(|path| GamePath::new(path)).collect(),
                ),
                (
                    "New Rocks",
                    paths
                        .iter()
                        .skip(1)
                        .map(|path| GamePath::new(path))
                        .collect(),
                ),
            ],
        };
        let findings = MostlyOverridden.run(&Context::new(&facts, &staging));
        let [finding] = findings.as_slice() else {
            panic!("expected one suggestion, got {findings:?}");
        };
        assert_eq!(finding.args["mod"], ArgValue::from("Old Rocks"));
        assert_eq!(finding.args["total"], ArgValue::Number(30));
        assert_eq!(finding.args["surviving"], ArgValue::Number(1));
        assert_eq!(finding.args["overriders"], ArgValue::from("New Rocks (29)"));

        // Without the lists, the deployed files are all that is known.
        let mods = Mods(staging.deployed.0.clone());
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
