//! BC7 blocks: 16 bytes for 4 × 4 pixels, in one of eight modes.
//!
//! Most textures of current mods are BC7. A block starts with its mode in
//! unary (the position of the first set bit), then partition, rotation and
//! index-selection bits as the mode has them, the colour endpoints channel
//! by channel, the alpha endpoints, the p-bits that extend the endpoints by
//! one bit, and the indices that pick a colour between two endpoints for
//! each pixel. The tables are those of the format's specification.

/// What a mode is made of: subsets, partition bits, rotation bits, index
/// selection bit, colour bits, alpha bits, p-bits per endpoint, shared
/// p-bits per subset, primary and secondary index bits.
#[derive(Clone, Copy)]
struct Mode {
    subsets: usize,
    partition_bits: u32,
    rotation_bits: u32,
    selection_bits: u32,
    color_bits: u32,
    alpha_bits: u32,
    endpoint_pbits: bool,
    shared_pbits: bool,
    index_bits: u32,
    index2_bits: u32,
}

const MODES: [Mode; 8] = [
    Mode {
        subsets: 3,
        partition_bits: 4,
        rotation_bits: 0,
        selection_bits: 0,
        color_bits: 4,
        alpha_bits: 0,
        endpoint_pbits: true,
        shared_pbits: false,
        index_bits: 3,
        index2_bits: 0,
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        selection_bits: 0,
        color_bits: 6,
        alpha_bits: 0,
        endpoint_pbits: false,
        shared_pbits: true,
        index_bits: 3,
        index2_bits: 0,
    },
    Mode {
        subsets: 3,
        partition_bits: 6,
        rotation_bits: 0,
        selection_bits: 0,
        color_bits: 5,
        alpha_bits: 0,
        endpoint_pbits: false,
        shared_pbits: false,
        index_bits: 2,
        index2_bits: 0,
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        selection_bits: 0,
        color_bits: 7,
        alpha_bits: 0,
        endpoint_pbits: true,
        shared_pbits: false,
        index_bits: 2,
        index2_bits: 0,
    },
    Mode {
        subsets: 1,
        partition_bits: 0,
        rotation_bits: 2,
        selection_bits: 1,
        color_bits: 5,
        alpha_bits: 6,
        endpoint_pbits: false,
        shared_pbits: false,
        index_bits: 2,
        index2_bits: 3,
    },
    Mode {
        subsets: 1,
        partition_bits: 0,
        rotation_bits: 2,
        selection_bits: 0,
        color_bits: 7,
        alpha_bits: 8,
        endpoint_pbits: false,
        shared_pbits: false,
        index_bits: 2,
        index2_bits: 2,
    },
    Mode {
        subsets: 1,
        partition_bits: 0,
        rotation_bits: 0,
        selection_bits: 0,
        color_bits: 7,
        alpha_bits: 7,
        endpoint_pbits: true,
        shared_pbits: false,
        index_bits: 4,
        index2_bits: 0,
    },
    Mode {
        subsets: 2,
        partition_bits: 6,
        rotation_bits: 0,
        selection_bits: 0,
        color_bits: 5,
        alpha_bits: 5,
        endpoint_pbits: true,
        shared_pbits: false,
        index_bits: 2,
        index2_bits: 0,
    },
];

