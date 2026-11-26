//! The things of the game one can look at, read from the plugins
//! (ADR 0054): weapons, armour, characters and other things with a model,
//! and the outfits characters wear.
//!
//! Only the top-level groups of the record types listed in [`RECORDS`] are
//! read, from every active plugin in load order. A later plugin's version
//! of a record replaces an earlier one, as in the game. Names of localized
//! plugins come from their string tables.
//!
//! The record layouts are those of Skyrim and Fallout 4 (ADR 0092), which
//! differ in how a weapon says what it is and who may use it, and in the
//! body slots; the older games of the family have no catalogue yet.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use mallet_core::WorldOrigin;
use mallet_core::catalogue::{
    Catalogue, CatalogueItem, CatalogueOutfit, CatalogueSource, ItemCategory,
};
use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, Plugin, Record, Signature};
use mallet_formats_bethesda::strings;

use crate::game::{Family, Game};
use crate::ingest;
use crate::lod_library::LodLibrary;
use crate::records::{self, FLAG_DELETED};

/// The record types read for the catalogue.
pub const RECORDS: [Signature; 22] = [
    *b"WEAP", *b"ARMO", *b"ARMA", *b"AMMO", *b"NPC_", *b"OTFT", *b"LVLI", *b"KYWD", *b"RACE",
    *b"MISC", *b"BOOK", *b"ALCH", *b"INGR", *b"KEYM", *b"SLGM", *b"SCRL", *b"FURN", *b"CONT",
    *b"DOOR", *b"ACTI", *b"FLOR", *b"LIGH",
];

/// Record flag of armour: it cannot be worn by players (a creature's skin,
/// for example).
const FLAG_ARMOUR_NOT_PLAYABLE: u32 = 0x0000_0004;

/// Record flag of Fallout 4's weapons, armour and ammunition: players
/// cannot use it (a creature's claws, a turret's ammunition).
const FLAG_FALLOUT4_NOT_PLAYABLE: u32 = 0x0000_0004;

/// Flag in a weapon's `DNAM`: it cannot be used by players (a creature's
/// claws, for example).
const WEAPON_NOT_PLAYABLE: u16 = 0x0080;

/// Flag in a character's `ACBS`: there is only one of them.
const NPC_UNIQUE: u32 = 0x0000_0020;

/// How deep leveled lists are followed.
const MAX_LIST_DEPTH: usize = 8;

/// How many weapons an outfit lists at most.
const MAX_OUTFIT_WEAPONS: usize = 12;

/// Where the game keeps the face of a character with a look of its own.
const FACE_MESHES: &str = "meshes/actors/character/facegendata/facegeom";

/// The version of a record the game uses, as far as the catalogue needs it.
#[derive(Debug, Clone, Default)]
struct Entry {
    signature: Signature,
    editor_id: Option<String>,
    name: Option<String>,
    /// `MODL`, or for armour the model lying on the ground (`MOD2`).
    model: Option<String>,
    keywords: Vec<RecordId>,
    /// Armour: its armour addons. Outfits and leveled lists: their
    /// entries. Characters: what they carry.
    references: Vec<RecordId>,
    template: Option<RecordId>,
    /// Armour: the body slots it covers.
    slots: u32,
    /// Weapons: how it is held.
    animation: Option<u8>,
    playable: bool,
    race: Option<RecordId>,
    outfit: Option<RecordId>,
    unique: bool,
    /// Characters: whether the look is its own rather than a template's.
    own_face: bool,
}

/// What is known about one record across the plugins.
#[derive(Debug, Clone)]
struct Known {
    /// The plugins that change it after the one that defines it.
    changed_by: Vec<String>,
    /// The version the game uses.
    entry: Entry,
}

/// Gathers the catalogue from the plugins, which have to be added in load
/// order.
#[derive(Debug, Default)]
pub struct Collector {
    game: Game,
    records: HashMap<RecordId, Known>,
    /// The plugins added, in load order.
    plugins: Vec<String>,
}

impl Collector {
    /// A collector for the plugins of `game`.
    pub fn new(game: Game) -> Self {
        Self {
            game,
            ..Self::default()
        }
    }

    /// Takes in the plugin `plugin_name`, read with [`RECORDS`] among its
    /// wanted types. `names` looks up the name of a localized plugin by its
    /// number.
    pub fn add(&mut self, plugin_name: &str, plugin: &Plugin, names: &HashMap<u32, String>) {
        self.plugins.push(plugin_name.to_lowercase());
        let masters = &plugin.header.masters;
        let localized = plugin.header.is_localized;
        for record in &plugin.records {
            let id = records::record_id(record.form_id, masters, plugin_name);
            if record.flags & FLAG_DELETED != 0 {
                self.records.remove(&id);
                continue;
            }
            let entry = entry(self.game, record, masters, plugin_name, localized, names);
            match self.records.get_mut(&id) {
                Some(known) => {
                    known.changed_by.push(plugin_name.to_lowercase());
                    known.entry = entry;
                }
                None => {
                    self.records.insert(
                        id,
                        Known {
                            changed_by: Vec::new(),
                            entry,
                        },
                    );
                }
            }
        }
    }

    /// The catalogue of the plugins added. `origin` tells where a plugin
    /// comes from; the mods that bring them are left for the caller.
    pub fn finish(self, origin: &dyn Fn(&str) -> WorldOrigin) -> Catalogue {
        let sources = self
            .plugins
            .iter()
            .map(|plugin| CatalogueSource {
                plugin: plugin.clone(),
                origin: origin(plugin),
                mod_ref: None,
            })
            .collect();
        let candidates: HashMap<&RecordId, CatalogueItem> = self
            .records
            .iter()
            .filter_map(|(id, known)| Some((id, self.item(id, known)?)))
            .collect();
        // Variants are counted at the thing they are made from.
        let mut variants: HashMap<RecordId, u32> = HashMap::new();
        for id in candidates.keys() {
            let root = self.root(id, &candidates);
            if root != **id {
                *variants.entry(root).or_default() += 1;
            }
        }
        let mut items: Vec<CatalogueItem> = candidates
            .iter()
            .filter(|(id, _)| self.root(id, &candidates) == ***id)
            .map(|(id, item)| CatalogueItem {
                variants: variants.get(*id).copied().unwrap_or_default(),
                ..item.clone()
            })
            .collect();
        items.sort_by(|a, b| {
            let name = |item: &CatalogueItem| {
                item.name
                    .clone()
                    .or_else(|| item.editor_id.clone())
                    .unwrap_or_default()
                    .to_lowercase()
            };
            name(a).cmp(&name(b)).then_with(|| a.id.cmp(&b.id))
        });
        let listed: HashSet<&str> = items.iter().map(|item| item.id.as_str()).collect();
        let mut outfits = self.outfits(&listed, &|id| key(&self.root(id, &candidates)));
        outfits.sort_by(|a, b| a.editor_id.cmp(&b.editor_id).then_with(|| a.id.cmp(&b.id)));
        Catalogue {
            items,
            outfits,
            sources,
        }
    }

