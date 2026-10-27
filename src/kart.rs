//! Kart handling. Our own model, tuned by feel:
//!
//! - Speed closes in on top speed, quickly at first and then slower.
//! - Steering is weak when crawling, strongest at middling speed and a bit
//!   looser at top speed.
//! - Drift: hold drift while steering to hop into a slide. Steering into the
//!   turn tightens it and charges faster, steering out widens it. The charge
//!   climbs through three spark tiers (frost blue, gold, aurora); letting go
//!   turns the tier into a boost.
//! - Deep powder caps your speed unless you are boosting, and stops drift
//!   charging. Snowbanks bounce you back, pines stop you dead.
//! - Rocket start: the time the gas has been held when the lights go is
//!   judged. Hit the gas as the 1 appears.
//! - Hills: downhill is faster, uphill slower, by the same share for every
//!   ride. Where the ground falls away
//!   faster than the sleigh can drop (a crest, the lip of a kicker) it takes
//!   off; in the air there is no grip, gas or drift charge. Tap drift in the
//!   air for a trick, and land it for a boost.

use serde::{Deserialize, Serialize};

use crate::track::{Surface, Track};

pub const DT: f32 = 1.0 / 60.0;

/// Spark tier thresholds, in charge (about seconds of a neutral drift).
pub const TIERS: [f32; 3] = [1.0, 2.2, 3.6];
/// Boost time for releasing at each tier (0 = no sparks yet).
const TIER_BOOST: [f32; 4] = [0.0, 0.45, 0.95, 1.6];

const KART_R: f32 = 1.6;
/// Gravity, units per second squared.
pub const GRAVITY: f32 = 50.0;
/// How much a slope changes top speed: share per unit of rise per run.
const SLOPE_SHARE: f32 = 1.6;
const TREE_R: f32 = 1.4;
/// A drift's turn rate, as a share of the ride's turn: with the stick
/// centred, steered fully in, and steered fully out.
const DRIFT_TURN: f32 = 0.55;
const DRIFT_TIGHT: f32 = 1.2;
const DRIFT_WIDE: f32 = 0.24;

/// A drift's turn rate as a share of the ride's turn, for steering
/// `inward` (-1 fully out .. 1 fully in).
fn drift_share(inward: f32) -> f32 {
    if inward >= 0.0 {
        DRIFT_TURN + (DRIFT_TIGHT - DRIFT_TURN) * inward.min(1.0)
    } else {
        DRIFT_TURN + (DRIFT_TURN - DRIFT_WIDE) * inward.max(-1.0)
    }
}

