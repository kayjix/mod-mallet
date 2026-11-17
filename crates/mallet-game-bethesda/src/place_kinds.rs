//! What kind of place a map marker is, by the number the game gives it.
//!
//! Every game numbers the kinds of its map markers in a list of its own
//! (the first byte of `TNAM`). The tables here translate them into the
//! kinds all games share. The tables of Skyrim, Fallout 4 and Oblivion were
//! checked against the markers of the games' own plugins; those of Fallout 3
//! and New Vegas follow xEdit's definitions and have not been checked
//! against the games yet.

use mallet_core::PlaceKind;

use crate::game::{Family, Game};

/// The kind of place of a map marker of `game` with the number `number`.
/// Numbers the table does not know are [`PlaceKind::Other`].
pub fn place_kind(game: Game, number: u8) -> PlaceKind {
    match game.family() {
        Family::Skyrim => skyrim(number),
        Family::Fallout4 => fallout_4(number),
        Family::Fallout3 => fallout_3(number),
        Family::Oblivion => oblivion(number),
    }
}

fn skyrim(number: u8) -> PlaceKind {
    use PlaceKind::{
        Camp, Castle, Cave, City, Clearing, DaedricShrine, Docks, Doomstone, DragonLair,
        DwemerRuin, Farm, Fort, GiantCamp, Grove, ImperialCamp, ImperialTower, Landmark,
        Lighthouse, Mine, NordicDwelling, NordicRuin, NordicTower, OrcStronghold, Other, Pass,
        Passage, Settlement, Shack, Shipwreck, Shrine, Smelter, Stable, StormcloakCamp, Town,
        WheatMill, WoodMill,
    };
    match number {
        1 => City,
        2 | 54 => Town,
        3 | 56 => Settlement,
        4 => Cave,
        5 => Camp,
        6 => Fort,
        7 => NordicRuin,
        8 => DwemerRuin,
        9 => Shipwreck,
        10 => Grove,
        11 | 26 | 53 => Landmark,
        12 => DragonLair,
        13 => Farm,
        14 => WoodMill,
        15 => Mine,
        16 => ImperialCamp,
        17 => StormcloakCamp,
        18 | 55 => Doomstone,
        19 => WheatMill,
        20 => Smelter,
        21 => Stable,
        22 => ImperialTower,
        23 => Clearing,
        24 => Pass,
        25 => Shrine,
        27 => Lighthouse,
        28 => OrcStronghold,
        29 => GiantCamp,
        30 => Shack,
        31 => NordicTower,
        32 => NordicDwelling,
        33 => Docks,
        34 => DaedricShrine,
        // The holds' castles (odd) and capitals (even).
        35..=52 if number.is_multiple_of(2) => City,
        35..=52 | 59 => Castle,
        57 | 58 => Passage,
        _ => Other,
    }
}

fn fallout_4(number: u8) -> PlaceKind {
    use PlaceKind::{
        Building, Camp, Castle, Cave, City, Docks, Factory, Farm, Fort, Landmark, Mine, Other,
        Ruin, Settlement, Shipwreck, Shrine, Station, Town, Vault,
    };
    match number {
        0 => Cave,
        // A city; Diamond City; Goodneighbor; the Institute.
        1 | 2 | 29 | 34 => City,
        // Encampments, campers, the Railroad's hideouts.
        3 | 18 | 60 => Camp,
        // Factories, industrial domes and stacks, junkyards, shipyards.
        4 | 32 | 33 | 35 | 36 => Factory,
        // Monuments and sights: Bunker Hill, the Custom House, the drive-in,
        // Faneuil Hall, forests, graveyards, observatories, ponds,
        // radioactive areas, radio towers, the Museum of Witchcraft, the
        // swan pond, the giant synth head, satellites; and the add-ons'
        // places: the Mechanist's lair (Automatron) and Nuka-World's
        // stations, rides and attractions.
        5 | 8 | 17 | 21..=23 | 25 | 28 | 30 | 37 | 39 | 41..=43 | 47 | 48 | 62 | 65..=80 => {
            Landmark
        }
        // Subway and railway stations.
        6 | 61 => Station,
        // Military bases, bunkers, the Brotherhood, the Minutemen, the
        // Prydwen.
        7 | 50 | 52 | 57 | 59 => Fort,
        // Offices, apartments, hospitals, schools, skyscrapers, police.
        9 | 31 | 44 | 51 | 54 | 56 | 58 => Building,
        // Town and city ruins, wrecked cars and highways, sewers.
        10 | 11 | 14 | 19 | 24 => Ruin,
        // Sanctuary and the other settlements.
        12 | 13 => Settlement,
        15 => Vault,
        // Airfields and wrecked aircraft, shipwrecks, submarines.
        16 | 45 | 46 => Shipwreck,
        20 => Shrine,
        26 => Farm,
        // Filling stations and towns.
        27 | 49 => Town,
        38 | 64 => Docks,
        40 => Mine,
        // The Castle, Libertalia, the Sentinel site.
        53 | 55 | 63 => Castle,
        _ => Other,
    }
}

