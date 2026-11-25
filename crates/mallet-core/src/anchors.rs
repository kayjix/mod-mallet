//! Why the game relies on a placed object, and whether it can be hidden
//! all the same (ADR 0046).
//!
//! Hiding swaps an object's base for an invisible marker (ADR 0025): the
//! object stays in place and keeps its links, but loses its look, its
//! collision and what its base does (a door's way through, a chest's
//! contents). The reasons say what points at the object; the verdict says
//! what hiding it would break.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectRef;

/// Why the game relies on one placed object, and what that means for
/// hiding it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ObjectAnchors {
    /// The object asked about.
    pub object: ObjectRef,
    /// Whether it can be hidden.
    pub verdict: AnchorVerdict,
    /// Everything found that ties it to the game, the most binding first.
    /// Empty for an object nothing relies on.
    pub reasons: Vec<Anchor>,
}

/// Whether an object can be hidden.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AnchorVerdict {
    /// Nothing relies on it.
    Free,
    /// It can be hidden, but something still uses it or it is seen from
    /// afar; the reasons say what.
    Careful,
    /// Hiding it would break the game: a door with a way through, or an
    /// object a quest names.
    Locked,
}

impl AnchorVerdict {
    /// The verdict the rules of ADR 0046 give for `reasons`: reasons whose
    /// other side is gone do not count.
    pub fn of(reasons: &[Anchor]) -> Self {
        reasons
            .iter()
            .filter(|reason| {
                reason
                    .other
                    .as_ref()
                    .is_none_or(|other| other.state != OtherState::Gone)
            })
            .map(|reason| reason.kind.verdict())
            .max()
            .unwrap_or(Self::Free)
    }
}

/// One thing that ties an object to the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    /// What kind of tie it is.
    pub kind: AnchorKind,
    /// The other record involved, for ties to another one.
    pub other: Option<AnchorOther>,
    /// The keyword of a linked reference, or the role the object has in a
    /// location, by internal name.
    pub keyword: Option<String>,
}

/// The kinds of ties, as seen from the object asked about.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum AnchorKind {
    /// It is a door that leads somewhere: the other is the door on the
    /// other side.
    TeleportDoor,
    /// A quest names it as one of its parts (a forced alias).
    QuestAlias,
    /// The worldspace counts it among its large objects, which are drawn
    /// from afar. Hidden, it may still show from a distance until the
    /// distant view is generated again.
    LargeReference,
    /// Using it activates the other.
    ActivateParent,
    /// It switches the other on and off: the other is shown only while it
    /// is.
    EnableParent,
    /// The other links to it, for example as a place to walk to or a part
    /// that belongs to it.
    LinkedRef,
    /// It has a role in a location, such as a boss's chest, that quests
    /// look for. The other is the location, if one lists it.
    LocationRole,
    /// A character's routine (an AI package) uses it.
    Package,
    /// A script of the other refers to it.
    Script,
    /// It is part of the distant view, which still shows it from afar.
    VisibleWhenDistant,
    /// The game keeps it loaded at all times, which it does for objects
    /// something refers to.
    Persistent,
}

impl AnchorKind {
    /// The verdict a tie of this kind gives on its own, while its other
    /// side is not gone.
    pub fn verdict(self) -> AnchorVerdict {
        match self {
            Self::TeleportDoor | Self::QuestAlias => AnchorVerdict::Locked,
            Self::LargeReference
            | Self::ActivateParent
            | Self::EnableParent
            | Self::LinkedRef
            | Self::LocationRole
            | Self::Package
            | Self::Script
            | Self::VisibleWhenDistant
            | Self::Persistent => AnchorVerdict::Careful,
        }
    }
}

/// The other record of a tie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnchorOther {
    /// The plugin that defined it first, by file name in lower case.
    pub plugin: String,
    /// Its number in that plugin.
    pub local_id: u32,
    /// The record's type, such as `REFR` for a placed object or `QUST`
    /// for a quest.
    pub record_type: String,
    /// Its internal name, or for a placed object without one the name of
    /// what is placed.
    pub name: Option<String>,
    /// Whether it is still in the game.
    pub state: OtherState,
}

/// Whether the other record of a tie is still in the game, in the version
/// the load order gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OtherState {
    /// It is there.
    Present,
    /// It is switched off when the game starts, and something may switch
    /// it on.
    Off,
    /// It is deleted, or switched off for good: nothing can bring it back,
    /// so the tie no longer counts.
    Gone,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tie(kind: AnchorKind, state: Option<OtherState>) -> Anchor {
        Anchor {
            kind,
            other: state.map(|state| AnchorOther {
                plugin: "a.esp".to_owned(),
                local_id: 1,
                record_type: "REFR".to_owned(),
                name: None,
                state,
            }),
            keyword: None,
        }
    }

    #[test]
    fn the_most_binding_tie_decides() {
        assert_eq!(AnchorVerdict::of(&[]), AnchorVerdict::Free);
        assert_eq!(
            AnchorVerdict::of(&[tie(AnchorKind::Persistent, None)]),
            AnchorVerdict::Careful
        );
        assert_eq!(
            AnchorVerdict::of(&[
                tie(AnchorKind::LinkedRef, Some(OtherState::Present)),
                tie(AnchorKind::TeleportDoor, Some(OtherState::Off)),
            ]),
            AnchorVerdict::Locked
        );
    }

    #[test]
    fn a_large_object_of_the_distant_view_alone_may_be_hidden_with_a_warning() {
        let reasons = [tie(AnchorKind::LargeReference, None)];
        assert_eq!(AnchorVerdict::of(&reasons), AnchorVerdict::Careful);
        // Doors and quests still lock what is also a large object.
        assert_eq!(
            AnchorVerdict::of(&[
                tie(AnchorKind::LargeReference, None),
                tie(AnchorKind::QuestAlias, Some(OtherState::Present)),
            ]),
            AnchorVerdict::Locked
        );
    }

    #[test]
    fn ties_to_what_is_gone_do_not_count() {
        assert_eq!(
            AnchorVerdict::of(&[
                tie(AnchorKind::TeleportDoor, Some(OtherState::Gone)),
                tie(AnchorKind::EnableParent, Some(OtherState::Gone)),
            ]),
            AnchorVerdict::Free
        );
    }
}
