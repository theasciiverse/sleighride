//! Wildlife at the penguin crossing.
//!
//! - Every so often a line of penguins waddles across the road in single
//!   file. Plough into one and it's bowled into a belly-slide away across the
//!   snow (penguins like that; nobody gets hurt), and you lose some speed.
//! - A polar bear lives by the crossing. It ambles about, and when the
//!   penguins cross it lumbers after them, never quite fast enough. Race past
//!   too close and it rears up and swipes you into a spin.

use serde::{Deserialize, Serialize};

use crate::kart::Kart;
use crate::track::Track;

const PENGUINS: usize = 7;
const WADDLE: f32 = 3.0;
/// Seconds before the first crossing, then between them.
const FIRST: f32 = 10.0;
const EVERY: f32 = 24.0;
const BUMP_R: f32 = 1.8;
/// The bear: how fast it lumbers, how close it notices you, how far it
/// reaches.
const BEAR_SPEED: f32 = 2.6;
const BEAR_NOTICE: f32 = 8.0;
const BEAR_REACH: f32 = 5.5;
const REAR: f32 = 0.6;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Gait {
    Waddling,
    /// Bowled over: sliding on its belly, seconds since.
    Sliding(f32),
    Gone,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Penguin {
    pub x: f32,
    pub y: f32,
    vx: f32,
    vy: f32,
    pub heading: f32,
    pub gait: Gait,
    /// Waddle, cycles.
    pub step: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Bear {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    home: (f32, f32),
    /// Rearing up to swipe: seconds left, 0 when on all fours.
    pub rear: f32,
    cool: f32,
    /// Walk cycle.
    pub step: f32,
}

/// What happened, for the race to pass on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Wild {
    /// Racer bowled a penguin.
    Bowled(usize),
    /// Racer swiped by the bear.
    Swiped(usize),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Wildlife {
    pub penguins: Vec<Penguin>,
    pub bear: Option<Bear>,
    /// Where the line crosses: a point on the centreline and the direction
    /// across the road.
    crossing: Option<((f32, f32), (f32, f32))>,
    half: f32,
    timer: f32,
    side: f32,
}

impl Wildlife {
    pub fn new(track: &Track) -> Wildlife {
        let crossing = track.penguins.map(|f| {
            let i = (f * track.len() as f32) as usize;
            let (tx, ty) = track.tangent(i);
            (track.pts[i], (-ty, tx))
        });
        let bear = crossing.map(|((cx, cy), (ax, ay))| {
            let home = (cx + ax * (track.half + 15.0) + ay * 10.0, cy + ay * (track.half + 15.0) - ax * 10.0);
            Bear { x: home.0, y: home.1, heading: 0.0, home, rear: 0.0, cool: 0.0, step: 0.0 }
        });
        Wildlife { penguins: Vec::new(), bear, crossing, half: track.half, timer: EVERY - FIRST, side: 1.0 }
    }

    pub fn step(&mut self, dt: f32, karts: &mut [&mut Kart]) -> Vec<Wild> {
        let mut out = Vec::new();
        let Some(((cx, cy), (ax, ay))) = self.crossing else { return out };
        // a new line sets off now and then, from alternate sides
        self.timer += dt;
        if self.timer >= EVERY && self.penguins.iter().all(|p| p.gait == Gait::Gone) {
            self.timer = 0.0;
            self.side = -self.side;
            let reach = self.half + 16.0;
            let (dx, dy) = (-ax * self.side, -ay * self.side);
            self.penguins = (0..PENGUINS)
                .map(|k| {
                    let back = reach + k as f32 * 2.4;
                    Penguin { x: cx - dx * back, y: cy - dy * back, vx: dx * WADDLE, vy: dy * WADDLE, heading: dy.atan2(dx), gait: Gait::Waddling, step: k as f32 * 0.3 }
                })
                .collect();
        }
        let far = self.half + 18.0;
        for p in &mut self.penguins {
            match p.gait {
                Gait::Waddling => {
                    p.x += p.vx * dt;
                    p.y += p.vy * dt;
                    p.step += dt * 2.5;
                    // off the far side: into the snow and gone
                    let across = (p.x - cx) * ax * self.side * -1.0 + (p.y - cy) * ay * self.side * -1.0;
                    if across > far {
                        p.gait = Gait::Gone;
                    }
                }
                Gait::Sliding(t) => {
                    p.x += p.vx * dt;
                    p.y += p.vy * dt;
                    let f = (1.0 - 1.5 * dt).max(0.0);
                    p.vx *= f;
                    p.vy *= f;
                    p.gait = if t + dt > 2.5 { Gait::Gone } else { Gait::Sliding(t + dt) };
                }
                Gait::Gone => {}
            }
        }
        // karts bowl penguins over
        for (i, k) in karts.iter_mut().enumerate() {
            if k.air {
                continue;
            }
            for p in &mut self.penguins {
                if p.gait == Gait::Waddling && (p.x - k.x).hypot(p.y - k.y) < BUMP_R {
                    let (kx, ky) = (k.travel.cos() * k.speed, k.travel.sin() * k.speed);
                    let (ox, oy) = (p.x - k.x, p.y - k.y);
                    let l = ox.hypot(oy).max(0.1);
                    p.vx = kx * 0.6 + ox / l * 12.0;
                    p.vy = ky * 0.6 + oy / l * 12.0;
                    p.heading = p.vy.atan2(p.vx);
                    p.gait = Gait::Sliding(0.0);
                    k.speed *= 0.72;
                    out.push(Wild::Bowled(i));
                }
            }
        }
        // the bear
        if let Some(b) = &mut self.bear {
            b.cool = (b.cool - dt).max(0.0);
            if b.rear > 0.0 {
                let before = b.rear;
                b.rear = (b.rear - dt).max(0.0);
                // the swipe lands halfway through rearing
                if before > REAR / 2.0 && b.rear <= REAR / 2.0 {
                    for (i, k) in karts.iter_mut().enumerate() {
                        if !k.stunned() && !k.air && (k.x - b.x).hypot(k.y - b.y) < BEAR_REACH {
                            k.hit(0.9, 1.0);
                            out.push(Wild::Swiped(i));
                        }
                    }
                }
            } else {
                // after the nearest waddling penguin, else home
                let target = self
                    .penguins
                    .iter()
                    .filter(|p| p.gait == Gait::Waddling)
                    .min_by(|a, c| (a.x - b.x).hypot(a.y - b.y).total_cmp(&(c.x - b.x).hypot(c.y - b.y)))
                    .map(|p| (p.x, p.y))
                    .unwrap_or(b.home);
                let (dx, dy) = (target.0 - b.x, target.1 - b.y);
                let d = dx.hypot(dy);
                let chasing = target != b.home;
                // never quite catching one: it trails at a distance, and
                // backs off when a penguin waddles toward it
                let (go, sp) = if chasing && d < 4.0 {
                    (-1.0, BEAR_SPEED * 1.4)
                } else if chasing && d < 5.0 {
                    (0.0, 0.0)
                } else if d > 1.5 {
                    (1.0, if chasing { BEAR_SPEED } else { BEAR_SPEED * 0.6 })
                } else {
                    (0.0, 0.0)
                };
                if go != 0.0 {
                    b.x += dx / d * sp * dt * go;
                    b.y += dy / d * sp * dt * go;
                    b.heading = (dy * go).atan2(dx * go);
                    b.step += dt * 1.6;
                } else if chasing {
                    b.heading = dy.atan2(dx);
                }
                if b.cool == 0.0 && karts.iter().any(|k| !k.air && (k.x - b.x).hypot(k.y - b.y) < BEAR_NOTICE) {
                    b.rear = REAR;
                    b.cool = 2.5;
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kart::DT;

    fn quiet(t: &Track) -> Kart {
        let mut k = Kart::new(-500.0, -500.0, 0.0, crate::roster::ROSTER[0].stats);
        k.z = t.lake;
        k
    }

    #[test]
    fn penguins_cross_in_single_file_and_the_bear_never_catches_one() {
        let t = Track::thin_ice();
        let mut w = Wildlife::new(&t);
        let mut k = quiet(&t);
        let (mut crossed, mut closest) = (false, f32::MAX);
        for _ in 0..(60.0 / DT) as usize {
            w.step(DT, &mut [&mut k]);
            if let Some(b) = &w.bear {
                for p in w.penguins.iter().filter(|p| p.gait == Gait::Waddling) {
                    closest = closest.min((p.x - b.x).hypot(p.y - b.y));
                }
            }
            if !w.penguins.is_empty() && w.penguins.iter().all(|p| p.gait == Gait::Gone) {
                crossed = true;
            }
        }
        assert!(crossed, "a line crossed and went");
        assert!(closest > 1.5, "the bear never caught one: {closest}");
    }

    #[test]
    fn a_penguin_in_the_way_is_bowled_and_slows_you() {
        let t = Track::thin_ice();
        let mut w = Wildlife::new(&t);
        let mut k = quiet(&t);
        // wait for the line to be on the road, then drive into the lead bird
        while w.penguins.is_empty() {
            w.step(DT, &mut [&mut k]);
        }
        for _ in 0..240 {
            w.step(DT, &mut [&mut k]);
        }
        let p = w.penguins[0];
        let mut k = Kart::new(p.x - 1.0, p.y, 0.0, crate::roster::ROSTER[0].stats);
        k.speed = 50.0;
        let hit = w.step(DT, &mut [&mut k]);
        assert_eq!(hit, vec![Wild::Bowled(0)]);
        assert!(k.speed < 40.0);
        assert!(matches!(w.penguins[0].gait, Gait::Sliding(_)));
    }

    #[test]
    fn the_bear_swipes_whoever_comes_close_then_waits() {
        let t = Track::thin_ice();
        let mut w = Wildlife::new(&t);
        let b = w.bear.unwrap();
        let mut k = Kart::new(b.x + 3.0, b.y, 0.0, crate::roster::ROSTER[0].stats);
        let mut swipes = 0;
        for _ in 0..(1.0 / DT) as usize {
            // hold the kart in place
            k.x = w.bear.unwrap().x + 3.0;
            k.y = w.bear.unwrap().y;
            swipes += w.step(DT, &mut [&mut k]).iter().filter(|e| matches!(e, Wild::Swiped(0))).count();
        }
        assert_eq!(swipes, 1, "one swipe, then the bear cools off");
        assert!(k.stunned() || k.spin > 0.0 || swipes == 1);
    }
}
