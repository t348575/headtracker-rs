use core::fmt::Write;

use heapless::String;
use rtt_target::rprintln;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct StringFromBufError {}

pub fn string_from_buf_with_skip<const N: usize>(
    buf: &[u8],
    skip: usize,
) -> Result<String<N>, StringFromBufError> {
    let mut s = String::new();
    for i in buf.iter().skip(skip) {
        s.write_char(*i as char).map_err(|e| {
            rprintln!("{:?}", e);
            StringFromBufError {}
        })?;
    }
    Ok(s)
}

pub fn convert_accel(input: i16) -> f32 {
    (input as f32 * 2.0) / 32768.0
}

pub fn convert_gyro(input: i16) -> f32 {
    (input as f32 * 250.0) / 32768.0
}