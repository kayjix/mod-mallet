//! The things of the game one can look at (ADR 0053): weapons, armour,
//! characters and other items, with the sets they form, from the game and
//! from every mod.
//!
//! The catalogue is read on demand, not with every scan; it is what the
//! desktop's *Assets* page lists and filters.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Action, ModRef, WorldOrigin};

/// Every thing of the setup worth looking at, with the sets it forms.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Catalogue {
    /// The things, sorted by name.
    pub items: Vec<CatalogueItem>,
    /// What characters wear, as sets of things.
    pub outfits: Vec<CatalogueOutfit>,
    /// The plugins things come from, in load order.
    pub sources: Vec<CatalogueSource>,
}

/// A plugin things come from, and the mod that brings it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueSource {
    /// The plugin's file name, in lower case.
    pub plugin: String,
    /// Whether it comes with the game, the Creation Club or a mod.
    pub origin: WorldOrigin,
    /// The mod that brings it, if the mod manager says so.
    #[serde(rename = "mod", default, skip_serializing_if = "Option::is_none")]
    pub mod_ref: Option<ModRef>,
}

/// One thing: a weapon, a piece of armour, a character, an item.
///
/// Variants made from another thing, such as the enchanted swords made
/// from a plain one, look like it and are counted there, not listed.
/// Characters are listed where there is only one of them and they have a
/// face of their own; what the many others wear is listed as outfits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueItem {
    /// Stable identity: the plugin that defines the record and its number
    /// there, as `skyrim.esm/00012eb7`. It does not depend on the load
    /// order.
    pub id: String,
    /// What kind of thing it is.
    pub category: ItemCategory,
    /// The name players see, from the version the game uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The internal name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_id: Option<String>,
    /// What it is made of, from its keywords, such as `Steel` or `Daedric`.
    /// Materials of mods keep the name the mod gives them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    /// The plugin that defines it, in lower case.
    pub plugin: String,
    /// The plugins that change it, in load order; the last one's version is
    /// the one the game uses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_by: Vec<String>,
    /// Its models below the data directory (`meshes/...`), lower case: the
    /// one lying on the ground first, then the ones worn or held. For a
    /// character, its face.
    pub models: Vec<String>,
    /// How many variants are made from it, such as enchanted swords.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub variants: u32,
    /// For a character, the name of its race.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub race: Option<String>,
    /// For a character, what it wears unless told otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit: Option<String>,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde passes a reference
const fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// What a thing is, as players would sort it.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ItemCategory {
    /// A one-handed sword.
    Sword,
    /// A dagger.
    Dagger,
    /// A war axe.
    WarAxe,
    /// A mace.
    Mace,
    /// A two-handed sword.
    Greatsword,
    /// A battle axe.
    Battleaxe,
    /// A warhammer.
    Warhammer,
    /// A bow.
    Bow,
    /// A crossbow.
    Crossbow,
    /// A staff.
    Staff,
    /// Any other weapon.
    OtherWeapon,
    /// Arrows and bolts.
    Ammunition,
    /// Something worn on the head: a helmet, a hood, a circlet.
    Head,
    /// Something worn on the body: a cuirass, robes, clothes.
    Body,
    /// Something worn on the hands: gauntlets, gloves.
    Hands,
    /// Something worn on the feet: boots, shoes.
    Feet,
    /// A shield.
    Shield,
    /// A ring or a necklace.
    Jewellery,
    /// Anything else that is worn.
    OtherWorn,
    /// A character.
    Character,
    /// A book, a note or a scroll.
    Book,
    /// A potion, a poison or something to eat or drink.
    Potion,
    /// An ingredient.
    Ingredient,
    /// A key.
    Key,
    /// A soul gem.
    SoulGem,
    /// Any other thing that can be carried.
    Misc,
    /// Something to sit or lie on, or to work at.
    Furniture,
    /// A chest, a barrel, a sack.
    Container,
    /// A door.
    Door,
    /// Something to interact with, such as a lever.
    Activator,
    /// A plant or something else to harvest.
    Flora,
    /// A light.
    Light,
}

impl ItemCategory {
    /// The broad group the category belongs to.
    pub const fn group(self) -> ItemGroup {
        match self {
            Self::Sword
            | Self::Dagger
            | Self::WarAxe
            | Self::Mace
            | Self::Greatsword
            | Self::Battleaxe
            | Self::Warhammer
            | Self::Bow
            | Self::Crossbow
            | Self::Staff
            | Self::OtherWeapon
            | Self::Ammunition => ItemGroup::Weapons,
            Self::Head
            | Self::Body
            | Self::Hands
            | Self::Feet
            | Self::Shield
            | Self::Jewellery
            | Self::OtherWorn => ItemGroup::Apparel,
            Self::Character => ItemGroup::Characters,
            Self::Book
            | Self::Potion
            | Self::Ingredient
            | Self::Key
            | Self::SoulGem
            | Self::Misc => ItemGroup::Items,
            Self::Furniture
            | Self::Container
            | Self::Door
            | Self::Activator
            | Self::Flora
            | Self::Light => ItemGroup::Other,
        }
    }
}

/// The broad groups of things.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ItemGroup {
    /// Characters.
    Characters,
    /// Weapons and ammunition.
    Weapons,
    /// Armour, clothes and jewellery.
    Apparel,
    /// Things to carry.
    Items,
    /// Everything else with a model of its own.
    Other,
}

/// What a group of characters wears: a set of things.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueOutfit {
    /// Stable identity, as for [`CatalogueItem::id`].
    pub id: String,
    /// The internal name, such as `GuardWhiterunOutfit`.
    pub editor_id: Option<String>,
    /// The plugin that defines it, in lower case.
    pub plugin: String,
    /// The things worn, by identity. Where the game picks one of several,
    /// all of them are listed.
    pub worn: Vec<String>,
    /// The weapons the characters who wear it carry, by identity, the most
    /// common first.
    pub weapons: Vec<String>,
    /// How many characters wear it unless told otherwise.
    pub characters: u32,
}

/// Every version of a thing the setup has: what the game and each mod
/// make it look like (ADR 0053).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetVersions {
    /// The files the thing is made of, below the data directory: its
    /// models, then the textures they name.
    pub files: Vec<String>,
    /// The versions, the one the game shows most of first.
    pub versions: Vec<AssetVersion>,
    /// What puts back the mod manager's own order for the thing's loose
    /// files, taking back a version chosen before; `None` where no mod
    /// brings them loose.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset: Option<Action>,
}

/// One version of a thing: the copies of its files one source brings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssetVersion {
    /// Identity, for asking for the version's view: `game`, `mod:<id>`,
    /// `archive:<name>` or `loose`.
    pub id: String,
    /// Where its copies come from.
    pub source: AssetSource,
    /// The thing's files it has a copy of.
    pub files: Vec<String>,
    /// Of those, the ones whose copy the game uses.
    pub shown: Vec<String>,
    /// What makes the game show this version, where the mod manager can:
    /// for loose files of other mods in the way. `None` where nothing is in
    /// the way, or where only the order of plugins would help.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<Action>,
}

/// Where the copies of a version come from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AssetSource {
    /// The game's own archives.
    Game,
    /// A mod, from its folder in the mod manager or its archives.
    Mod {
        /// The mod.
        #[serde(rename = "mod")]
        mod_ref: ModRef,
    },
    /// An archive no mod manager knows, such as Creation Club content.
    Archive {
        /// Its file name.
        name: String,
    },
    /// Loose files in the data directory no mod manager knows.
    Loose,
}
