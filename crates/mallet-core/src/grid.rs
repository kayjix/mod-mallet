//! The grid of outside cells the games lay their worldspaces out in, and
//! positions in whole game units.
//!
//! Readers, analyses and the engine all turn positions into cells and
//! round them to whole units; they do it here so that a position lands in
//! the same cell everywhere.

/// Game units per side of an outside cell.
pub const CELL_SIDE: i32 = 4096;

/// Positions are rounded into this range before they become whole
/// numbers. The worlds of the games are a few hundred thousand units wide.
const WHOLE_LIMIT: f64 = 1.0e9;

/// The outside cell (column or row) a coordinate in game units lies in. A
/// coordinate that is not a number counts as the origin; one far outside
/// any world counts as the farthest cell a plugin can name.
#[allow(clippy::cast_possible_truncation)]
pub fn cell_of(coordinate: impl Into<f64>) -> i32 {
    // Worldspaces lie within a few hundred cells of their origin.
    (coordinate.into() / f64::from(CELL_SIDE))
        .floor()
        .clamp(-32_768.0, 32_767.0) as i32
}

/// The outside cell (column or row) a coordinate in whole game units lies
/// in.
pub fn cell_of_unit(coordinate: i32) -> i32 {
    coordinate.div_euclid(CELL_SIDE)
}

/// A coordinate or length rounded to whole game units. Values far outside
/// any world are cut off at a billion units; a value that is not a number
/// counts as zero.
#[allow(clippy::cast_possible_truncation)]
pub fn whole(value: impl Into<f64>) -> i32 {
    // Bounded before converting; positions of the games are far inside.
    value.into().round().clamp(-WHOLE_LIMIT, WHOLE_LIMIT) as i32
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn cells_are_found_from_game_units() {
        assert_eq!(cell_of(0.0_f32), 0);
        assert_eq!(cell_of(4095.9_f64), 0);
        assert_eq!(cell_of(4096.0_f32), 1);
        assert_eq!(cell_of(-1.0_f32), -1);
        assert_eq!(cell_of(f32::NAN), 0);
        assert_eq!(cell_of(f64::INFINITY), 32_767);
        assert_eq!(cell_of_unit(4095), 0);
        assert_eq!(cell_of_unit(-1), -1);
        assert_eq!(cell_of_unit(-4096), -1);
        assert_eq!(cell_of_unit(-4097), -2);
    }

    #[test]
    fn whole_units_are_rounded_and_bounded() {
        assert_eq!(whole(12.5_f32), 13);
        assert_eq!(whole(-12.5_f64), -13);
        assert_eq!(whole(f32::NAN), 0);
        assert_eq!(whole(f64::INFINITY), 1_000_000_000);
    }

    proptest! {
        #[test]
        fn floats_and_whole_units_land_in_the_same_cell(coordinate in -10_000_000i32..10_000_000) {
            prop_assert_eq!(cell_of(f64::from(coordinate)), cell_of_unit(coordinate));
        }

        #[test]
        fn never_panics(value in any::<f64>(), unit in any::<i32>()) {
            let _ = cell_of(value);
            let _ = whole(value);
            let _ = cell_of_unit(unit);
        }
    }
}