/// Which subset each pixel belongs to, for the 64 partitions of two
/// subsets: one digit per pixel, row by row.
const PARTITIONS_2: [&str; 64] = [
    "0011001100110011",
    "0001000100010001",
    "0111011101110111",
    "0001001100110111",
    "0000000100010011",
    "0011011101111111",
    "0001001101111111",
    "0000000100110111",
    "0000000000010011",
    "0011011111111111",
    "0000000101111111",
    "0000000000010111",
    "0001011111111111",
    "0000000011111111",
    "0000111111111111",
    "0000000000001111",
    "0000100011101111",
    "0111000100000000",
    "0000000010001110",
    "0111001100010000",
    "0011000100000000",
    "0000100011001110",
    "0000000010001100",
    "0111001100110001",
    "0011000100010000",
    "0000100010001100",
    "0110011001100110",
    "0011011001101100",
    "0001011111101000",
    "0000111111110000",
    "0111000110001110",
    "0011100110011100",
    "0101010101010101",
    "0000111100001111",
    "0101101001011010",
    "0011001111001100",
    "0011110000111100",
    "0101010110101010",
    "0110100101101001",
    "0101101010100101",
    "0111001111001110",
    "0001001111001000",
    "0011001001001100",
    "0011101111011100",
    "0110100110010110",
    "0011110011000011",
    "0110011010011001",
    "0000011001100000",
    "0100111001000000",
    "0010011100100000",
    "0000001001110010",
    "0000010011100100",
    "0110110010010011",
    "0011011011001001",
    "0110001110011100",
    "0011100111000110",
    "0110110011001001",
    "0110001100111001",
    "0111111010000001",
    "0001100011100111",
    "0000111100110011",
    "0011001111110000",
    "0010001011101110",
    "0100010001110111",
];

/// Which subset each pixel belongs to, for the 64 partitions of three
/// subsets: one digit per pixel, row by row.
const PARTITIONS_3: [&str; 64] = [
    "0011001102212222",
    "0001001122112221",
    "0000200122112211",
    "0222002200110111",
    "0000000011221122",
    "0011001100220022",
    "0022002211111111",
    "0011001122112211",
    "0000000011112222",
    "0000111111112222",
    "0000111122222222",
    "0012001200120012",
    "0112011201120112",
    "0122012201220122",
    "0011011211221222",
    "0011200122002220",
    "0001001101121122",
    "0111001120012200",
    "0000112211221122",
    "0022002200221111",
    "0111011102220222",
    "0001000122212221",
    "0000001101220122",
    "0000110022102210",
    "0122012200110000",
    "0012001211222222",
    "0110122112210110",
    "0000011012211221",
    "0022110211020022",
    "0110011020022222",
    "0011012201220011",
    "0000200022112221",
    "0000000211221222",
    "0222002200120011",
    "0011001200220222",
    "0120012001200120",
    "0000111122220000",
    "0120120120120120",
    "0120201212010120",
    "0011220011220011",
    "0011112222000011",
    "0101010122222222",
    "0000000021212121",
    "0022112200221122",
    "0022001100220011",
    "0220122102201221",
    "0101222222220101",
    "0000212121212121",
    "0101010101012222",
    "0222011102220111",
    "0002111200021112",
    "0000211221122112",
    "0222011101110222",
    "0002111211120002",
    "0110011001102222",
    "0000000021122112",
    "0110011022222222",
    "0022001100110022",
    "0022112211220022",
    "0000000000002112",
    "0002000100020001",
    "0222122202221222",
    "0101222222222222",
    "0111201122012220",
];

/// The pixel of subset 1 whose index has one bit less, in partitions of two.
const ANCHORS_2: [usize; 64] = [
    15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 15, 2, 8, 2, 2, 8, 8, 15, 2, 8,
    2, 2, 8, 8, 2, 2, 15, 15, 6, 8, 2, 8, 15, 15, 2, 8, 2, 2, 2, 15, 15, 6, 6, 2, 6, 8, 15, 15, 2,
    2, 15, 15, 15, 15, 15, 2, 2, 15,
];

/// The anchor pixel of subset 1 in partitions of three.
const ANCHORS_3A: [usize; 64] = [
    3, 3, 15, 15, 8, 3, 15, 15, 8, 8, 6, 6, 6, 5, 3, 3, 3, 3, 8, 15, 3, 3, 6, 10, 5, 8, 8, 6, 8, 5,
    15, 15, 8, 15, 3, 5, 6, 10, 8, 15, 15, 3, 15, 5, 15, 15, 15, 15, 3, 15, 5, 5, 5, 8, 5, 10, 5,
    10, 8, 13, 15, 12, 3, 3,
];