    fn get(&self, id: &RecordId) -> Option<&Entry> {
        self.records.get(id).map(|known| &known.entry)
    }

    /// The thing `id` is a variant of, following templates as far as they
    /// lead to things that are listed; `id` itself if it is none's. Where
    /// templates go round in a circle, the lowest of them.
    fn root(&self, id: &RecordId, candidates: &HashMap<&RecordId, CatalogueItem>) -> RecordId {
        let mut seen = vec![id.clone()];
        for _ in 0..MAX_LIST_DEPTH {
            let Some(template) = seen
                .last()
                .and_then(|current| self.get(current))
                .and_then(|entry| entry.template.as_ref())
                .filter(|template| candidates.contains_key(template))
            else {
                break;
            };
            if seen.contains(template) {
                return seen.iter().min().cloned().unwrap_or_else(|| id.clone());
            }
            seen.push(template.clone());
        }
        seen.pop().unwrap_or_else(|| id.clone())
    }

    /// The editor IDs of `keywords`.
    fn keyword_names<'a>(&'a self, keywords: &'a [RecordId]) -> impl Iterator<Item = &'a str> {
        keywords
            .iter()
            .filter_map(|keyword| self.get(keyword)?.editor_id.as_deref())
    }

    /// The catalogue's view of one record, if it is a thing to list.
    fn item(&self, id: &RecordId, known: &Known) -> Option<CatalogueItem> {
        let entry = &known.entry;
        if !entry.playable || (entry.signature == *b"NPC_" && !entry.unique) {
            return None;
        }
        let category = self.category(entry)?;
        let models = self.models(id, entry);
        if models.is_empty() {
            return None;
        }
        let material = self.keyword_names(&entry.keywords).find_map(material);
        let race = entry
            .race
            .as_ref()
            .and_then(|race| self.get(race))
            .and_then(|race| race.name.clone().or_else(|| race.editor_id.clone()));
        Some(CatalogueItem {
            id: key(id),
            category,
            name: entry.name.clone(),
            editor_id: entry.editor_id.clone(),
            material,
            plugin: id.plugin.clone(),
            changed_by: known.changed_by.clone(),
            models,
            variants: 0,
            race,
            outfit: entry.outfit.as_ref().map(key),
        })
    }

    fn category(&self, entry: &Entry) -> Option<ItemCategory> {
        Some(match &entry.signature {
            b"WEAP" => self.weapon_category(entry),
            b"ARMO" if self.game.family() == Family::Fallout4 => {
                fallout4_armour_category(entry.slots)
            }
            b"ARMO" => armour_category(entry.slots),
            b"AMMO" => ItemCategory::Ammunition,
            b"NPC_" => ItemCategory::Character,
            b"BOOK" | b"SCRL" => ItemCategory::Book,
            b"ALCH" => ItemCategory::Potion,
            b"INGR" => ItemCategory::Ingredient,
            b"KEYM" => ItemCategory::Key,
            b"SLGM" => ItemCategory::SoulGem,
            b"MISC" => ItemCategory::Misc,
            b"FURN" => ItemCategory::Furniture,
            b"CONT" => ItemCategory::Container,
            b"DOOR" => ItemCategory::Door,
            b"ACTI" => ItemCategory::Activator,
            b"FLOR" => ItemCategory::Flora,
            b"LIGH" => ItemCategory::Light,
            _ => return None,
        })
    }

    /// The kind of weapon, by its keywords where it has the game's, else by
    /// how it is held.
    fn weapon_category(&self, entry: &Entry) -> ItemCategory {
        if self.game.family() == Family::Fallout4 {
            return self.fallout4_weapon_category(entry);
        }
        let by_keyword = self.keyword_names(&entry.keywords).find_map(|keyword| {
            match keyword.to_ascii_lowercase().as_str() {
                "weaptypesword" => Some(ItemCategory::Sword),
                "weaptypedagger" => Some(ItemCategory::Dagger),
                "weaptypewaraxe" => Some(ItemCategory::WarAxe),
                "weaptypemace" => Some(ItemCategory::Mace),
                "weaptypegreatsword" => Some(ItemCategory::Greatsword),
                "weaptypebattleaxe" => Some(ItemCategory::Battleaxe),
                "weaptypewarhammer" => Some(ItemCategory::Warhammer),
                "weaptypebow" => Some(ItemCategory::Bow),
                "weaptypestaff" => Some(ItemCategory::Staff),
                _ => None,
            }
        });
        by_keyword.unwrap_or(match entry.animation {
            Some(1) => ItemCategory::Sword,
            Some(2) => ItemCategory::Dagger,
            Some(3) => ItemCategory::WarAxe,
            Some(4) => ItemCategory::Mace,
            Some(5) => ItemCategory::Greatsword,
            Some(6) => ItemCategory::Battleaxe,
            Some(7) => ItemCategory::Bow,
            Some(8) => ItemCategory::Staff,
            Some(9) => ItemCategory::Crossbow,
            _ => ItemCategory::OtherWeapon,
        })
    }