fn fallout_3(number: u8) -> PlaceKind {
    use PlaceKind::{
        Building, Camp, Cave, City, Factory, Fort, Landmark, Other, Ruin, Settlement, Station,
        Vault,
    };
    match number {
        1 => City,
        2 => Settlement,
        3 => Camp,
        4 | 7 => Landmark,
        5 => Cave,
        6 => Factory,
        8 => Fort,
        9 => Building,
        10..=12 => Ruin,
        13 => Station,
        14 => Vault,
        _ => Other,
    }
}

fn oblivion(number: u8) -> PlaceKind {
    use PlaceKind::{
        AyleidRuin, Camp, Cave, City, DaedricShrine, Fort, Inn, Landmark, Mine, OblivionGate,
        Other, Settlement,
    };
    match number {
        1 => Camp,
        2 => Cave,
        3 => City,
        4 => AyleidRuin,
        5 => Fort,
        6 => Mine,
        7 => Landmark,
        8 => Inn,
        9 => Settlement,
        10 => DaedricShrine,
        11 => OblivionGate,
        _ => Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skyrims_kinds_are_grouped_as_the_map_shows_them() {
        for game in [Game::SkyrimSe, Game::SkyrimVr, Game::Skyrim] {
            assert_eq!(place_kind(game, 1), PlaceKind::City);
            assert_eq!(place_kind(game, 4), PlaceKind::Cave);
            assert_eq!(place_kind(game, 35), PlaceKind::Castle);
            assert_eq!(place_kind(game, 36), PlaceKind::City);
            assert_eq!(place_kind(game, 58), PlaceKind::Passage);
            assert_eq!(place_kind(game, 0), PlaceKind::Other);
            assert_eq!(place_kind(game, 200), PlaceKind::Other);
        }
    }

    /// Checked against `Fallout4.esm`: Diamond City is 2, Sanctuary 12,
    /// Vault 111 15, Abernathy Farm 26, Walden Pond 39, the Prydwen 59.
    #[test]
    fn fallout_4s_places_have_kinds_of_their_own() {
        let kind = |number| place_kind(Game::Fallout4, number);
        assert_eq!(kind(0), PlaceKind::Cave);
        assert_eq!(kind(2), PlaceKind::City);
        assert_eq!(kind(12), PlaceKind::Settlement);
        assert_eq!(kind(15), PlaceKind::Vault);
        assert_eq!(kind(26), PlaceKind::Farm);
        assert_eq!(kind(39), PlaceKind::Landmark);
        assert_eq!(kind(59), PlaceKind::Fort);
        assert_eq!(kind(6), PlaceKind::Station);
        assert_eq!(kind(74), PlaceKind::Landmark);
        assert_eq!(kind(200), PlaceKind::Other);
        assert_eq!(place_kind(Game::Fallout4Vr, 15), PlaceKind::Vault);
    }

    /// Checked against `Oblivion.esm`: Kvatch is 3, Vilverin 4, the gates
    /// to Oblivion 11.
    #[test]
    fn oblivions_places_have_kinds_of_their_own() {
        let kind = |number| place_kind(Game::Oblivion, number);
        assert_eq!(kind(3), PlaceKind::City);
        assert_eq!(kind(4), PlaceKind::AyleidRuin);
        assert_eq!(kind(8), PlaceKind::Inn);
        assert_eq!(kind(11), PlaceKind::OblivionGate);
        assert_eq!(kind(0), PlaceKind::Other);
    }

    #[test]
    fn the_older_fallouts_share_a_table() {
        for game in [Game::Fallout3, Game::FalloutNv] {
            assert_eq!(place_kind(game, 14), PlaceKind::Vault);
            assert_eq!(place_kind(game, 13), PlaceKind::Station);
        }
    }
}