/// The anchor pixel of subset 2 in partitions of three.
const ANCHORS_3B: [usize; 64] = [
    15, 8, 8, 3, 15, 15, 3, 8, 15, 15, 15, 15, 15, 15, 15, 8, 15, 8, 15, 3, 15, 8, 15, 8, 3, 15, 6,
    10, 15, 15, 10, 8, 15, 3, 15, 10, 10, 8, 9, 10, 6, 15, 8, 15, 3, 6, 6, 8, 15, 3, 15, 15, 15,
    15, 15, 15, 15, 15, 15, 15, 3, 15, 15, 8,
];

const WEIGHTS_2: [u32; 4] = [0, 21, 43, 64];
const WEIGHTS_3: [u32; 8] = [0, 9, 18, 27, 37, 46, 55, 64];
const WEIGHTS_4: [u32; 16] = [0, 4, 9, 13, 17, 21, 26, 30, 34, 38, 43, 47, 51, 55, 60, 64];

/// Reads a block's bits from the lowest upwards.
struct Bits {
    value: u128,
    position: u32,
}

impl Bits {
    fn take(&mut self, count: u32) -> u32 {
        if count == 0 || self.position >= 128 {
            return 0;
        }
        let mask = (1u128 << count.min(32)) - 1;
        let taken = (self.value >> self.position) & mask;
        self.position += count;
        u32::try_from(taken).unwrap_or(0)
    }
}

/// The subset of pixel `pixel` in partition `partition` of a mode with
/// `subsets` subsets.
fn subset_of(subsets: usize, partition: usize, pixel: usize) -> usize {
    let table = match subsets {
        2 => &PARTITIONS_2,
        3 => &PARTITIONS_3,
        _ => return 0,
    };
    table
        .get(partition)
        .and_then(|row| row.as_bytes().get(pixel))
        .map_or(0, |digit| usize::from(digit.saturating_sub(b'0')))
}

/// Whether pixel `pixel` is the anchor of its subset: its index has one
/// bit less, because its top bit is always zero.
fn is_anchor(subsets: usize, partition: usize, pixel: usize) -> bool {
    if pixel == 0 {
        return true;
    }
    match subsets {
        2 => ANCHORS_2.get(partition) == Some(&pixel),
        3 => ANCHORS_3A.get(partition) == Some(&pixel) || ANCHORS_3B.get(partition) == Some(&pixel),
        _ => false,
    }
}

/// A channel of `bits` bits widened to eight, its top bits repeated below.
fn widen(value: u32, bits: u32) -> u32 {
    if bits == 0 || bits >= 8 {
        return value & 0xFF;
    }
    let shifted = value << (8 - bits);
    shifted | (shifted >> bits)
}

fn interpolate(low: u32, high: u32, weight: u32) -> u8 {
    u8::try_from(((64 - weight) * low + weight * high + 32) >> 6).unwrap_or(u8::MAX)
}

fn weight(bits: u32, index: u32) -> u32 {
    let table: &[u32] = match bits {
        2 => &WEIGHTS_2,
        3 => &WEIGHTS_3,
        _ => &WEIGHTS_4,
    };
    table.get(index as usize).copied().unwrap_or(0)
}

/// The endpoints of a block: per subset two, each red, green, blue, alpha.
type Endpoints = [[[u32; 4]; 2]; 3];

