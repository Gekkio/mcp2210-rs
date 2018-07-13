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