    /// The kind of weapon of Fallout 4, by its `WeaponType…` keywords. A
    /// weapon has several (`WeaponTypeBallistic`, `WeaponTypePistol`); the
    /// one that says most wins.
    fn fallout4_weapon_category(&self, entry: &Entry) -> ItemCategory {
        let kinds: Vec<String> = self
            .keyword_names(&entry.keywords)
            .filter_map(|keyword| {
                let lower = keyword.to_ascii_lowercase();
                lower.strip_prefix("weapontype").map(str::to_owned)
            })
            .collect();
        let has = |names: &[&str]| kinds.iter().any(|kind| names.contains(&kind.as_str()));
        if has(&["heavygun"]) {
            ItemCategory::HeavyWeapon
        } else if has(&["grenade", "mine", "thrown", "explosive", "molotov"]) {
            ItemCategory::Explosive
        } else if has(&["shotgun"]) {
            ItemCategory::Shotgun
        } else if has(&["pistol"]) {
            ItemCategory::Pistol
        } else if has(&[
            "rifle",
            "assaultrifle",
            "sniper",
            "gaussrifle",
            "railwayrifle",
            "lasermusket",
        ]) {
            ItemCategory::Rifle
        } else if has(&["melee1h", "melee2h", "unarmed", "handtohand"]) {
            ItemCategory::MeleeWeapon
        } else {
            ItemCategory::OtherWeapon
        }
    }

    /// The models of a thing: its own, then for armour the ones worn.
    fn models(&self, id: &RecordId, entry: &Entry) -> Vec<String> {
        if entry.signature == *b"NPC_" {
            if !entry.own_face {
                return Vec::new();
            }
            return vec![format!(
                "{FACE_MESHES}/{}/{:08x}.nif",
                id.plugin, id.local_id
            )];
        }
        let mut models: Vec<String> = entry.model.iter().cloned().collect();
        if entry.signature == *b"ARMO" {
            for addon in &entry.references {
                if let Some(worn) = self.get(addon).and_then(|addon| addon.model.clone())
                    && !models.contains(&worn)
                {
                    models.push(worn);
                }
            }
        }
        models
    }

    /// Every thing a leveled list or outfit can give, following nested
    /// lists.
    fn expand(&self, references: &[RecordId], depth: usize, found: &mut Vec<RecordId>) {
        for reference in references {
            match self.get(reference) {
                Some(entry) if entry.signature == *b"LVLI" => {
                    if depth < MAX_LIST_DEPTH {
                        self.expand(&entry.references, depth + 1, found);
                    }
                }
                Some(_) if !found.contains(reference) => found.push(reference.clone()),
                _ => {}
            }
        }
    }

    /// The outfits, with what they consist of and what their wearers carry.
    /// `shown` gives the identity a thing is listed under: a variant's is
    /// that of the thing it is made from.
    fn outfits(
        &self,
        listed: &HashSet<&str>,
        shown: &dyn Fn(&RecordId) -> String,
    ) -> Vec<CatalogueOutfit> {
        // What the wearers of each outfit carry, and how many there are.
        let mut wearers: HashMap<&RecordId, (u32, BTreeMap<String, u32>)> = HashMap::new();
        for known in self.records.values() {
            let entry = &known.entry;
            let (b"NPC_", Some(outfit)) = (&entry.signature, &entry.outfit) else {
                continue;
            };
            let (count, weapons) = wearers.entry(outfit).or_default();
            *count += 1;
            let mut carried = Vec::new();
            self.expand(&entry.references, 0, &mut carried);
            let carried: HashSet<String> = carried
                .iter()
                .filter(|item| {
                    self.get(item)
                        .is_some_and(|entry| entry.signature == *b"WEAP")
                })
                .map(shown)
                .collect();
            for weapon in carried {
                *weapons.entry(weapon).or_default() += 1;
            }
        }
        self.records
            .iter()
            .filter(|(_, known)| known.entry.signature == *b"OTFT")
            .filter_map(|(id, known)| {
                let mut found = Vec::new();
                self.expand(&known.entry.references, 0, &mut found);
                let mut worn: Vec<String> = Vec::new();
                for item in found.iter().map(shown) {
                    if listed.contains(item.as_str()) && !worn.contains(&item) {
                        worn.push(item);
                    }
                }
                if worn.is_empty() {
                    return None;
                }
                let (characters, carried) = wearers.get(id).cloned().unwrap_or_default();
                let mut weapons: Vec<(String, u32)> = carried.into_iter().collect();
                weapons.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                let weapons = weapons
                    .into_iter()
                    .map(|(weapon, _)| weapon)
                    .filter(|weapon| listed.contains(weapon.as_str()))
                    .take(MAX_OUTFIT_WEAPONS)
                    .collect();
                Some(CatalogueOutfit {
                    id: key(id),
                    editor_id: known.entry.editor_id.clone(),
                    plugin: id.plugin.clone(),
                    worn,
                    weapons,
                    characters,
                })
            })
            .collect()
    }
}

/// The identity of a record in the catalogue.
fn key(id: &RecordId) -> String {
    format!("{}/{:08x}", id.plugin, id.local_id)
}