/// Reads the endpoints and their p-bits, and widens them to eight bits.
fn read_endpoints(bits: &mut Bits, mode: Mode) -> Endpoints {
    let mut endpoints: Endpoints = [[[0; 4]; 2]; 3];
    for channel in 0..3 {
        for subset in endpoints.iter_mut().take(mode.subsets) {
            for end in subset.iter_mut() {
                if let Some(slot) = end.get_mut(channel) {
                    *slot = bits.take(mode.color_bits);
                }
            }
        }
    }
    for subset in endpoints.iter_mut().take(mode.subsets) {
        for [.., alpha] in subset.iter_mut() {
            *alpha = if mode.alpha_bits > 0 {
                bits.take(mode.alpha_bits)
            } else {
                255
            };
        }
    }

    // P-bits add one bit at the bottom of every channel of an endpoint.
    let mut color_bits = mode.color_bits;
    let mut alpha_bits = mode.alpha_bits;
    if mode.endpoint_pbits || mode.shared_pbits {
        let mut pbits = [[0u32; 2]; 3];
        for subset in pbits.iter_mut().take(mode.subsets) {
            *subset = if mode.endpoint_pbits {
                [bits.take(1), bits.take(1)]
            } else {
                let shared = bits.take(1);
                [shared, shared]
            };
        }
        for (subset, pbit) in endpoints.iter_mut().zip(pbits).take(mode.subsets) {
            for (end, bit) in subset.iter_mut().zip(pbit) {
                for (channel, value) in end.iter_mut().enumerate() {
                    if channel < 3 || mode.alpha_bits > 0 {
                        *value = (*value << 1) | bit;
                    }
                }
            }
        }
        color_bits += 1;
        if mode.alpha_bits > 0 {
            alpha_bits += 1;
        }
    }
    for subset in endpoints.iter_mut().take(mode.subsets) {
        for end in subset.iter_mut() {
            for (channel, value) in end.iter_mut().enumerate() {
                if channel < 3 {
                    *value = widen(*value, color_bits);
                } else if mode.alpha_bits > 0 {
                    *value = widen(*value, alpha_bits);
                }
            }
        }
    }
    endpoints
}

/// Reads the primary and, where the mode has them, the secondary indices.
fn read_indices(bits: &mut Bits, mode: Mode, partition: usize) -> ([u32; 16], [u32; 16]) {
    let mut primary = [0u32; 16];
    for (pixel, index) in primary.iter_mut().enumerate() {
        let anchor = is_anchor(mode.subsets, partition, pixel);
        *index = bits.take(mode.index_bits - u32::from(anchor));
    }
    let mut secondary = [0u32; 16];
    if mode.index2_bits > 0 {
        for (pixel, index) in secondary.iter_mut().enumerate() {
            *index = bits.take(mode.index2_bits - u32::from(pixel == 0));
        }
    }
    (primary, secondary)
}

