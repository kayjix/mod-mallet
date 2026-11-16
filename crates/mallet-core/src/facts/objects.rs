//! Objects placed in the world, and what they are.
//!
//! A plugin places an object by writing a reference: which base object
//! (a house, a tree, a chest), where, turned how far and how large. The
//! base object brings its size, the box that encloses its model. That is
//! enough to tell whether two objects stand in the same spot.

use super::RecordId;

/// The placed objects of a setup that mods put there, as the game will
/// show them, and the base objects and places they refer to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObjectFacts {
    /// Objects placed by mods, in the version the game uses. Objects of the
    /// game itself are included only where a mod moved, turned or resized
    /// them. Objects that the winning version deletes or switches off are
    /// not included.
    pub placed: Vec<PlacedObject>,
    /// The base objects the placed objects refer to, in the version the
    /// game uses.
    pub bases: Vec<BaseObject>,
    /// The internal names of the cells and worldspaces the objects are in,
    /// where they have one.
    pub place_names: Vec<PlaceName>,
}

/// Where an object stands.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Place {
    /// In a closed space, such as a house or a cave: one cell.
    Interior {
        /// The cell.
        cell: RecordId,
    },
    /// Outside, in a worldspace. Coordinates are those of the whole
    /// worldspace, so objects near a cell border can be compared.
    Exterior {
        /// The worldspace.
        worldspace: RecordId,
    },
}

/// An object a plugin placed in the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedObject {
    /// The reference itself. Its plugin is the one that placed it first.
    pub id: RecordId,
    /// The plugin whose version of the reference the game uses, in the
    /// spelling of the load order.
    pub plugin: String,
    /// What is placed.
    pub base: RecordId,
    /// Where.
    pub place: Place,
    /// The position, in game units.
    pub position: [i32; 3],
    /// The rotation about the x, y and z axes, in thousandths of a radian.
    pub rotation: [i32; 3],
    /// The size relative to the base object, in thousandths.
    pub scale: i32,
    /// Whether the object is persistent: quests and scripts may refer to
    /// it wherever the player is.
    pub persistent: bool,
    /// Whether the object is a large reference, which the worldspace's
    /// distant view is built around.
    pub large: bool,
    /// For an object of the game itself that a mod moved, turned or
    /// resized: by which plugin, and from where. The object then counts as
    /// that plugin's. `None` for the objects mods add.
    pub moved: Option<Moved>,
}

impl PlacedObject {
    /// The plugin the object counts as, in lower case: the one that placed
    /// it, or, for an object of the game, the one that moved it.
    pub fn owner(&self) -> &str {
        self.moved
            .as_ref()
            .map_or(&self.id.plugin, |moved| &moved.by)
    }
}

/// How a mod changed where an object of the game stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    /// The plugin that moved it, in lower case.
    pub by: String,
    /// Where the game has it, or `None` if the game switches it off and
    /// the mod brings it in.
    pub from: Option<Pose>,
}

/// Where and how an object stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pose {
    /// The position, in game units.
    pub position: [i32; 3],
    /// The rotation about the x, y and z axes, in thousandths of a radian.
    pub rotation: [i32; 3],
    /// The size relative to the base object, in thousandths.
    pub scale: i32,
}

/// The kinds of base objects whose placement is compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseKind {
    /// A fixed object: a building, a wall, a rock.
    Static,
    /// A fixed object that moves in place, such as a waterwheel.
    MovableStatic,
    /// A group of fixed objects placed as one.
    StaticCollection,
    /// A tree or a bush.
    Tree,
    /// Something to sit or lie on.
    Furniture,
    /// A chest, a barrel, a wardrobe.
    Container,
    /// A door.
    Door,
    /// Something to interact with, such as a lever.
    Activator,
}

/// A base object: what can be placed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseObject {
    /// The record.
    pub id: RecordId,
    /// Its internal name, if it has one.
    pub editor_id: Option<String>,
    /// The name players see, if the plugin keeps it as text (localized
    /// plugins keep it in separate string files, which are not read).
    pub name: Option<String>,
    /// Its model, below the data directory (`meshes/...`), lower case.
    pub model: Option<String>,
    /// What kind of object it is.
    pub kind: BaseKind,
    /// Whether the object is something one can see and bump into. Markers,
    /// trigger zones, light effects and small plants are not.
    pub solid: bool,
    /// Whether the object is a piece of landscape: a rock, a cliff, a tree,
    /// a snow drift. Such pieces are set into each other on purpose.
    pub natural: bool,
    /// The box that encloses its model, in game units, before it is
    /// turned and scaled.
    pub bounds: Bounds,
}

/// A box given by two opposite corners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// The corner with the smallest x, y and z.
    pub min: [i32; 3],
    /// The corner with the largest x, y and z.
    pub max: [i32; 3],
}

/// The internal name of a cell or worldspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceName {
    /// The cell or worldspace.
    pub id: RecordId,
    /// Its internal name.
    pub editor_id: String,
}
