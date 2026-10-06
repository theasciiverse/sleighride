//! Spectators in the workshop: elves packed in stands beside the track,
//! cheering the racers on. Every few seconds someone in a stand gets
//! carried away and lobs a present at a racer coming past. They aim where
//! the racer is going to be, but they're spectators, not marksmen, and the
//! present's shadow on the floor shows where it will land.

use crate::kart::Kart;
use crate::track::Track;
use serde::{Deserialize, Serialize};

/// Elves in each stand, standing in a row along the road.
pub const PER_STAND: usize = 5;
/// How far from the road's edge the stands are.
const BACK: f32 = 7.0;
/// Seconds between throws from one stand, and the first wait.
const COOL: f32 = 6.0;
/// How near a racer has to come before a stand throws at it.
const RANGE: f32 = 45.0;
/// Seconds a present is in the air.
pub const FLIGHT: f32 = 1.1;
/// Seconds of arms in the air after a throw or a racer going past.
const CHEER: f32 = 1.4;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Fan {
    pub x: f32,
    pub y: f32,
    /// Facing the road.
    pub heading: f32,
    /// Seconds left of cheering, arms up.
    pub cheer: f32,
    /// Which coat and hat, for drawing.
    pub look: u8,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Stand {
    x: f32,
    y: f32,
    cool: f32,
}

/// A present thrown from the stands: where from, and its velocity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lob {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    /// The racer it was thrown at.
    pub at: usize,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Crowd {
    pub fans: Vec<Fan>,
    stands: Vec<Stand>,
    seed: u32,
}

fn hash(n: u32) -> u32 {
    let mut h = n.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^ (h >> 13)
}

/// Where the elves stand: (x, y, facing), stand by stand, alternating sides.
pub fn spots(track: &Track) -> Vec<(f32, f32, f32)> {
    let n = track.len();
    let mut out = Vec::new();
    for (k, &f) in track.stands.iter().enumerate() {
        let i = (f * n as f32) as usize % n;
        let (px, py) = track.pts[i];
        let (tx, ty) = track.tangent(i);
        let side = if k % 2 == 0 { 1.0 } else { -1.0 };
        let off = track.half + BACK;
        // left of the direction of travel is (-ty, tx)
        let (cx, cy) = (px - ty * off * side, py + tx * off * side);
        let facing = (py - cy).atan2(px - cx);
        for e in 0..PER_STAND {
            let along = (e as f32 - (PER_STAND - 1) as f32 / 2.0) * 2.6;
            let back = if e % 2 == 0 { 0.0 } else { 1.2 };
            out.push((cx + tx * along - ty * back * side, cy + ty * along + tx * back * side, facing));
        }
    }
    out
}

impl Crowd {
    pub fn new(track: &Track, seed: u32) -> Crowd {
        let spots = spots(track);
        let fans = spots.iter().enumerate().map(|(k, &(x, y, heading))| Fan { x, y, heading, cheer: 0.0, look: (hash(k as u32 + seed) % 4) as u8 }).collect();
        let stands = spots
            .chunks(PER_STAND)
            .enumerate()
            .map(|(k, c)| {
                let (x, y) = c.iter().fold((0.0, 0.0), |a, s| (a.0 + s.0 / PER_STAND as f32, a.1 + s.1 / PER_STAND as f32));
                // stagger the first throws so the stands don't all go at once
                Stand { x, y, cool: COOL * 0.5 + k as f32 * 1.3 }
            })
            .collect();
        Crowd { fans, stands, seed: seed.max(1) }
    }

    fn roll(&mut self) -> f32 {
        self.seed = hash(self.seed);
        (self.seed & 0xffff) as f32 / 65535.0
    }

    /// One step: cheering, and any presents thrown this step.
    pub fn step(&mut self, dt: f32, karts: &[&Kart]) -> Vec<Lob> {
        let mut out = Vec::new();
        for f in &mut self.fans {
            f.cheer = (f.cheer - dt).max(0.0);
        }
        for s in 0..self.stands.len() {
            self.stands[s].cool -= dt;
            let (sx, sy) = (self.stands[s].x, self.stands[s].y);
            // anyone coming up on this stand, not yet past it
            let coming = karts.iter().enumerate().filter(|(_, k)| {
                let (dx, dy) = (sx - k.x, sy - k.y);
                let d = dx.hypot(dy);
                d < RANGE && d > 8.0 && !k.stunned() && (dx * k.heading.cos() + dy * k.heading.sin()) > 0.0
            });
            let Some((at, k)) = coming.min_by(|a, b| {
                let da = (sx - a.1.x).hypot(sy - a.1.y);
                let db = (sx - b.1.x).hypot(sy - b.1.y);
                da.total_cmp(&db)
            }) else {
                continue;
            };
            let (kx, ky, kh, ksp) = (k.x, k.y, k.heading, k.speed.max(0.0));
            for f in self.fans[s * PER_STAND..(s + 1) * PER_STAND].iter_mut() {
                // racers coming past get a cheer anyway
                if (f.x - kx).hypot(f.y - ky) < 25.0 {
                    f.cheer = f.cheer.max(0.5);
                }
            }
            if self.stands[s].cool > 0.0 {
                continue;
            }
            self.stands[s].cool = COOL * (0.8 + 0.5 * self.roll());
            // aim at where the racer will be, give or take
            let (along, across) = ((self.roll() - 0.5) * 12.0, (self.roll() - 0.5) * 9.0);
            let (c, sn) = (kh.cos(), kh.sin());
            let lead = ksp * FLIGHT + along;
            let (lx, ly) = (kx + c * lead - sn * across, ky + sn * lead + c * across);
            let who = s * PER_STAND + (self.roll() * PER_STAND as f32) as usize % PER_STAND;
            let f = &mut self.fans[who];
            f.cheer = CHEER;
            let z = 3.0;
            // height above the ground: z + vz t - 18 t^2 lands at 0 after FLIGHT
            let vz = (18.0 * FLIGHT * FLIGHT - z) / FLIGHT;
            out.push(Lob { x: f.x, y: f.y, z, vx: (lx - f.x) / FLIGHT, vy: (ly - f.y) / FLIGHT, vz, at });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kart::DT;

    #[test]
    fn the_stands_stand_off_the_road_facing_it() {
        let t = Track::workshop();
        let c = Crowd::new(&t, 3);
        assert_eq!(c.fans.len(), t.stands.len() * PER_STAND);
        for f in &c.fans {
            assert!(t.dist(f.x, f.y) > t.half + 4.0, "off the road");
            assert!(t.trees.iter().all(|p| (p.x - f.x).hypot(p.y - f.y) > 3.0), "not inside a toy");
            // facing roughly towards the road
            let i = t.idx(f.x, f.y).unwrap();
            let (px, py) = t.pts[i];
            let to = (py - f.y).atan2(px - f.x);
            let turn = (to - f.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
            assert!(turn.abs() < 0.8);
        }
        assert!(Crowd::new(&Track::frozen_lake(), 3).fans.is_empty(), "no crowds outdoors");
    }

    #[test]
    fn a_fan_lobs_a_present_onto_the_road_ahead_of_a_racer() {
        let t = Track::workshop();
        let mut c = Crowd::new(&t, 3);
        // a racer driving up the road towards the first stand
        let i = (t.stands[0] * t.len() as f32) as usize - 30;
        let (x, y) = t.pts[i];
        let (tx, ty) = t.tangent(i);
        let mut k = Kart::new(x, y, ty.atan2(tx), crate::roster::ROSTER[0].stats);
        k.speed = 40.0;
        let mut lob = None;
        for _ in 0..(10.0 / DT) as usize {
            if let Some(l) = c.step(DT, &[&k]).first() {
                lob = Some(*l);
                break;
            }
        }
        let l = lob.expect("someone threw");
        assert_eq!(l.at, 0);
        let (lx, ly) = (l.x + l.vx * FLIGHT, l.y + l.vy * FLIGHT);
        assert!(t.dist(lx, ly) < t.half + 2.0, "it lands on the road");
        // ahead of the racer, near where it will be
        let ahead = (lx - k.x) * k.heading.cos() + (ly - k.y) * k.heading.sin();
        assert!(ahead > 20.0 && ahead < 70.0, "ahead by {ahead}");
        assert!(c.fans.iter().any(|f| f.cheer > 1.0), "and cheers");
    }
}