/// What one record says, with its references resolved.
fn entry(
    game: Game,
    record: &Record,
    masters: &[String],
    plugin_name: &str,
    localized: bool,
    names: &HashMap<u32, String>,
) -> Entry {
    let id = |form_id: u32| records::record_id(form_id, masters, plugin_name);
    let form_ids = |data: &[u8]| -> Vec<RecordId> {
        data.as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_le_bytes(*bytes))
            .filter(|&form_id| form_id != 0)
            .map(id)
            .collect()
    };
    let form_id = |signature: &Signature| {
        record
            .first(signature)
            .and_then(|data| data.get(..4))
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .filter(|&form_id| form_id != 0)
            .map(id)
    };
    let mut entry = Entry {
        signature: record.signature,
        editor_id: records::editor_id(record),
        name: name(record, localized, names),
        model: model(record, *b"MODL"),
        keywords: record.first(b"KWDA").map_or_default(form_ids),
        playable: true,
        ..Entry::default()
    };
    match &record.signature {
        b"WEAP" if game.family() == Family::Fallout4 => {
            // Fallout 4's `DNAM` starts with the ammunition; what the weapon
            // is comes from its keywords alone.
            entry.playable = record.flags & FLAG_FALLOUT4_NOT_PLAYABLE == 0;
            entry.template = form_id(b"CNAM");
        }
        b"AMMO" if game.family() == Family::Fallout4 => {
            entry.playable = record.flags & FLAG_FALLOUT4_NOT_PLAYABLE == 0;
        }
        b"WEAP" => {
            let data = record.first(b"DNAM");
            entry.animation = data.and_then(|data| data.first().copied());
            let flags = data
                .and_then(|data| data.get(12..14))
                .and_then(|bytes| bytes.try_into().ok())
                .map_or(0, u16::from_le_bytes);
            entry.playable = flags & WEAPON_NOT_PLAYABLE == 0;
            entry.template = form_id(b"CNAM");
        }
        b"ARMO" => {
            entry.model = model(record, *b"MOD2").or_else(|| model(record, *b"MOD4"));
            entry.references = record
                .subrecords
                .iter()
                .filter(|field| &field.signature == b"MODL")
                .flat_map(|field| form_ids(&field.data))
                .collect();
            entry.slots = record
                .first(b"BOD2")
                .or_else(|| record.first(b"BODT"))
                .and_then(|data| data.get(..4))
                .and_then(|bytes| bytes.try_into().ok())
                .map_or(0, u32::from_le_bytes);
            entry.playable = record.flags & FLAG_ARMOUR_NOT_PLAYABLE == 0;
            entry.template = form_id(b"TNAM");
        }
        b"ARMA" => {
            entry.model = model(record, *b"MOD2").or_else(|| model(record, *b"MOD3"));
        }
        b"NPC_" => {
            entry.race = form_id(b"RNAM");
            entry.outfit = form_id(b"DOFT");
            entry.references = record
                .subrecords
                .iter()
                .filter(|field| &field.signature == b"CNTO")
                .filter_map(|field| field.data.get(..4))
                .flat_map(form_ids)
                .collect();
            entry.unique = record
                .first(b"ACBS")
                .and_then(|data| data.get(..4))
                .and_then(|bytes| bytes.try_into().ok())
                .map_or(0, u32::from_le_bytes)
                & NPC_UNIQUE
                != 0;
            entry.own_face = !crate::npc::inherits_appearance_of(game, record);
        }
        b"OTFT" => {
            entry.references = record.first(b"INAM").map_or_default(form_ids);
        }
        b"LVLI" => {
            // Each entry: level (2 bytes), 2 unused, the thing, count.
            entry.references = record
                .subrecords
                .iter()
                .filter(|field| &field.signature == b"LVLO")
                .filter_map(|field| field.data.get(4..8))
                .flat_map(form_ids)
                .collect();
        }
        _ => {}
    }
    entry
}

/// The kind of thing worn, by the body slots it covers. Slot 30 is the
/// lowest bit.
fn armour_category(slots: u32) -> ItemCategory {
    const HEAD: u32 = 1 << 0 | 1 << 1 | 1 << 12;
    const BODY: u32 = 1 << 2;
    const HANDS: u32 = 1 << 3 | 1 << 4;
    const FEET: u32 = 1 << 7 | 1 << 8;
    const SHIELD: u32 = 1 << 9;
    const JEWELLERY: u32 = 1 << 5 | 1 << 6;
    if slots & BODY != 0 {
        ItemCategory::Body
    } else if slots & HEAD != 0 {
        ItemCategory::Head
    } else if slots & HANDS != 0 {
        ItemCategory::Hands
    } else if slots & FEET != 0 {
        ItemCategory::Feet
    } else if slots & SHIELD != 0 {
        ItemCategory::Shield
    } else if slots & JEWELLERY != 0 {
        ItemCategory::Jewellery
    } else {
        ItemCategory::OtherWorn
    }
}

/// The kind of thing worn in Fallout 4, by the body slots it covers. Slot
/// 30 is the lowest bit. Clothes and armour cover the body (33), the
/// clothes under armour (36 to 40) or the armour's pieces (41 to 45).
fn fallout4_armour_category(slots: u32) -> ItemCategory {
    const BODY: u32 = 1 << 3 | 1 << 6 | 1 << 11;
    // Hair, headband, eyes, beard, mouth, scalp.
    const HEAD: u32 = 1 << 0 | 1 << 1 | 1 << 16 | 1 << 17 | 1 << 18 | 1 << 19 | 1 << 22;
    const ARMS: u32 = 1 << 7 | 1 << 8 | 1 << 12 | 1 << 13;
    const LEGS: u32 = 1 << 9 | 1 << 10 | 1 << 14 | 1 << 15;
    const HANDS: u32 = 1 << 4 | 1 << 5;
    const JEWELLERY: u32 = 1 << 20 | 1 << 21;
    const SHIELD: u32 = 1 << 29;
    if slots & BODY != 0 {
        ItemCategory::Body
    } else if slots & HEAD != 0 {
        ItemCategory::Head
    } else if slots & ARMS != 0 {
        ItemCategory::Arms
    } else if slots & LEGS != 0 {
        ItemCategory::Legs
    } else if slots & HANDS != 0 {
        ItemCategory::Hands
    } else if slots & JEWELLERY != 0 {
        ItemCategory::Jewellery
    } else if slots & SHIELD != 0 {
        ItemCategory::Shield
    } else {
        ItemCategory::OtherWorn
    }
}

/// The material a keyword names, such as `Steel` for
/// `WeapMaterialSteel` and `Chitin` for `DLC2ArmorMaterialChitinLight`.
fn material(keyword: &str) -> Option<String> {
    let lower = keyword.to_ascii_lowercase();
    if lower.starts_with("ma_") {
        return fallout4_material(keyword);
    }
    let at = lower
        .find("weapmaterial")
        .map(|at| at + "weapmaterial".len())
        .or_else(|| {
            lower
                .find("armormaterial")
                .map(|at| at + "armormaterial".len())
        })?;
    let rest = keyword.get(at..)?;
    let rest = rest
        .strip_suffix("Heavy")
        .or_else(|| rest.strip_suffix("Light"))
        .unwrap_or(rest);
    (!rest.is_empty()).then(|| rest.to_owned())
}

