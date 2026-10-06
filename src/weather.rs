//! Weather: blizzards that come and go, and snow tornadoes that wander the
//! course.
//!
//! - A blizzard builds over a few seconds, rages for about twenty, and
//!   eases off again; then a calm spell. While it blows, sight shortens,
//!   snow streaks across the view and gusts push everyone sideways.
//! - A snow tornado drifts against the race and weaves across the road.
//!   Touch it and you're thrown into the air, spinning.

use serde::{Deserialize, Serialize};

use crate::track::Track;

/// The first calm is short, so a race sees a blizzard early on.
const FIRST_CALM: f32 = 22.0;
const BUILD: f32 = 6.0;
/// How hard gusts push at full blow, units per second.
const PUSH: f32 = 5.0;
pub const TWISTER_R: f32 = 3.6;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
enum Spell {
    Calm(f32),
    Building(f32),
    Blowing(f32),
    Easing(f32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Twister {
    pub x: f32,
    pub y: f32,
    /// Centreline sample it is near, moving back against the race.
    pos: f32,
    phase: f32,
    /// Its spin, for drawing.
    pub spin: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Weather {
    /// 0 calm .. 1 full blizzard.
    pub blizzard: f32,
    spell: Spell,
    /// The wind's direction, a unit vector.
    pub wind: (f32, f32),
    pub twisters: Vec<Twister>,
    blizzards: bool,
    t: f32,
    seed: u32,
}

fn hash(n: u32) -> u32 {
    let mut h = n.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^ (h >> 13)
}

impl Weather {
    pub fn new(track: &Track, seed: u32) -> Weather {
        let n = track.len() as f32;
        let twisters = (0..track.twisters)
            .map(|k| {
                // spread round the lap, away from the start
                let pos = n * (0.3 + 0.4 * k as f32 / track.twisters.max(1) as f32);
                let (x, y) = track.pts[pos as usize];
                Twister { x, y, pos, phase: k as f32 * 2.1, spin: 0.0 }
            })
            .collect();
        Weather { blizzard: 0.0, spell: Spell::Calm(FIRST_CALM), wind: (1.0, 0.0), twisters, blizzards: track.blizzards, t: 0.0, seed: seed.max(1) }
    }

    fn roll(&mut self) -> f32 {
        self.seed = hash(self.seed);
        (self.seed & 0xffff) as f32 / 65535.0
    }

    /// True on the step a blizzard starts to build.
    pub fn step(&mut self, dt: f32, track: &Track) -> bool {
        self.t += dt;
        let mut began = false;
        if self.blizzards {
            self.spell = match self.spell {
                Spell::Calm(left) if left - dt <= 0.0 => {
                    let a = self.roll() * std::f32::consts::TAU;
                    self.wind = (a.cos(), a.sin());
                    began = true;
                    Spell::Building(0.0)
                }
                Spell::Calm(left) => Spell::Calm(left - dt),
                Spell::Building(t) if t + dt >= BUILD => {
                    let len = 16.0 + self.roll() * 10.0;
                    Spell::Blowing(len)
                }
                Spell::Building(t) => Spell::Building(t + dt),
                Spell::Blowing(left) if left - dt <= 0.0 => Spell::Easing(0.0),
                Spell::Blowing(left) => Spell::Blowing(left - dt),
                Spell::Easing(t) if t + dt >= BUILD => {
                    let calm = 40.0 + self.roll() * 30.0;
                    Spell::Calm(calm)
                }
                Spell::Easing(t) => Spell::Easing(t + dt),
            };
            self.blizzard = match self.spell {
                Spell::Calm(_) => 0.0,
                Spell::Building(t) => t / BUILD,
                Spell::Blowing(_) => 1.0,
                Spell::Easing(t) => 1.0 - t / BUILD,
            };
        }
        let n = track.len() as f32;
        for tw in &mut self.twisters {
            // drift back against the race, weaving across the road
            tw.pos = (tw.pos - 7.0 * dt).rem_euclid(n);
            tw.phase += dt * 0.5;
            tw.spin += dt * 9.0;
            let i = tw.pos as usize % track.len();
            let (px, py) = track.pts[i];
            let (tx, ty) = track.tangent(i);
            let side = tw.phase.sin() * (track.half - 2.0);
            tw.x = px - ty * side;
            tw.y = py + tx * side;
        }
        began
    }

    /// How far a gust pushes something this step.
    pub fn push(&self, dt: f32) -> (f32, f32) {
        let gust = 0.55 + 0.45 * (self.t * 1.7).sin() * (self.t * 0.6).sin().abs();
        let k = PUSH * self.blizzard * gust * dt;
        (self.wind.0 * k, self.wind.1 * k)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blizzard_builds_blows_and_eases() {
        let t = Track::frozen_lake();
        let mut w = Weather::new(&t, 7);
        let (mut began, mut peak, mut after) = (0, 0.0f32, false);
        for s in 0..(110.0 * 60.0) as usize {
            if w.step(1.0 / 60.0, &t) {
                began += 1;
            }
            peak = peak.max(w.blizzard);
            if peak >= 1.0 && w.blizzard == 0.0 && s > 0 {
                after = true;
            }
        }
        assert!(began >= 1, "a blizzard came");
        assert_eq!(peak, 1.0, "at full blow");
        assert!(after, "and calmed again");
    }

    #[test]
    fn calm_courses_stay_calm() {
        let t = Track::eggnogg();
        let mut w = Weather::new(&t, 7);
        for _ in 0..(120.0 * 60.0) as usize {
            assert!(!w.step(1.0 / 60.0, &t));
        }
        assert_eq!(w.blizzard, 0.0);
        assert!(w.twisters.is_empty());
    }

    #[test]
    fn a_twister_wanders_the_road_against_the_race() {
        let t = Track::frozen_lake();
        let mut w = Weather::new(&t, 7);
        let start = t.idx(w.twisters[0].x, w.twisters[0].y).unwrap();
        for _ in 0..600 {
            w.step(1.0 / 60.0, &t);
            let tw = w.twisters[0];
            assert!(t.dist(tw.x, tw.y) <= t.half, "on the road");
        }
        let now = t.idx(w.twisters[0].x, w.twisters[0].y).unwrap();
        assert!(now < start, "moving back round the lap");
    }
}
