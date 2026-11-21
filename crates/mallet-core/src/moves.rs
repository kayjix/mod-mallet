//! Moving placed objects a little, instead of hiding them (ADR 0060).
//!
//! Where two mods build into each other, both structures can often stay if
//! one of them stands a little to one side. A move names an object, how far
//! it goes along each axis and how far it turns about the upright axis. A
//! plugin of overrides that Mod Mallet writes carries the moves into the
//! game; removing it puts every object back.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectRef;

/// The name of the plugin that carries the moves, without its extension.
/// The mod the mod manager installs is named the same.
pub const MOVES_NAME: &str = "Mod Mallet - Moves";

/// One placed object moved: by how much, and where it was found.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectMove {
    /// The object.
    pub object: ObjectRef,
    /// The interior cell it stands in, by its editor ID; `None` outside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<String>,
    /// Where it stood when it was moved, in whole game units. Only a hint
    /// for finding it: the plugin starts from where the game has it.
    pub from: [i32; 3],
    /// How far it goes east, north and up, in game units (about 1.4 cm
    /// each).
    pub offset: [i32; 3],
    /// How far it turns about the upright axis through its own origin, in
    /// degrees, the way the game counts (clockwise seen from above).
    pub turn: i32,
    /// For a door: where its teleport is to lead instead, when stacked
    /// doors are merged into one (ADR 0067).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teleport: Option<DoorTarget>,
}

/// Where a door is to lead: the door on the other side, and where the
/// player arrives there (ADR 0067).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorTarget {
    /// The door on the other side.
    pub door: ObjectRef,
    /// Where the player arrives, in whole game units; `None` keeps the
    /// point the door has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrival: Option<[i32; 3]>,
    /// Which way the player faces there, in thousandths of a radian;
    /// `None` keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub facing: Option<[i32; 3]>,
}

impl ObjectMove {
    /// Whether it changes nothing.
    pub fn is_still(&self) -> bool {
        self.offset == [0, 0, 0] && self.turn % 360 == 0 && self.teleport.is_none()
    }
}

/// The moves to carry into the game, as one plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Moves {
    /// The moves, one per object.
    pub moves: Vec<ObjectMove>,
}

/// Why an object could not be moved by the plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum MoveRefusal {
    /// No active plugin places it where it was said to be.
    NotFound,
    /// The version the game uses deletes it or switches it off.
    Gone,
    /// The object, its cell or its worldspace has a field Mod Mallet does
    /// not know how to copy; copying it blindly could break what it
    /// refers to.
    UnknownField {
        /// The record type and the field, such as `REFR XCVL`.
        field: String,
    },
    /// A name the game keeps in a string table could not be found there.
    NameNotFound,
    /// The game is not one whose plugins Mod Mallet can write.
    UnsupportedGame,
}

/// An object that was not moved, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefusedMove {
    /// The object.
    pub object: ObjectRef,
    /// Why.
    pub reason: MoveRefusal,
}

/// The plugin of moves as written into Mod Mallet's own folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreatedMoves {
    /// The archive that holds it, ready for the mod manager.
    pub path: String,
    /// How many objects it moves.
    pub moved: u32,
    /// The plugins it names as masters: those whose versions it copies
    /// and those they refer to. It loads after them.
    pub masters: Vec<String>,
    /// Whether it is flagged as light.
    pub light: bool,
    /// The objects it could not move, and why.
    pub refused: Vec<RefusedMove>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_move_serializes_in_camel_case_and_a_full_turn_is_still() {
        let still = ObjectMove {
            object: ObjectRef {
                plugin: "Town.esp".to_owned(),
                local_id: 0x812,
            },
            cell: None,
            from: [1, 2, 3],
            offset: [0, 0, 0],
            turn: 360,
            teleport: None,
        };
        assert!(still.is_still());
        let json = serde_json::to_value(&still).unwrap();
        assert_eq!(json["object"]["localId"], 0x812);
        assert!(json.get("cell").is_none());
        let refusal = serde_json::to_value(MoveRefusal::UnknownField {
            field: "REFR XCVL".to_owned(),
        })
        .unwrap();
        assert_eq!(refusal["type"], "unknown-field");
    }
}