/// The kind of armour one of Fallout 4's keywords for its modifications
/// names: `Combat` for `ma_armor_Combat_Arm`, `T60` for `ma_T60_Torso`,
/// `Raider` for `ma_armor_RaiderMod_Leg` and `ma_PA_Raider_Helmet`. Linings,
/// paints and the other keywords that fit any armour name none.
fn fallout4_material(keyword: &str) -> Option<String> {
    let rest = keyword.get("ma_".len()..)?;
    if rest
        .split('_')
        .any(|part| part.eq_ignore_ascii_case("lining"))
    {
        return None;
    }
    let mut parts = rest.split('_');
    let first = parts.next()?;
    let kind = match first.to_ascii_lowercase().as_str() {
        "armor" | "pa" => parts.next()?,
        "t45" | "t51" | "t60" | "x01" => first,
        _ => return None,
    };
    // A piece follows the kind: `ma_PA_T45` names a frame, not a kind.
    parts.next()?;
    let kind = kind.strip_suffix("Mod").unwrap_or(kind);
    let generic = [
        "lining",
        "universal",
        "paint",
        "material",
        "misc",
        "arm",
        "leg",
        "torso",
        "helmet",
    ];
    (!kind.is_empty() && !generic.contains(&kind.to_ascii_lowercase().as_str()))
        .then(|| kind.to_owned())
}