/// How far in to steer for a drift turning at `share` of the ride's turn:
/// the inverse of `drift_share`.
pub fn drift_inward_for(share: f32) -> f32 {
    if share >= DRIFT_TURN {
        ((share - DRIFT_TURN) / (DRIFT_TIGHT - DRIFT_TURN)).min(1.0)
    } else {
        ((share - DRIFT_TURN) / (DRIFT_TURN - DRIFT_WIDE)).max(-1.0)
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Controls {
    /// -1 full left .. 1 full right.
    pub steer: f32,
    pub gas: bool,
    pub brake: bool,
    pub drift: bool,
    /// Use the item (an edge, true for one step).
    pub item: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub top: f32,
    pub accel: f32,
    /// Turn rate at its best, radians per second.
    pub turn: f32,
    /// How fast the direction of travel catches up with the nose.
    pub grip: f32,
    pub charge: f32,
    /// Who shoves whom in a bump.
    pub weight: f32,
    /// Flies low (Befana's broom): powder and cream hardly slow it. (Low
    /// enough to dip into a hole in the ice like anyone else, though.)
    pub hover: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Event {
    Hop,
    Tier(u8),
    Boost(u8),
    RocketStart,
    GoodStart,
    Stall,
    Bank,
    Tree,
    Hit,
    Launch,
    Trick,
    /// Touched down; a trick landed gives a boost.
    Land { trick: bool },
    /// Into a hole in the ice.
    Splash,
    /// Dropped into a crevasse.
    Crevasse,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kart {
    pub x: f32,
    pub y: f32,
    /// Where the nose points.
    pub heading: f32,
    /// Where the kart is actually travelling (lags the nose in a slide).
    pub travel: f32,
    pub speed: f32,
    pub stats: Stats,
    /// 0 none, -1 sliding left, 1 sliding right.
    pub drift: i8,
    pub charge: f32,
    /// Drift is being held but has not started a slide yet.
    drift_armed: bool,
    drift_was: bool,
    pub boost: f32,
    /// Hop height timer, counting down.
    pub hop: f32,
    /// Stunned (spun out or stalled), counting down.
    pub stun: f32,
    stun_len: f32,
    /// Whole turns a spin-out makes over the stun.
    spin_turns: f32,
    /// How far round a spin-out has turned the sleigh, radians.
    pub spin: f32,
    /// Since the last bump was reported, so pushing on a bank is one bump.
    knock: f32,
    pub surface: Surface,
    /// Santa's plough: goes through pines and can't be stopped.
    pub plough: bool,
    /// Distance travelled, for animation.
    pub odo: f32,
    /// Height of the runners, and how fast that is changing.
    pub z: f32,
    pub vz: f32,
    pub air: bool,
    /// A trick done in this jump; how far through its flip it is (0..1).
    pub trick: bool,
    pub flip: f32,
    /// Time the gas has been held during the countdown.
    pub start_hold: f32,
    pub events: Vec<Event>,
}

pub fn tier_of(charge: f32) -> u8 {
    TIERS.iter().filter(|&&t| charge >= t).count() as u8
}

fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    let a = a.rem_euclid(t);
    if a > std::f32::consts::PI {
        a - t
    } else {
        a
    }
}

impl Kart {
    pub fn new(x: f32, y: f32, heading: f32, stats: Stats) -> Kart {
        Kart {
            x,
            y,
            heading,
            travel: heading,
            speed: 0.0,
            stats,
            drift: 0,
            charge: 0.0,
            drift_armed: false,
            drift_was: false,
            boost: 0.0,
            hop: 0.0,
            stun: 0.0,
            stun_len: 0.0,
            spin_turns: 0.0,
            spin: 0.0,
            knock: 0.0,
            surface: Surface::Road,
            plough: false,
            odo: 0.0,
            z: 0.0,
            vz: 0.0,
            air: false,
            trick: false,
            flip: 0.0,
            start_hold: 0.0,
            events: Vec::new(),
        }
    }

    pub fn tier(&self) -> u8 {
        if self.drift == 0 {
            0
        } else {
            tier_of(self.charge)
        }
    }

    /// One step while the lights are still counting down.
    pub fn wait(&mut self, c: Controls) {
        self.events.clear();
        if c.gas {
            self.start_hold += DT;
        } else {
            self.start_hold = 0.0;
        }
    }

    /// The lights went: judge the start.
    pub fn go(&mut self) {
        let h = self.start_hold;
        if (0.75..=1.15).contains(&h) {
            self.boost = 1.4;
            self.speed = self.stats.top * 0.5;
            self.events.push(Event::RocketStart);
        } else if (0.45..0.75).contains(&h) || (1.15..1.6).contains(&h) {
            self.boost = 0.5;
            self.events.push(Event::GoodStart);
        } else if h > 2.1 {
            self.stun(0.9, 0.0);
            self.events.push(Event::Stall);
        }
        log::debug!("start: gas held {h:.2}s");
        self.start_hold = 0.0;
    }

    /// Sit on the ground where the kart is.
    pub fn settle(&mut self, track: &Track) {
        self.z = track.height(self.x, self.y);
        self.vz = 0.0;
        self.air = false;
    }

    pub fn step(&mut self, c: Controls, track: &Track) {
        self.events.clear();
        self.drive(c, track);
        self.vertical(track);
        if !self.air && !self.stunned() && !self.plough && track.in_hole(self.x, self.y) {
            // even a broom flies low enough to dip into open water
            self.splash(track, Event::Splash);
        } else if !self.plough && track.in_crevasse(self.x, self.y) && self.z < track.height(self.x, self.y) + crate::track::CREVASSE_DEPTH - 2.5 {
            // came at it too slow: dropped below the lip, and you're in
            self.splash(track, Event::Crevasse);
        }
    }

    /// Fallen through the ice or into a crevasse: hauled out a little way
    /// back, shivering, all speed gone.
    fn splash(&mut self, track: &Track, why: Event) {
        self.events.push(why);
        let n = track.len() as isize;
        if let Some(i) = track.idx(self.x, self.y) {
            // back along the road to a spot clear of holes; from a crevasse,
            // back past its kicker, with room to get up to speed again
            let crevasse = why == Event::Crevasse;
            let mut back = if crevasse { 100 } else { 10 };
            loop {
                let j = ((i as isize - back) % n + n) % n;
                let (px, py) = track.pts[j as usize];
                if !track.in_hole(px, py) && !track.in_crevasse(px, py) || back > if crevasse { 200 } else { 60 } {
                    self.x = px;
                    self.y = py;
                    self.heading = track.heading_at(j as usize);
                    break;
                }
                back += 5;
            }
        }
        self.travel = self.heading;
        self.speed = 0.0;
        self.end_drift(false);
        self.stun(1.3, 0.0);
        self.settle(track);
    }

    /// Follow the ground, or fly and fall.
    fn vertical(&mut self, track: &Track) {
        let ground = track.height(self.x, self.y);
        if self.air {
            self.vz -= GRAVITY * DT;
            self.z += self.vz * DT;
            if self.trick {
                self.flip = (self.flip + DT * 2.2).min(1.0);
            }
            if self.z <= ground {
                let trick = self.trick && self.flip >= 1.0;
                if trick {
                    self.boost = self.boost.max(0.8);
                } else if self.trick {
                    // landed mid-flip: a stumble
                    self.speed *= 0.6;
                } else if -self.vz > 30.0 {
                    self.speed *= 0.92;
                }
                self.events.push(Event::Land { trick });
                self.z = ground;
                self.vz = 0.0;
                self.air = false;
                self.trick = false;
                self.flip = 0.0;
            }
            return;
        }
        let follow = (ground - self.z) / DT;
        // the ground falls away faster than the sleigh can drop: take off
        if follow < self.vz - GRAVITY * DT - 6.0 && self.speed.abs() > 15.0 {
            self.air = true;
            self.vz -= GRAVITY * DT;
            self.z += self.vz * DT;
            self.events.push(Event::Launch);
        } else {
            self.vz = follow.clamp(-80.0, 80.0);
            self.z = ground;
        }
    }

    fn drive(&mut self, c: Controls, track: &Track) {
        let st = self.stats;
        self.surface = track.surface(self.x, self.y);
        // deep powder and wading through cream: slow, and no sparks
        let powder = matches!(self.surface, Surface::Powder | Surface::Cream);
        self.boost = (self.boost - DT).max(0.0);
        self.hop = (self.hop - DT).max(0.0);
        self.knock = (self.knock - DT).max(0.0);
        let stunned = self.stun > 0.0;
        if stunned {
            self.stun = (self.stun - DT).max(0.0);
            // ease out, landing facing forward again
            let t = 1.0 - self.stun / self.stun_len.max(DT);
            self.spin = std::f32::consts::TAU * self.spin_turns * (1.0 - (1.0 - t).powi(2));
            self.speed *= 1.0 - 2.5 * DT;
            self.end_drift(false);
        } else {
            self.spin = 0.0;
        }
        let c = if stunned { Controls::default() } else { c };

        // speed
        let boosting = self.boost > 0.0;
        let mut top = st.top;
        if powder && !boosting {
            top *= if st.hover { 0.85 } else { 0.55 };
        }
        if boosting {
            top *= 1.32;
        }
        let belt = self.surface == Surface::Conveyor && !self.air;
        if belt {
            // the belt carries you along
            top *= 1.22;
        }
        // a climb lowers everyone's top speed by the same share, a descent
        // raises it; how quickly you settle to it is your acceleration
        let slope = if self.air { 0.0 } else { track.slope(self.x, self.y, self.travel) * self.speed.signum().max(0.0) };
        top *= (1.0 - SLOPE_SHARE * slope).clamp(0.7, 1.3);
        if self.air {
            // no grip in the air: only a little drag
            self.speed -= self.speed * 0.05 * DT;
        } else if c.gas && !c.brake {
            if self.speed < top {
                let a = st.accel * if boosting { 3.0 } else if belt { 2.0 } else { 1.0 };
                self.speed += (top - self.speed) * a * DT;
            }
        } else if c.brake {
            if self.speed > 0.0 {
                self.speed = (self.speed - 70.0 * DT).max(0.0);
            } else {
                self.speed = (self.speed - 20.0 * DT).max(-14.0);
            }
        } else {
            let drag = if powder { 30.0 } else { 9.0 };
            self.speed -= self.speed.signum() * (drag * DT).min(self.speed.abs());
        }
        if self.speed > top && !self.air {
            // let go of speed above the cap gently, faster in powder
            let k = if powder { 2.5 } else { 1.2 };
            self.speed -= (self.speed - top) * k * DT;
        }

        // drift
        let pressed = c.drift && !self.drift_was;
        self.drift_was = c.drift;
        if pressed && self.air {
            // a trick, once a jump
            if !self.trick {
                self.trick = true;
                self.events.push(Event::Trick);
            }
        } else if pressed {
            self.drift_armed = true;
            if self.hop == 0.0 && self.speed > 8.0 {
                self.hop = 0.2;
                self.events.push(Event::Hop);
            }
        }
        if !c.drift {
            self.drift_armed = false;
            if self.drift != 0 {
                self.end_drift(true);
            }
        }
        if self.drift_armed && self.drift == 0 && !self.air && c.steer.abs() > 0.3 && self.speed > 22.0 {
            self.drift = if c.steer > 0.0 { 1 } else { -1 };
            self.drift_armed = false;
            self.charge = 0.0;
        }
        if self.drift != 0 && self.speed < 16.0 {
            self.end_drift(false);
        }

        // turning
        let s = self.speed.abs();
        let reach = (s / 16.0).min(1.0) * (1.0 - 0.3 * (s / st.top).min(1.3));
        let rate = if self.drift != 0 {
            let d = self.drift as f32;
            let inward = c.steer * d;
            if (!powder || st.hover) && !self.air {
                let before = tier_of(self.charge);
                self.charge += DT * st.charge * (1.0 + 0.5 * inward.max(0.0));
                let after = tier_of(self.charge);
                if after > before {
                    self.events.push(Event::Tier(after));
                }
            }
            // a gentle arc with the stick centred, tightened by steering in
            // and widened by steering out; the tightest and widest are as
            // before (David, 2026-10-27: centred, it turned as hard as full
            // lock, too quick to hold on a gamepad)
            d * st.turn * drift_share(inward)
        } else {
            let sharper = if boosting { 1.15 } else { 1.0 };
            c.steer * st.turn * reach * sharper * self.speed.signum()
        };
        // a little air control, no more
        let rate = if self.air { rate * 0.3 } else { rate };
        self.heading = wrap(self.heading + rate * DT);

        // travel direction trails the nose; in a slide it trails a lot
        let ice = self.surface == Surface::Ice;
        let grip = if self.air {
            0.0
        } else if self.surface == Surface::BlackIce {
            st.grip * 0.35
        } else if ice {
            st.grip * 0.1
        } else if self.drift != 0 {
            st.grip * 0.3
        } else if powder {
            st.grip * 0.6
        } else {
            st.grip
        };
        let lag = wrap(self.heading - self.travel);
        self.travel = wrap(self.travel + lag * (grip * DT).min(1.0));

        // move and collide
        let (nx, ny) = (self.x + self.travel.cos() * self.speed * DT, self.y + self.travel.sin() * self.speed * DT);
        if track.surface(nx, ny) == Surface::Bank {
            // the bank faces back toward the centreline
            if let Some(i) = track.idx(self.x, self.y) {
                let (cx, cy) = track.pts[i];
                let (ax, ay) = (cx - self.x, cy - self.y);
                let l = ax.hypot(ay).max(1e-3);
                self.deflect(ax / l, ay / l, Event::Bank);
            } else {
                self.speed = 0.0;
            }
            return;
        }
        // a cottage in the road: over its roof, or into its wall. Already
        // inside it can only mean coming down on the roof just short of
        // clearing it: slide off to the nearest side, a hard landing, or
        // every step after would be inside too and the sleigh stuck there
        if let Some((ox, oy)) = track.out_of_cottage(self.x, self.y, self.z, KART_R) {
            self.x = ox;
            self.y = oy;
            self.speed *= 0.5;
            if !self.stunned() {
                self.stun(0.9, 1.0);
                self.events.push(Event::Tree);
            }
            return;
        }
        if let Some((cx, cy)) = track.cottage_at(nx, ny, self.z, KART_R) {
            let (ax, ay) = (nx - cx, ny - cy);
            let l = ax.hypot(ay).max(1e-3);
            self.deflect(ax / l, ay / l, Event::Tree);
            if self.speed.abs() > 20.0 && !self.stunned() {
                self.stun(0.9, 1.0);
            }
            return;
        }
        // pines are taller than any jump, but a sleigh well up clears the
        // little ones
        let up = self.z - track.height(nx, ny);
        for t in track.trees.iter().filter(|t| !self.plough && up < t.size * 6.0) {
            let r = KART_R + TREE_R * t.size;
            if (t.x - nx).abs() < r && (t.y - ny).abs() < r && (t.x - nx).hypot(t.y - ny) < r {
                let (ax, ay) = (nx - t.x, ny - t.y);
                let l = ax.hypot(ay).max(1e-3);
                let (ux, uy) = (ax / l, ay / l);
                // stand off the trunk, then glance away from it
                self.x = t.x + ux * (r + 0.05);
                self.y = t.y + uy * (r + 0.05);
                self.deflect(ux, uy, Event::Tree);
                return;
            }
        }
        self.x = nx;
        self.y = ny;
        self.odo += self.speed.abs() * DT;
    }

    /// Hit something whose surface faces along (`nx`, `ny`): a hard hit
    /// spins the sleigh out, a glancing or slow one slides it along.
    fn deflect(&mut self, nx: f32, ny: f32, what: Event) {
        let (vx, vy) = (self.travel.cos() * self.speed, self.travel.sin() * self.speed);
        let into = -(vx * nx + vy * ny);
        if into <= 0.0 {
            return;
        }
        let (tx, ty) = (vx + into * nx, vy + into * ny);
        if into > 6.0 {
            if self.knock == 0.0 {
                self.events.push(what);
            }
            self.knock = 0.4;
            self.end_drift(false);
        }
        let crash = into > 28.0 && what == Event::Tree;
        if crash {
            self.stun(0.9, 2.0);
        }
        // keep what runs along the surface, less some scrub, plus a small
        // push off it so the sleigh doesn't grind
        let keep = if crash { 0.15 } else if into > 6.0 { 0.75 } else { 0.95 };
        let (wx, wy) = (tx * keep + nx * into * 0.3, ty * keep + ny * into * 0.3);
        let sp = wx.hypot(wy);
        if sp > 0.01 {
            self.travel = wy.atan2(wx);
            if self.speed < 0.0 {
                self.travel = wrap(self.travel + std::f32::consts::PI);
            }
        }
        self.speed = sp * self.speed.signum();
    }

    /// Struck by something: spin out for `secs`, turning `turns` times.
    pub fn hit(&mut self, secs: f32, turns: f32) {
        if self.plough {
            return;
        }
        self.stun(secs, turns);
        self.speed *= 0.35;
        self.end_drift(false);
        self.events.push(Event::Hit);
    }

    pub fn stunned(&self) -> bool {
        self.stun > 0.0
    }

    fn stun(&mut self, secs: f32, turns: f32) {
        self.stun = secs;
        self.stun_len = secs;
        self.spin_turns = turns;
    }

    fn end_drift(&mut self, release: bool) {
        if self.drift == 0 {
            return;
        }
        let t = tier_of(self.charge);
        if release && t > 0 {
            self.boost = self.boost.max(TIER_BOOST[t as usize]);
            self.events.push(Event::Boost(t));
        }
        // the nose swings back toward the line of travel
        self.drift = 0;
        self.charge = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kart_on_straight() -> (Kart, Track) {
        // a ring of wide road, so a long slide stays on the road
        let t = Track::ring(70.0);
        let i = 60;
        let (x, y) = t.pts[i];
        (Kart::new(x, y, t.heading_at(i), crate::roster::ROSTER[0].stats), t)
    }

    fn gas() -> Controls {
        Controls { gas: true, ..Default::default() }
    }

    #[test]
    fn accelerates_toward_top_speed() {
        let (mut k, t) = kart_on_straight();
        for _ in 0..60 {
            k.step(gas(), &t);
        }
        assert!(k.speed > crate::roster::ROSTER[0].stats.top * 0.6 && k.speed < crate::roster::ROSTER[0].stats.top, "{}", k.speed);
    }

    #[test]
    fn drift_climbs_tiers_and_releases_a_boost() {
        let (mut k, t) = kart_on_straight();
        k.speed = 50.0;
        let mut c = Controls { gas: true, steer: 1.0, drift: true, ..Default::default() };
        let mut tiers = Vec::new();
        for _ in 0..(1.6 / DT) as usize {
            k.step(c, &t);
            if k.surface == Surface::Bank {
                break;
            }
            tiers.extend(k.events.iter().filter_map(|e| if let Event::Tier(n) = e { Some(*n) } else { None }));
        }
        assert_eq!(k.drift, 1);
        assert_eq!(tiers, vec![1, 2], "full inward steering reaches tier 2, not 3, in 1.6 s");
        c.drift = false;
        k.step(c, &t);
        assert!(k.events.contains(&Event::Boost(2)));
        assert!(k.boost > 0.9);
    }

    #[test]
    fn a_pine_head_on_spins_you_out_once_then_lets_you_go() {
        let (mut k, mut t) = kart_on_straight();
        t.trees.clear();
        let (hx, hy) = (k.heading.cos(), k.heading.sin());
        t.trees.push(crate::track::Tree { x: k.x + hx * 20.0, y: k.y + hy * 20.0 + 0.3, size: 1.0 });
        k.speed = 50.0;
        let mut hits = 0;
        for _ in 0..(6.0 / DT) as usize {
            k.step(gas(), &t);
            hits += k.events.iter().filter(|e| **e == Event::Tree).count();
        }
        assert_eq!(hits, 1, "one crash, not a pin");
        assert!(k.speed > 20.0, "drove on round it: speed {}", k.speed);
    }

    #[test]
    fn the_kicker_on_the_crest_throws_you_and_you_land() {
        let mut t = Track::frozen_lake();
        t.trees.clear();
        let n = t.len();
        let lip = (0.125 * n as f32) as usize;
        let i = lip - 60;
        let (x, y) = t.pts[i];
        let mut k = Kart::new(x, y, t.heading_at(i), crate::roster::ROSTER[0].stats);
        k.settle(&t);
        k.speed = 62.0;
        let (mut launched, mut landed, mut peak) = (false, false, 0.0f32);
        for _ in 0..(3.0 / DT) as usize {
            // steer at the centreline so the run is straight
            let j = t.idx(k.x, k.y).unwrap();
            let (cx, cy) = t.pts[t.ahead(j, 15)];
            let err = wrap((cy - k.y).atan2(cx - k.x) - k.heading);
            k.step(Controls { gas: true, steer: (err * 3.0).clamp(-1.0, 1.0), ..Default::default() }, &t);
            launched |= k.events.contains(&Event::Launch);
            landed |= k.events.iter().any(|e| matches!(e, Event::Land { .. }));
            if k.air {
                peak = peak.max(k.z - t.height(k.x, k.y));
            }
        }
        assert!(launched && landed, "launched {launched} landed {landed}");
        assert!(peak > 2.0, "real air: {peak}");
    }

    #[test]
    fn a_trick_in_the_air_lands_as_a_boost() {
        let mut t = Track::ring(70.0);
        t.trees.clear();
        let (x, y) = t.pts[60];
        let mut k = Kart::new(x, y, t.heading_at(60), crate::roster::ROSTER[0].stats);
        k.settle(&t);
        k.speed = 50.0;
        // throw it up by hand
        k.air = true;
        k.vz = 18.0;
        k.step(Controls { drift: true, ..Default::default() }, &t);
        assert!(k.events.contains(&Event::Trick));
        let mut boost = false;
        for _ in 0..120 {
            k.step(Controls::default(), &t);
            boost |= k.events.contains(&Event::Land { trick: true });
        }
        assert!(boost);
        assert!(k.boost > 0.0 || boost);
    }

    #[test]
    fn a_hole_in_the_ice_means_a_splash_and_a_haul_out() {
        let t = Track::thin_ice();
        let (hx, hy, _) = t.holes[2];
        let mut k = Kart::new(hx, hy, 0.0, crate::roster::ROSTER[0].stats);
        k.settle(&t);
        k.speed = 40.0;
        k.step(Controls { gas: true, ..Default::default() }, &t);
        assert!(k.events.contains(&Event::Splash));
        assert_eq!(k.speed, 0.0);
        assert!(!t.in_hole(k.x, k.y), "hauled out");
        assert!(k.stunned());
    }

    /// A kart `back` samples before the first crevasse, pointing along the
    /// road at `speed`, run for up to `secs`: whether it fell in, and how
    /// far round the lap it got.
    fn at_the_crevasse(back: usize, speed: f32, gas: bool, secs: f32) -> (bool, usize) {
        let t = Track::glacier();
        let (s0, len) = t.crevasses[0];
        let i = s0 - back;
        let (x, y) = t.pts[i];
        let mut k = Kart::new(x, y, t.heading_at(i), crate::roster::ROSTER[0].stats);
        k.settle(&t);
        k.speed = speed;
        let mut fell = false;
        let mut furthest = i;
        for _ in 0..(secs / DT) as usize {
            // stay on the line
            let j = t.idx(k.x, k.y).unwrap_or(i);
            let off = wrap(t.heading_at(j) - k.heading);
            k.step(Controls { gas, steer: (off * 3.0).clamp(-1.0, 1.0), ..Default::default() }, &t);
            fell |= k.events.contains(&Event::Crevasse);
            if !fell {
                furthest = furthest.max(t.idx(k.x, k.y).unwrap_or(0));
            }
            if fell {
                assert!(!t.in_crevasse(k.x, k.y), "hauled out");
                let at = t.idx(k.x, k.y).unwrap();
                assert!(at + 80 < s0, "back before its kicker ({at})");
                // from there, flat out, you clear it
                for _ in 0..(6.0 / DT) as usize {
                    let j = t.idx(k.x, k.y).unwrap_or(at);
                    let off = wrap(t.heading_at(j) - k.heading);
                    k.step(Controls { gas: true, steer: (off * 3.0).clamp(-1.0, 1.0), ..Default::default() }, &t);
                    assert!(!k.events.contains(&Event::Crevasse), "fell in again");
                }
                assert!(t.idx(k.x, k.y).unwrap() > s0 + len, "cleared it second time");
                break;
            }
        }
        (fell, furthest)
    }

    #[test]
    fn at_speed_you_clear_a_crevasse_slow_you_fall_in() {
        let t = Track::glacier();
        let (s0, len) = t.crevasses[0];
        let (fell, got) = at_the_crevasse(60, 62.0, true, 3.0);
        assert!(!fell, "flew over it");
        assert!(got > s0 + len + 20, "and on down the road ({got})");
        let (fell, _) = at_the_crevasse(8, 10.0, true, 4.0);
        assert!(fell, "crept in");
    }

    #[test]
    fn fly_over_the_cottage_or_into_its_wall() {
        let mut t = Track::gingerbread_village();
        t.trees.clear();
        let n = t.len();
        let lip = (0.33 * n as f32) as usize;
        let run = |speed: f32, from: usize| -> (bool, bool) {
            let i = from;
            let (x, y) = t.pts[i];
            let mut k = Kart::new(x, y, t.heading_at(i), crate::roster::ROSTER[0].stats);
            k.settle(&t);
            k.speed = speed;
            let (mut crashed, mut past) = (false, false);
            for _ in 0..(2.5 / DT) as usize {
                let j = t.idx(k.x, k.y).unwrap();
                let (cx, cy) = t.pts[t.ahead(j, 12)];
                let err = wrap((cy - k.y).atan2(cx - k.x) - k.heading);
                k.step(Controls { gas: true, steer: (err * 3.0).clamp(-1.0, 1.0), ..Default::default() }, &t);
                crashed |= k.events.contains(&Event::Tree);
                past |= t.idx(k.x, k.y).unwrap() > lip + 40;
            }
            (crashed, past)
        };
        assert_eq!(run(64.0, lip - 50), (false, true), "at speed: over the roof");
        // rolling down off the lip slowly: straight into the wall
        let (crashed, _) = run(12.0, lip + 3);
        assert!(crashed, "crawling: into the wall");
    }

    #[test]
    fn start_judging() {
        let (mut k, _) = kart_on_straight();
        for (held, want) in [(1.0, Some(Event::RocketStart)), (0.6, Some(Event::GoodStart)), (0.2, None), (2.5, Some(Event::Stall))] {
            k.events.clear();
            k.boost = 0.0;
            k.stun = 0.0;
            k.start_hold = held;
            k.go();
            assert_eq!(k.events.first().copied(), want, "held {held}");
        }
    }
}
