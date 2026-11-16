//! Encodings the report uses to carry binary data as text.

/// Bytes in Base64, the standard alphabet with padding.
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let symbol =
        |value: u32| char::from(ALPHABET.get((value & 63) as usize).copied().unwrap_or(b'='));
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let [a, b, c] = [0, 1, 2].map(|index| u32::from(chunk.get(index).copied().unwrap_or(0)));
        let group = (a << 16) | (b << 8) | c;
        text.push(symbol(group >> 18));
        text.push(symbol(group >> 12));
        text.push(if chunk.len() > 1 {
            symbol(group >> 6)
        } else {
            '='
        });
        text.push(if chunk.len() > 2 { symbol(group) } else { '=' });
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(&[0xFF, 0xEF]), "/+8=");
    }
}