/// The 16 pixels of a block as red, green, blue and alpha, row by row.
/// An invalid block is transparent black.
pub(crate) fn decode_block(block: &[u8]) -> [[u8; 4]; 16] {
    let mut pixels = [[0u8; 4]; 16];
    let Some(bytes) = block
        .get(..16)
        .and_then(|bytes| <[u8; 16]>::try_from(bytes).ok())
    else {
        return pixels;
    };
    let value = u128::from_le_bytes(bytes);
    let mode_number = value.trailing_zeros();
    let Some(mode) = MODES.get(mode_number as usize).copied() else {
        return pixels;
    };
    let mut bits = Bits {
        value,
        position: mode_number + 1,
    };
    let partition = bits.take(mode.partition_bits) as usize;
    let rotation = bits.take(mode.rotation_bits);
    let selection = bits.take(mode.selection_bits);
    let endpoints = read_endpoints(&mut bits, mode);
    let (primary, secondary) = read_indices(&mut bits, mode, partition);

    for (pixel, ((target, first), second)) in
        pixels.iter_mut().zip(primary).zip(secondary).enumerate()
    {
        let subset = subset_of(mode.subsets, partition, pixel);
        let Some([low, high]) = endpoints.get(subset).copied() else {
            continue;
        };
        let (color_weight, alpha_weight) = if mode.index2_bits == 0 {
            let both = weight(mode.index_bits, first);
            (both, both)
        } else if selection == 0 {
            (
                weight(mode.index_bits, first),
                weight(mode.index2_bits, second),
            )
        } else {
            (
                weight(mode.index2_bits, second),
                weight(mode.index_bits, first),
            )
        };
        let [low_r, low_g, low_b, low_a] = low;
        let [high_r, high_g, high_b, high_a] = high;
        let mut rgba = [
            interpolate(low_r, high_r, color_weight),
            interpolate(low_g, high_g, color_weight),
            interpolate(low_b, high_b, color_weight),
            if mode.alpha_bits > 0 {
                interpolate(low_a, high_a, alpha_weight)
            } else {
                255
            },
        ];
        // Modes 4 and 5 may store one colour channel in the alpha's place.
        match rotation {
            1 => rgba.swap(0, 3),
            2 => rgba.swap(1, 3),
            3 => rgba.swap(2, 3),
            _ => {}
        }
        *target = rgba;
    }
    pixels
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// Writes bits from the lowest upwards.
    struct Writer {
        value: u128,
        position: u32,
    }

    impl Writer {
        fn put(&mut self, count: u32, value: u32) {
            self.value |= (u128::from(value) & ((1u128 << count) - 1)) << self.position;
            self.position += count;
        }
    }

    #[test]
    fn a_mode_6_block_of_one_colour() {
        // Mode 6: one subset, 7-bit colour and alpha, a p-bit per endpoint,
        // 4-bit indices.
        let mut block = Writer {
            value: 0,
            position: 0,
        };
        block.put(7, 0b100_0000);
        // Red, green, blue, alpha of both endpoints: 127, 0, 63, 127.
        for value in [127, 127, 0, 0, 63, 63, 127, 127] {
            block.put(7, value);
        }
        block.put(1, 1);
        block.put(1, 1);
        // All indices 0: the first endpoint.
        let pixels = decode_block(&block.value.to_le_bytes());
        // The p-bit lifts green from 0 to 1 as well.
        assert_eq!(pixels[0], [255, 1, 127, 255]);
        assert_eq!(pixels[15], [255, 1, 127, 255]);
    }

    #[test]
    fn mode_6_interpolates_between_its_endpoints() {
        let mut block = Writer {
            value: 0,
            position: 0,
        };
        block.put(7, 0b100_0000);
        // Red 0 to 127 (255 with the p-bit); other channels 0, alpha 127.
        for value in [0, 127, 0, 0, 0, 0, 127, 127] {
            block.put(7, value);
        }
        block.put(1, 0);
        block.put(1, 1);
        // Pixel 0 (anchor, 3 bits) index 0, pixel 1 index 15.
        block.put(3, 0);
        block.put(4, 15);
        let pixels = decode_block(&block.value.to_le_bytes());
        assert_eq!(pixels[0][0], 0);
        assert_eq!(pixels[1][0], 255);
    }

    #[test]
    fn an_invalid_mode_is_transparent_black() {
        assert_eq!(decode_block(&[0u8; 16]), [[0u8; 4]; 16]);
        assert_eq!(decode_block(&[1u8; 3]), [[0u8; 4]; 16]);
    }

    #[test]
    fn partition_tables_match_their_anchors() {
        // Every anchor pixel belongs to the subset it anchors.
        for partition in 0..64 {
            let a2 = ANCHORS_2[partition];
            assert_eq!(
                subset_of(2, partition, a2),
                1,
                "two subsets, partition {partition}"
            );
            assert_eq!(subset_of(2, partition, 0), 0);
            let a = ANCHORS_3A[partition];
            let b = ANCHORS_3B[partition];
            assert_eq!(
                subset_of(3, partition, a),
                1,
                "three subsets, partition {partition}"
            );
            assert_eq!(
                subset_of(3, partition, b),
                2,
                "three subsets, partition {partition}"
            );
            assert_eq!(subset_of(3, partition, 0), 0);
        }
    }

    proptest! {
        #[test]
        fn never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..20)) {
            let _ = decode_block(&bytes);
        }
    }
}
