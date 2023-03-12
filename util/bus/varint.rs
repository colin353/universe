pub fn encode_ivarint<W: std::io::Write>(ivalue: i64, writer: &mut W) -> std::io::Result<usize> {
    let mut bytes_written = 0;
    if ivalue == 0 {
        return Ok(0);
    }

    // First bit is the sign bit
    let mut value = ivalue.abs_diff(0);
    let sign_mask = if ivalue < 0 { 0b10000000 } else { 0x0 };

    // Special rule for writing the first byte with sign + overflow bits
    if value < 0b00111111 {
        writer.write_all(&[value as u8 | sign_mask])?;
        return Ok(1);
    } else {
        bytes_written += 1;
        writer.write_all(&[((value & 0b00111111) as u8 | 0b01000000 | sign_mask)])?;
        value >>= 6;
    }

    loop {
        if value < 0b01111111 {
            writer.write_all(&[value as u8])?;
            return Ok(bytes_written + 1);
        } else {
            bytes_written += 1;
            writer.write_all(&[((value & 0b01111111) | 0b10000000) as u8])?;
            value >>= 7;
        }
    }
}

pub fn decode_ivarint(bytes: &[u8]) -> (i64, usize) {
    if bytes.len() == 0 {
        return (0, 0);
    }

    let mut out = 0;
    let mut bytes_read = 0;
    let is_negative = (bytes[0] & 0b10000000) > 0;
    out |= bytes[0] as usize & 0b00111111;
    bytes_read += 1;

    for byte in &bytes[1..] {
        out |= ((byte & 0b01111111) as usize) << (bytes_read * 7 - 1);
        bytes_read += 1;
    }

    if is_negative {
        (-(out as i64), bytes_read)
    } else {
        (out as i64, bytes_read)
    }
}

pub fn encode_varint<W: std::io::Write>(
    mut value: u64,
    writer: &mut W,
) -> Result<usize, std::io::Error> {
    let mut bytes_written = 0;
    loop {
        if value < 0b01111111 {
            writer.write_all(&[value as u8])?;
            return Ok(bytes_written + 1);
        } else {
            bytes_written += 1;
            writer.write_all(&[((value & 0b01111111) | 0b10000000) as u8])?;
            value >>= 7;
        }
    }
}

pub fn decode_varint(bytes: &[u8]) -> (usize, usize) {
    let mut out = 0;
    let mut bytes_read = 0;
    for byte in bytes {
        if byte & 0b10000000 == 0 {
            out |= (*byte as usize) << (bytes_read * 7);
            break;
        } else {
            out |= ((byte & 0b01111111) as usize) << (bytes_read * 7);
            bytes_read += 1;
        }
    }
    (out, bytes_read + 1)
}

pub fn encode_reverse_varint<W: std::io::Write>(
    value: u32,
    writer: &mut W,
) -> Result<usize, std::io::Error> {
    if value == 0 {
        return Ok(0);
    }

    if value > 0xFFFFFFF {
        writer.write_all(&[
            (value >> 28) as u8,
            ((value >> 21) as u8) | 0b10000000,
            ((value >> 14) as u8) | 0b10000000,
            ((value >> 7) as u8) | 0b10000000,
            (value as u8) | 0b10000000,
        ])?;
        Ok(5)
    } else if value > 0x1FFFFF {
        writer.write_all(&[
            ((value >> 21) as u8),
            ((value >> 14) as u8) | 0b10000000,
            ((value >> 7) as u8) | 0b10000000,
            (value as u8) | 0b10000000,
        ])?;
        Ok(4)
    } else if value > 0x3FFF {
        writer.write_all(&[
            ((value >> 14) as u8),
            ((value >> 7) as u8) | 0b10000000,
            (value as u8) | 0b10000000,
        ])?;
        Ok(3)
    } else if value > 0x7F {
        writer.write_all(&[((value >> 7) as u8), (value as u8) | 0b10000000])?;
        Ok(2)
    } else {
        writer.write_all(&[value as u8])?;
        Ok(1)
    }
}

pub fn decode_reverse_varint(bytes: &[u8]) -> (usize, usize) {
    let mut out = 0;
    let mut bytes_read = 0;
    for byte in bytes.iter().rev() {
        if byte & 0b10000000 == 0 {
            out |= (*byte as usize) << (bytes_read * 7);
            bytes_read += 1;
            break;
        } else {
            out |= ((byte & 0b01111111) as usize) << (bytes_read * 7);
            bytes_read += 1;
        }

        if bytes_read > 9 {
            break;
        }
    }
    (out, bytes_read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_encoding() {
        let mut buf = Vec::new();
        encode_varint(123, &mut buf).unwrap();
        assert_eq!(decode_varint(&buf), (123, 1));

        let mut buf = Vec::new();
        encode_varint(455, &mut buf).unwrap();
        assert_eq!(decode_varint(&buf), (455, 2));

        let mut buf = Vec::new();
        encode_varint(123456, &mut buf).unwrap();
        assert_eq!(decode_varint(&buf), (123456, 3));

        let mut buf = Vec::new();
        encode_varint(123456789, &mut buf).unwrap();
        assert_eq!(decode_varint(&buf), (123456789, 4));
    }

    #[test]
    fn test_reverse_varint_encoding() {
        let mut buf = Vec::new();
        encode_reverse_varint(0, &mut buf).unwrap();
        assert_eq!(buf.len(), 0);
        assert_eq!(decode_reverse_varint(&buf), (0, 0));

        let mut buf = Vec::new();
        encode_reverse_varint(123, &mut buf).unwrap();
        assert_eq!(decode_reverse_varint(&buf), (123, 1));

        let mut buf = Vec::new();
        encode_reverse_varint(455, &mut buf).unwrap();
        assert_eq!(decode_reverse_varint(&buf), (455, 2));

        let mut buf = Vec::new();
        encode_reverse_varint(123456, &mut buf).unwrap();
        assert_eq!(decode_reverse_varint(&buf), (123456, 3));

        let mut buf = Vec::new();
        encode_reverse_varint(123456789, &mut buf).unwrap();
        assert_eq!(decode_reverse_varint(&buf), (123456789, 4));
    }

    #[test]
    fn test_reverse_varint_extra_data() {
        let buf = vec![0xFF, 0xFF, 0xFF, 5];
        assert_eq!(decode_reverse_varint(&buf), (5, 1));
    }

    #[test]
    fn test_encode_reverse_varint_max() {
        decode_reverse_varint(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_ivarint_encoding() {
        let mut buf = Vec::new();
        encode_ivarint(0, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (0, 0));

        let mut buf = Vec::new();
        encode_ivarint(-12, &mut buf).unwrap();
        println!("{buf:#?}");
        assert_eq!(decode_ivarint(&buf), (-12, 1));

        let mut buf = Vec::new();
        encode_ivarint(123, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (123, 2));

        let mut buf = Vec::new();
        encode_ivarint(-123, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (-123, 2));

        let mut buf = Vec::new();
        encode_ivarint(455, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (455, 2));

        let mut buf = Vec::new();
        encode_ivarint(-455, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (-455, 2));

        let mut buf = Vec::new();
        encode_ivarint(123456, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (123456, 3));

        let mut buf = Vec::new();
        encode_ivarint(-123456, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (-123456, 3));

        let mut buf = Vec::new();
        encode_ivarint(123456789, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (123456789, 4));

        let mut buf = Vec::new();
        encode_ivarint(-123456789, &mut buf).unwrap();
        assert_eq!(decode_ivarint(&buf), (-123456789, 4));
    }
}
