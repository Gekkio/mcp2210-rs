#[inline]
pub fn as_bool(v: u8) -> Result<bool, u8> {
    match v {
        0x00 => Ok(false),
        0x01 => Ok(true),
        _ => Err(v),
    }
}

#[inline]
pub fn as_u16(a: u8, b: u8) -> u16 {
    (u16::from(b) << 8) | u16::from(a)
}

#[inline]
pub fn as_u32(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (u32::from(d) << 24) | (u32::from(c) << 16) | (u32::from(b) << 8) | u32::from(a)
}

#[inline]
pub fn encode_utf16_to_buffer(s: &str, buf: &mut [u8]) {
    let mut idx = 0;
    for ch in s.encode_utf16() {
        buf[idx] = ch as u8;
        buf[idx + 1] = (ch >> 8) as u8;
        idx += 2;
    }
}

#[test]
fn test_as_u16() {
    assert_eq!(as_u16(0xaa, 0x55), 0x55aa);
}

#[test]
fn test_as_u32() {
    assert_eq!(as_u32(0x11, 0x22, 0x33, 0x44), 0x44332211);
}

#[test]
fn test_encode_utf16_to_buffer() {
    let mut buf = [0; 14];
    encode_utf16_to_buffer("보라색 고양이", &mut buf);
    assert_eq!(
        buf,
        [0xf4, 0xbc, 0x7c, 0xb7, 0xc9, 0xc0, 0x20, 0x00, 0xe0, 0xac, 0x91, 0xc5, 0x74, 0xc7]
    );
}