/// The name players see: `FULL`, looked up in the string table for a
/// localized plugin.
fn name(record: &Record, localized: bool, names: &HashMap<u32, String>) -> Option<String> {
    records::full_name(record, localized, names)
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

/// A model field as a path below the data directory.
fn model(record: &Record, signature: Signature) -> Option<String> {
    let model = record.first(&signature).map(plugin::zstring)?;
    let model = model.replace('\\', "/").to_ascii_lowercase();
    let model = model.trim_start_matches('/');
    if model.is_empty() {
        return None;
    }
    Some(if model.starts_with("meshes/") {
        model.to_owned()
    } else {
        format!("meshes/{model}")
    })
}

/// The catalogue of the setup of `game` in `game_dir`. `local_dir` holds
/// `plugins.txt`, as for [`ingest::read_setup`]. Names are taken from the
/// string tables in `language` (as the game names it, such as `german`)
/// where the setup has them, else in English. Plugins that cannot be read
/// are left out. Only reads.
pub fn read_catalogue(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    language: &str,
) -> Catalogue {
    if !game.has_catalogue() {
        return Catalogue::default();
    }
    let tables = LodLibrary::with_prefixes(game, game_dir, local_dir, &["strings/".to_owned()]);
    let shipped = crate::locate::shipped_plugins(game, game_dir);
    let mut collector = Collector::new(game);
    for (name, path) in ingest::active_plugin_files(game, game_dir, local_dir) {
        let Ok(file) = File::open(&path) else {
            continue;
        };
        let Ok(plugin) = plugin::read(&mut BufReader::new(file), &RECORDS) else {
            continue;
        };
        let names = if plugin.header.is_localized {
            string_table(&tables, &name, language)
        } else {
            HashMap::new()
        };
        collector.add(&name, &plugin, &names);
    }
    collector.finish(&|plugin| ingest::origin(game, &shipped, plugin))
}

/// The names of the string table of `plugin_name`, in `language` if the
/// setup has it, else in English, else in any language there is.
pub fn string_table(
    tables: &LodLibrary,
    plugin_name: &str,
    language: &str,
) -> HashMap<u32, String> {
    let stem = plugin_name
        .rsplit_once('.')
        .map_or(plugin_name, |(stem, _)| stem)
        .to_lowercase();
    let path = |language: &str| format!("strings/{stem}_{language}.strings");
    let prefix = format!("strings/{stem}_");
    let other = tables
        .paths()
        .filter(|path| path.starts_with(&prefix) && path.ends_with(".strings"))
        .min()
        .map(str::to_owned);
    // Skyrim names its tables by the language (`skyrim_german`), Fallout 4
    // by its code (`fallout4_de`).
    let code = match language {
        "german" => "de",
        "french" => "fr",
        "spanish" => "es",
        "italian" => "it",
        "polish" => "pl",
        "russian" => "ru",
        "japanese" => "ja",
        _ => "en",
    };
    [path(language), path(code), path("english"), path("en")]
        .into_iter()
        .chain(other)
        .find_map(|path| tables.read(&path))
        .map_or_default(|data| strings::parse_strings(&data))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use proptest::prelude::*;

    use super::*;

    fn form_ids(ids: &[u32]) -> Vec<u8> {
        ids.iter().flat_map(|id| id.to_le_bytes()).collect()
    }

    fn keyword(form_id: u32, editor_id: &str) -> RecordSpec {
        RecordSpec::new(*b"KYWD", form_id).with(*b"EDID", fixture::zstring(editor_id))
    }

    fn weapon(form_id: u32, name: &str, model: &str, keywords: &[u32]) -> RecordSpec {
        let mut dnam = vec![1, 0, 0, 0];
        dnam.extend([0; 12]);
        RecordSpec::new(*b"WEAP", form_id)
            .with(*b"EDID", fixture::zstring(&name.replace(' ', "")))
            .with(*b"FULL", fixture::zstring(name))
            .with(*b"MODL", fixture::zstring(model))
            .with(
                *b"KSIZ",
                u32::try_from(keywords.len())
                    .unwrap()
                    .to_le_bytes()
                    .to_vec(),
            )
            .with(*b"KWDA", form_ids(keywords))
            .with(*b"DNAM", dnam)
    }

    fn armour(form_id: u32, name: &str, slots: u32, addon: u32) -> RecordSpec {
        RecordSpec::new(*b"ARMO", form_id)
            .with(*b"FULL", fixture::zstring(name))
            .with(*b"MOD2", fixture::zstring(&format!("Armor\\{name}_go.nif")))
            .with(*b"BOD2", [slots.to_le_bytes(), 0u32.to_le_bytes()].concat())
            .with(*b"MODL", addon.to_le_bytes().to_vec())
    }

    fn addon(form_id: u32, model: &str) -> RecordSpec {
        RecordSpec::new(*b"ARMA", form_id).with(*b"MOD2", fixture::zstring(model))
    }

    fn read(bytes: &[u8]) -> Plugin {
        plugin::read(&mut Cursor::new(bytes), &RECORDS).unwrap()
    }

    fn game_plugin() -> Vec<u8> {
        fixture::plugin(
            &[],
            0,
            &[
                (
                    *b"KYWD",
                    vec![
                        keyword(0x10, "WeapTypeSword"),
                        keyword(0x11, "WeapMaterialSteel"),
                        keyword(0x12, "ArmorMaterialSteelHeavy"),
                    ],
                ),
                (
                    *b"WEAP",
                    vec![weapon(
                        0x100,
                        "Steel Sword",
                        "Weapons\\Steel\\SteelSword.nif",
                        &[0x10, 0x11],
                    )],
                ),
                (
                    *b"ARMA",
                    vec![
                        addon(0x200, "Armor\\Steel\\HelmetM.nif"),
                        addon(0x201, "Armor\\Steel\\BootsM.nif"),
                    ],
                ),
                (
                    *b"ARMO",
                    vec![
                        armour(0x300, "Steel Helmet", 1 << 1, 0x200)
                            .with(*b"KWDA", form_ids(&[0x12])),
                        armour(0x301, "Steel Boots", 1 << 7, 0x201),
                        {
                            let mut skin = armour(0x302, "Skin", 1 << 2, 0x200);
                            skin.flags |= FLAG_ARMOUR_NOT_PLAYABLE;
                            skin
                        },
                    ],
                ),
                (
                    *b"LVLI",
                    vec![RecordSpec::new(*b"LVLI", 0x400).with(*b"LVLO", {
                        let mut entry = vec![1, 0, 0, 0];
                        entry.extend(0x100u32.to_le_bytes());
                        entry.extend([1, 0, 0, 0]);
                        entry
                    })],
                ),
                (
                    *b"OTFT",
                    vec![
                        RecordSpec::new(*b"OTFT", 0x500)
                            .with(*b"EDID", fixture::zstring("GuardOutfit"))
                            .with(*b"INAM", form_ids(&[0x300, 0x301])),
                    ],
                ),
                (
                    *b"NPC_",
                    vec![
                        RecordSpec::new(*b"NPC_", 0x600)
                            .with(*b"FULL", fixture::zstring("Guard"))
                            .with(*b"ACBS", [vec![0x20], vec![0; 23]].concat())
                            .with(*b"DOFT", 0x500u32.to_le_bytes().to_vec())
                            .with(
                                *b"CNTO",
                                [0x400u32.to_le_bytes(), 1u32.to_le_bytes()].concat(),
                            ),
                    ],
                ),
            ],
        )
    }

    fn collect(plugins: &[(&str, Vec<u8>)]) -> Catalogue {
        let mut collector = Collector::new(Game::SkyrimSe);
        for (name, bytes) in plugins {
            collector.add(name, &read(bytes), &HashMap::new());
        }
        collector.finish(&|plugin| {
            if plugin == "skyrim.esm" {
                WorldOrigin::Game
            } else {
                WorldOrigin::Mod
            }
        })
    }

    fn item<'a>(catalogue: &'a Catalogue, id: &str) -> &'a CatalogueItem {
        catalogue.items.iter().find(|item| item.id == id).unwrap()
    }

    #[test]
    fn weapons_and_armour_are_sorted_by_what_they_are_and_made_of() {
        let catalogue = collect(&[("Skyrim.esm", game_plugin())]);
        let sword = item(&catalogue, "skyrim.esm/00000100");
        assert_eq!(sword.category, ItemCategory::Sword);
        assert_eq!(sword.material.as_deref(), Some("Steel"));
        assert_eq!(sword.name.as_deref(), Some("Steel Sword"));
        assert_eq!(sword.models, ["meshes/weapons/steel/steelsword.nif"]);

        let helmet = item(&catalogue, "skyrim.esm/00000300");
        assert_eq!(helmet.category, ItemCategory::Head);
        assert_eq!(helmet.material.as_deref(), Some("Steel"));
        assert_eq!(
            helmet.models,
            [
                "meshes/armor/steel helmet_go.nif",
                "meshes/armor/steel/helmetm.nif"
            ]
        );
        assert_eq!(
            item(&catalogue, "skyrim.esm/00000301").category,
            ItemCategory::Feet
        );
    }

    #[test]
    fn what_players_cannot_wear_is_left_out() {
        let catalogue = collect(&[("Skyrim.esm", game_plugin())]);
        assert!(
            catalogue
                .items
                .iter()
                .all(|item| item.id != "skyrim.esm/00000302")
        );
    }

    #[test]
    fn an_outfit_lists_what_is_worn_and_what_its_wearers_carry() {
        let catalogue = collect(&[("Skyrim.esm", game_plugin())]);
        let outfit = &catalogue.outfits[0];
        assert_eq!(outfit.editor_id.as_deref(), Some("GuardOutfit"));
        assert_eq!(outfit.worn, ["skyrim.esm/00000300", "skyrim.esm/00000301"]);
        assert_eq!(outfit.weapons, ["skyrim.esm/00000100"]);
        assert_eq!(outfit.characters, 1);

        let guard = item(&catalogue, "skyrim.esm/00000600");
        assert_eq!(guard.category, ItemCategory::Character);
        assert_eq!(guard.outfit.as_deref(), Some("skyrim.esm/00000500"));
        assert_eq!(
            guard.models,
            ["meshes/actors/character/facegendata/facegeom/skyrim.esm/00000600.nif"]
        );
    }

    #[test]
    fn a_later_plugin_changes_a_thing_and_is_named() {
        let changed = fixture::plugin(
            &["Skyrim.esm"],
            0,
            &[(
                *b"WEAP",
                vec![weapon(0x100, "Steel Sword", "NewSteel\\Sword.nif", &[0x10])],
            )],
        );
        let catalogue = collect(&[
            ("Skyrim.esm", game_plugin()),
            ("Better Swords.esp", changed),
        ]);
        let sword = item(&catalogue, "skyrim.esm/00000100");
        assert_eq!(sword.changed_by, ["better swords.esp"]);
        assert_eq!(sword.models, ["meshes/newsteel/sword.nif"]);
        assert_eq!(sword.material, None);
        assert_eq!(catalogue.sources[1].origin, WorldOrigin::Mod);
    }

    #[test]
    fn variants_are_counted_at_what_they_are_made_from() {
        let enchanted = fixture::plugin(
            &["Skyrim.esm"],
            0,
            &[
                (
                    *b"WEAP",
                    vec![
                        weapon(
                            0x0100_0800,
                            "Steel Sword of Fire",
                            "Weapons\\Steel\\SteelSword.nif",
                            &[],
                        )
                        .with(*b"CNAM", 0x100u32.to_le_bytes().to_vec()),
                    ],
                ),
                (
                    *b"OTFT",
                    vec![
                        RecordSpec::new(*b"OTFT", 0x0100_0801)
                            .with(*b"INAM", form_ids(&[0x0100_0800])),
                    ],
                ),
            ],
        );
        let catalogue = collect(&[("Skyrim.esm", game_plugin()), ("Fire.esp", enchanted)]);
        assert!(
            catalogue
                .items
                .iter()
                .all(|item| item.id != "fire.esp/00000800")
        );
        assert_eq!(item(&catalogue, "skyrim.esm/00000100").variants, 1);
        let outfit = catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.id == "fire.esp/00000801")
            .unwrap();
        assert_eq!(outfit.worn, ["skyrim.esm/00000100"]);
    }

    #[test]
    fn characters_without_a_face_of_their_own_or_many_of_a_kind_are_left_out() {
        let generic = fixture::plugin(
            &["Skyrim.esm"],
            0,
            &[(
                *b"NPC_",
                vec![
                    RecordSpec::new(*b"NPC_", 0x0100_0900)
                        .with(*b"FULL", fixture::zstring("Bandit"))
                        .with(*b"ACBS", vec![0; 24]),
                ],
            )],
        );
        let catalogue = collect(&[("Skyrim.esm", game_plugin()), ("Bandits.esp", generic)]);
        assert!(
            catalogue
                .items
                .iter()
                .all(|item| item.id != "bandits.esp/00000900")
        );
    }

    #[test]
    fn materials_are_named_without_prefix_and_weight() {
        assert_eq!(material("WeapMaterialSteel").as_deref(), Some("Steel"));
        assert_eq!(
            material("DLC2ArmorMaterialChitinLight").as_deref(),
            Some("Chitin")
        );
        assert_eq!(
            material("ArmorMaterialSteelPlate").as_deref(),
            Some("SteelPlate")
        );
        assert_eq!(material("ArmorHelmet"), None);
    }

    /// Slots of Fallout 4 as bits, from their numbers (30 and up).
    fn slots(numbers: &[u32]) -> u32 {
        numbers.iter().map(|number| 1 << (number - 30)).sum()
    }

    fn fallout4_plugin() -> Vec<u8> {
        let fallout4_weapon = |form_id: u32, model: &str, keywords: &[u32]| {
            RecordSpec::new(*b"WEAP", form_id)
                .with(*b"MODL", fixture::zstring(model))
                .with(*b"KWDA", form_ids(keywords))
                // Fallout 4's data starts with the ammunition.
                .with(*b"DNAM", [0x0700_0000u32.to_le_bytes(), [0; 4]].concat())
        };
        let fallout4_armour = |form_id: u32, slots: u32, addon: u32, keywords: &[u32]| {
            RecordSpec::new(*b"ARMO", form_id)
                .with(*b"MOD2", fixture::zstring("Armor\\Piece_go.nif"))
                .with(*b"BOD2", slots.to_le_bytes().to_vec())
                .with(*b"KWDA", form_ids(keywords))
                .with(*b"INDX", 0u16.to_le_bytes().to_vec())
                .with(*b"MODL", addon.to_le_bytes().to_vec())
        };
        fixture::plugin(
            &[],
            0,
            &[
                (
                    *b"KYWD",
                    vec![
                        keyword(0x10, "WeaponTypeBallistic"),
                        keyword(0x11, "WeaponTypePistol"),
                        keyword(0x12, "WeaponTypeHeavyGun"),
                        keyword(0x13, "WeaponTypeMelee1H"),
                        keyword(0x14, "WeaponTypeGrenade"),
                        keyword(0x15, "WeaponTypeExplosive"),
                        keyword(0x16, "ma_armor_Combat_Arm"),
                        keyword(0x17, "ma_armor_Lining_Leather_Limb"),
                    ],
                ),
                (
                    *b"WEAP",
                    vec![
                        fallout4_weapon(0x100, "Weapons\\10mmPistol\\10mm.nif", &[0x10, 0x11])
                            // An object template's name comes before the
                            // weapon's own here.
                            .with(*b"OBTE", 1u32.to_le_bytes().to_vec())
                            .with(*b"FULL", fixture::zstring("Default"))
                            .with(*b"STOP", Vec::new())
                            .with(*b"FULL", fixture::zstring("10mm Pistol")),
                        fallout4_weapon(0x101, "Weapons\\Minigun\\Minigun.nif", &[0x10, 0x12]),
                        fallout4_weapon(0x102, "Weapons\\Knife\\Knife.nif", &[0x13]),
                        fallout4_weapon(0x103, "Weapons\\Grenade\\Frag.nif", &[0x14, 0x15]),
                        {
                            let mut claws = fallout4_weapon(0x104, "Weapons\\Claws.nif", &[0x13]);
                            claws.flags |= FLAG_FALLOUT4_NOT_PLAYABLE;
                            claws
                        },
                    ],
                ),
                (
                    *b"ARMA",
                    vec![
                        addon(0x200, "Armor\\Combat\\ArmLeft.nif"),
                        addon(0x201, "Armor\\Hat.nif"),
                        addon(0x202, "Clothes\\Suit.nif"),
                    ],
                ),
                (
                    *b"ARMO",
                    vec![
                        fallout4_armour(0x300, slots(&[42]), 0x200, &[0x17, 0x16]),
                        fallout4_armour(0x301, slots(&[30, 46]), 0x201, &[]),
                        fallout4_armour(0x302, slots(&[33, 41, 42, 43, 44, 45]), 0x202, &[]),
                        fallout4_armour(0x303, slots(&[45]), 0x200, &[]),
                    ],
                ),
                (
                    *b"AMMO",
                    vec![
                        RecordSpec::new(*b"AMMO", 0x400)
                            .with(*b"MODL", fixture::zstring("Ammo\\10mm.nif")),
                        {
                            let mut turret = RecordSpec::new(*b"AMMO", 0x401)
                                .with(*b"MODL", fixture::zstring("Ammo\\Turret.nif"));
                            turret.flags |= FLAG_FALLOUT4_NOT_PLAYABLE;
                            turret
                        },
                    ],
                ),
            ],
        )
    }

    fn collect_fallout4() -> Catalogue {
        let mut collector = Collector::new(Game::Fallout4);
        collector.add("Fallout4.esm", &read(&fallout4_plugin()), &HashMap::new());
        collector.finish(&|_| WorldOrigin::Game)
    }

    #[test]
    fn fallout_4_weapons_are_sorted_by_their_keywords() {
        let catalogue = collect_fallout4();
        let category = |id: &str| item(&catalogue, id).category;
        assert_eq!(category("fallout4.esm/00000100"), ItemCategory::Pistol);
        assert_eq!(category("fallout4.esm/00000101"), ItemCategory::HeavyWeapon);
        assert_eq!(category("fallout4.esm/00000102"), ItemCategory::MeleeWeapon);
        assert_eq!(category("fallout4.esm/00000103"), ItemCategory::Explosive);
        assert_eq!(
            item(&catalogue, "fallout4.esm/00000100").name.as_deref(),
            Some("10mm Pistol")
        );
    }

    #[test]
    fn fallout_4_leaves_out_what_its_records_flag_as_not_playable() {
        let catalogue = collect_fallout4();
        let ids: Vec<&str> = catalogue
            .items
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        assert!(!ids.contains(&"fallout4.esm/00000104"), "{ids:?}");
        assert!(!ids.contains(&"fallout4.esm/00000401"), "{ids:?}");
        assert_eq!(
            item(&catalogue, "fallout4.esm/00000400").category,
            ItemCategory::Ammunition
        );
    }

    #[test]
    fn fallout_4_armour_is_sorted_by_its_body_slots() {
        let catalogue = collect_fallout4();
        let arm = item(&catalogue, "fallout4.esm/00000300");
        assert_eq!(arm.category, ItemCategory::Arms);
        assert_eq!(arm.material.as_deref(), Some("Combat"));
        assert_eq!(
            arm.models,
            [
                "meshes/armor/piece_go.nif",
                "meshes/armor/combat/armleft.nif"
            ]
        );
        assert_eq!(
            item(&catalogue, "fallout4.esm/00000301").category,
            ItemCategory::Head
        );
        assert_eq!(
            item(&catalogue, "fallout4.esm/00000302").category,
            ItemCategory::Body
        );
        assert_eq!(
            item(&catalogue, "fallout4.esm/00000303").category,
            ItemCategory::Legs
        );
    }

    #[test]
    fn fallout_4_materials_come_from_the_keywords_of_its_modifications() {
        assert_eq!(material("ma_armor_Combat_Arm").as_deref(), Some("Combat"));
        assert_eq!(
            material("ma_armor_RaiderMod_Leg").as_deref(),
            Some("Raider")
        );
        assert_eq!(material("ma_T60_Torso").as_deref(), Some("T60"));
        assert_eq!(material("ma_PA_Raider_Helmet").as_deref(), Some("Raider"));
        assert_eq!(material("ma_PA_T45"), None);
        assert_eq!(material("ma_PA_Arm"), None);
        assert_eq!(material("ma_armor_Lining_Leather_Limb"), None);
        assert_eq!(material("ma_armor_RaiderModl_Lining_Torso"), None);
        assert_eq!(material("ma_VaultSuit"), None);
    }

    #[test]
    fn worn_things_are_sorted_by_body_slot() {
        assert_eq!(armour_category(1 << 2 | 1 << 3), ItemCategory::Body);
        assert_eq!(armour_category(1 << 12), ItemCategory::Head);
        assert_eq!(armour_category(1 << 9), ItemCategory::Shield);
        assert_eq!(armour_category(1 << 6), ItemCategory::Jewellery);
        assert_eq!(armour_category(0), ItemCategory::OtherWorn);
    }

    proptest! {
        #[test]
        fn any_record_content_never_panics(
            fields in proptest::collection::vec(
                (prop::sample::select(vec![*b"FULL", *b"MODL", *b"MOD2", *b"KWDA", *b"DNAM",
                    *b"BOD2", *b"CNTO", *b"LVLO", *b"INAM", *b"ACBS", *b"DOFT", *b"TNAM",
                    *b"OBTE", *b"STOP"]),
                 proptest::collection::vec(any::<u8>(), 0..24)),
                0..8),
            signature in prop::sample::select(RECORDS.to_vec()),
            flags in any::<u32>(),
        ) {
            let mut spec = RecordSpec::new(signature, 0x800);
            spec.flags = flags & !FLAG_DELETED;
            for (field, data) in fields {
                spec = spec.with(field, data);
            }
            let bytes = fixture::plugin(&[], 0, &[(signature, vec![spec])]);
            let _ = collect(&[("Skyrim.esm", game_plugin()), ("Any.esp", bytes.clone())]);
            let mut collector = Collector::new(Game::Fallout4);
            collector.add("Fallout4.esm", &read(&fallout4_plugin()), &HashMap::new());
            collector.add("Any.esp", &read(&bytes), &HashMap::new());
            let _ = collector.finish(&|_| WorldOrigin::Mod);
        }

        #[test]
        fn any_keyword_names_a_material_without_panicking(keyword in ".*") {
            let _ = material(&keyword);
        }
    }
}
