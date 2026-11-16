//! Writing images as PNG, for pictures the window shows.

use std::io::Write;

/// The file signature every PNG starts with.
const SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// An RGB image of `width × height` pixels, rows top to bottom, as PNG.
pub(crate) fn encode_rgb(width: u32, height: u32, rgb: &[u8]) -> std::io::Result<Vec<u8>> {
    let row = (width as usize).saturating_mul(3);
    if row == 0 || rgb.len() != row.saturating_mul(height as usize) {
        return Err(std::io::Error::other("the pixels do not match the size"));
    }
    // Each row starts with its filter; none.
    let mut raw = Vec::with_capacity(rgb.len() + height as usize);
    for line in rgb.chunks_exact(row) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut compressor = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    compressor.write_all(&raw)?;
    let data = compressor.finish()?;

    let mut header = Vec::with_capacity(13);
    header.extend(width.to_be_bytes());
    header.extend(height.to_be_bytes());
    // 8 bits per channel, colour type RGB, standard compression and
    // filtering, no interlacing.
    header.extend([8, 2, 0, 0, 0]);

    let mut png = SIGNATURE.to_vec();
    chunk(&mut png, *b"IHDR", &header)?;
    chunk(&mut png, *b"IDAT", &data)?;
    chunk(&mut png, *b"IEND", &[])?;
    Ok(png)
}

fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) -> std::io::Result<()> {
    let length =
        u32::try_from(data.len()).map_err(|_| std::io::Error::other("the image is too large"))?;
    png.extend(length.to_be_bytes());
    let mut crc = flate2::Crc::new();
    crc.update(&kind);
    crc.update(data);
    png.extend(kind);
    png.extend(data);
    png.extend(crc.sum().to_be_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tiny_image_is_a_valid_png() {
        let png = encode_rgb(2, 1, &[255, 0, 0, 0, 0, 255]).unwrap();
        assert!(png.starts_with(SIGNATURE));
        // IHDR right after the signature, 13 bytes long.
        assert_eq!(&png[8..16], b"\0\0\0\x0dIHDR");
        // The CRC of the IEND chunk is always the same.
        assert!(png.ends_with(&[0, 0, 0, 0, b'I', b'E', b'N', b'D', 0xAE, 0x42, 0x60, 0x82]));
    }

    #[test]
    fn a_wrong_size_is_refused() {
        assert!(encode_rgb(2, 2, &[0; 3]).is_err());
        assert!(encode_rgb(0, 0, &[]).is_err());
    }
}
