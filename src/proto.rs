//! Wire messages between a Sleighride host and its guests.
//!
//! The host runs the only race. Each guest sends its controls every step
//! and its racer choice when it changes; the host sends the lobby (course,
//! difficulty, who is in and on what) every half second, and while racing
//! the whole race as it stands every step (`race::Snap`), events included,
//! so guests hear and draw the same things.
//!
//! Encoding is postcard (serde) inside u32-length-prefixed frames, as in
//! Tetrix and Panzer.

use crate::kart::Controls;
use crate::race::Snap;
use serde::{Deserialize, Serialize};

/// Bumped on any incompatible change; the host rejects mismatched guests.
pub const VERSION: u32 = 1;

/// Who is in: (seat id, name, racer). Seat 0 is the host.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Lobby {
    pub course: usize,
    pub level: usize,
    pub players: Vec<(u32, String, usize)>,
    /// Bumped every time a race starts, so guests rebuild theirs.
    pub race_no: u32,
    pub racing: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Msg {
    /// Guest's first message.
    Hello { name: String, version: u32 },
    Reject { reason: String },
    /// The host's answer: your seat id.
    Welcome { seat: u32 },
    Lobby(Lobby),
    Frame { race_no: u32, snap: Box<Snap> },
    Input(Controls),
    Pick(usize),
    /// Keeps a quiet connection from timing out.
    Ping,
}

impl std::fmt::Debug for Msg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Msg::Hello { name, version } => write!(f, "Hello({name:?}, v{version})"),
            Msg::Reject { reason } => write!(f, "Reject({reason:?})"),
            Msg::Welcome { seat } => write!(f, "Welcome(seat {seat})"),
            Msg::Lobby(l) => write!(f, "Lobby({l:?})"),
            Msg::Frame { race_no, .. } => write!(f, "Frame(race {race_no})"),
            Msg::Input(c) => write!(f, "Input({c:?})"),
            Msg::Pick(p) => write!(f, "Pick({p})"),
            Msg::Ping => write!(f, "Ping"),
        }
    }
}

pub fn encode(m: &Msg) -> Vec<u8> {
    postcard::to_allocvec(m).expect("postcard encoding of an owned message cannot fail")
}

pub fn decode(b: &[u8]) -> Option<Msg> {
    postcard::from_bytes(b).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::race::Race;
    use crate::track::Track;

    #[test]
    fn messages_round_trip_and_frames_stay_small() {
        let lobby = Lobby { course: 2, level: 1, players: vec![(0, "dåvid".into(), 0), (3, "guest".into(), 4)], race_no: 7, racing: true };
        match decode(&encode(&Msg::Lobby(lobby.clone()))) {
            Some(Msg::Lobby(l)) => assert_eq!(l, lobby),
            other => panic!("decoded {other:?}"),
        }
        let mut r = Race::with_humans(Track::thin_ice(), &[0, 4], 5);
        r.autopilot = true;
        for _ in 0..1800 {
            r.step(Controls::default());
        }
        let bytes = encode(&Msg::Frame { race_no: 7, snap: Box::new(r.snapshot()) });
        // sent sixty times a second
        assert!(bytes.len() < 6000, "frame is {} bytes", bytes.len());
        assert!(matches!(decode(&bytes), Some(Msg::Frame { race_no: 7, .. })));
        assert!(decode(&[0xff, 0xff]).is_none());
    }
}
