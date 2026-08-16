use std::io::Write;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
    Stop,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Track {
    Forward,
    Reverse,
    Halt,
}

impl Direction {
    pub fn tracks(self) -> (Track, Track) {
        match self {
            Direction::Up => (Track::Forward, Track::Forward),
            Direction::Down => (Track::Reverse, Track::Reverse),
            Direction::Left => (Track::Reverse, Track::Forward),
            Direction::Right => (Track::Forward, Track::Reverse),
            Direction::Stop => (Track::Halt, Track::Halt),
        }
    }

    pub fn parse(payload: &[u8]) -> Option<Self> {
        let key = b"\"dir\"";
        let key_at = find(payload, key)? + key.len();
        let colon = find(&payload[key_at..], b":")? + key_at;
        let open = find(&payload[colon..], b"\"")? + colon;
        let value = &payload[open + 1..];
        let end = find(value, b"\"")?;
        match &value[..end] {
            b"up" => Some(Direction::Up),
            b"down" => Some(Direction::Down),
            b"left" => Some(Direction::Left),
            b"right" => Some(Direction::Right),
            b"stop" => Some(Direction::Stop),
            _ => None,
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

pub fn encode_cam_frame(jpeg: &[u8], out: &mut Vec<u8>) {
    out.clear();
    out.extend_from_slice(b"{\"cam\":\"");
    let tail = out.len() + jpeg.len().div_ceil(3) * 4;
    out.resize(tail, 0);
    let written = STANDARD.encode_slice(jpeg, &mut out[tail - jpeg.len().div_ceil(3) * 4..]);
    out.truncate(tail - jpeg.len().div_ceil(3) * 4 + written.unwrap_or(0));
    out.extend_from_slice(b"\"}");
}

pub fn encode_telemetry(free_heap: u32, uptime_s: u64, out: &mut Vec<u8>) {
    out.clear();
    let _ = write!(out, "{{\"heap\":{free_heap},\"uptime_s\":{uptime_s}}}");
}
