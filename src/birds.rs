//! Birds, for life in the scenery; they never get in anyone's way.
//!
//! - Crows sit on some of the pines by the road. When a racer comes close
//!   they flap off, climbing and veering away, and come back to the same
//!   perch a while later.
//! - A flock crosses the sky ahead every so often, each bird bobbing in a
//!   loose V; between passes it is gone over the hills.

use crate::track::Track;

const SCARE_R: f32 = 24.0;
/// How long a crow is gone once scared off.
const AWAY: f32 = 22.0;
const FLOCK: usize = 11;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Perch {
    Sitting,
    /// Flying off, seconds since take-off.
    Flying(f32),
    /// Out of sight, seconds left until back.
    Away(f32),
}

#[derive(Clone, Copy, Debug)]
pub struct Crow {
    pub home: (f32, f32, f32),
    pub x: f32,
    pub y: f32,
    pub z: f32,
    vx: f32,
    vy: f32,
    pub state: Perch,
    /// Wing beat, cycles.
    pub flap: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Flyer {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub flap: f32,
}

pub struct Birds {
    pub crows: Vec<Crow>,
    pub flock: Vec<Flyer>,
    /// The flock's pass: where it set off, its heading and height, and how
    /// long it has been flying (it crosses for PASS seconds, then rests).
    pass: (f32, f32, f32, f32),
    pass_t: f32,
    passes: u32,
    t: f32,
}

/// Seconds a flock takes to cross, and between the start of one pass and
/// the next.
const PASS: f32 = 16.0;
const EVERY: f32 = 30.0;

fn hash(n: u32) -> u32 {
    let mut h = n.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^ (h >> 13)
}

impl Birds {
    pub fn new(track: &Track) -> Birds {
        // crows on every so often pine close to the road
        let mut crows = Vec::new();
        for (i, t) in track.trees.iter().enumerate().filter(|_| !track.theme.indoors) {
            let d = track.dist(t.x, t.y);
            if d < track.half + 16.0 && hash(i as u32) % 5 == 0 && crows.len() < 30 {
                let z = track.height(t.x, t.y) + 11.0 * t.size * 0.92;
                crows.push(Crow { home: (t.x, t.y, z), x: t.x, y: t.y, z, vx: 0.0, vy: 0.0, state: Perch::Sitting, flap: 0.0 });
            }
        }
        // the first flock comes over soon after the start
        // no flock indoors
        let pass_t = if track.theme.indoors { f32::NEG_INFINITY } else { EVERY - 6.0 };
        Birds { crows, flock: Vec::new(), pass: (0.0, 0.0, 0.0, 0.0), pass_t, passes: 0, t: 0.0 }
    }

    /// One step: `racers` are where everyone is; `view` is the camera's
    /// position, heading and ground height, so the flock crosses in sight.
    pub fn step(&mut self, dt: f32, racers: &[(f32, f32)], view: (f32, f32, f32, f32)) {
        self.t += dt;
        for c in &mut self.crows {
            match c.state {
                Perch::Sitting => {
                    if let Some(&(rx, ry)) = racers.iter().find(|&&(rx, ry)| (rx - c.x).hypot(ry - c.y) < SCARE_R) {
                        // away from whoever came close, with a slant of its own
                        let (dx, dy) = (c.x - rx, c.y - ry);
                        let l = dx.hypot(dy).max(0.1);
                        let slant = ((c.home.0 * 7.0 + c.home.1).sin()) * 0.8;
                        let (ux, uy) = (dx / l, dy / l);
                        c.vx = (ux - uy * slant) * 16.0;
                        c.vy = (uy + ux * slant) * 16.0;
                        c.state = Perch::Flying(0.0);
                    }
                }
                Perch::Flying(t) => {
                    c.x += c.vx * dt;
                    c.y += c.vy * dt;
                    // climbing hard at first, then levelling off
                    c.z += (14.0 * (-t * 0.8).exp() + 2.0) * dt;
                    c.flap += dt * if t < 1.0 { 7.0 } else { 4.5 };
                    c.state = if t + dt > 5.0 { Perch::Away(AWAY) } else { Perch::Flying(t + dt) };
                }
                Perch::Away(left) => {
                    if left - dt <= 0.0 {
                        c.x = c.home.0;
                        c.y = c.home.1;
                        c.z = c.home.2;
                        c.state = Perch::Sitting;
                    } else {
                        c.state = Perch::Away(left - dt);
                    }
                }
            }
        }
        // the flock: now and then it drifts across the sky ahead. It is
        // far off and high, so like distant birds it seems to keep pace
        // with you and only slowly cross the view: it flies in the camera's
        // frame, not the world's
        self.pass_t += dt;
        if self.pass_t >= EVERY {
            self.pass_t = 0.0;
            self.passes += 1;
            let side = if hash(self.passes) % 2 == 0 { 1.0 } else { -1.0 };
            let ahead = 120.0 + (hash(self.passes + 7) % 50) as f32;
            let height = 30.0 + (hash(self.passes + 5) % 22) as f32;
            let tilt = ((hash(self.passes + 3) % 100) as f32 / 100.0 - 0.5) * 0.5;
            self.pass = (side, ahead, tilt, height);
        }
        self.flock.clear();
        if self.pass_t < PASS && self.passes > 0 {
            let (side, ahead, tilt, height) = self.pass;
            let (vx, vy, va, vz) = view;
            let (fx, fy) = (va.cos(), va.sin());
            let (rx, ry) = (-fy, fx);
            // across from one side to the other over the pass
            let across = side * (-1.0 + 2.0 * self.pass_t / PASS) * ahead * 0.85;
            let dist = ahead + across * tilt;
            for k in 0..FLOCK {
                let hk = hash(k as u32 + 99);
                // a loose V, pointing the way it flies
                let rank = ((k + 1) / 2) as f32;
                let wing = if k % 2 == 0 { 1.0 } else { -1.0 } * rank;
                let back = rank * 4.0 + (hk % 7) as f32 * 0.4;
                let a = across - side * back + (self.t * 1.3 + k as f32).sin() * 1.2;
                let d = dist + wing * 4.0;
                self.flock.push(Flyer {
                    x: vx + fx * d + rx * a,
                    y: vy + fy * d + ry * a,
                    z: vz + height + (self.t * 0.9 + k as f32 * 0.7).sin() * 1.5,
                    flap: self.t * 3.5 + (hk % 100) as f32 / 100.0,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crows_sit_on_pines_and_fly_off_when_you_come_close() {
        let t = Track::frozen_lake();
        let mut b = Birds::new(&t);
        assert!(b.crows.len() >= 10, "{} crows", b.crows.len());
        let c = b.crows[0];
        // nobody near: they stay put
        let far = (0.0, 0.0, 0.0, 0.0);
        b.step(0.1, &[(-500.0, -500.0)], far);
        assert_eq!(b.crows[0].state, Perch::Sitting);
        // a racer right below: off it goes, up and away
        for _ in 0..30 {
            b.step(1.0 / 60.0, &[(c.home.0 + 5.0, c.home.1)], far);
        }
        let flown = b.crows[0];
        assert!(matches!(flown.state, Perch::Flying(_)));
        assert!(flown.z > c.home.2 + 2.0, "climbed");
        assert!(flown.x < c.home.0, "away from the racer");
        // later it is back on its perch
        for _ in 0..(40.0 * 60.0) as usize {
            b.step(1.0 / 60.0, &[(-500.0, -500.0)], far);
        }
        assert_eq!(b.crows[0].state, Perch::Sitting);
        assert_eq!((b.crows[0].x, b.crows[0].y), (c.home.0, c.home.1));
    }

    #[test]
    fn a_flock_crosses_the_sky_ahead_now_and_then() {
        let t = Track::frozen_lake();
        let mut b = Birds::new(&t);
        let view = (500.0, 500.0, 0.3, 10.0);
        let (mut seen, mut gone) = (false, false);
        for _ in 0..(70.0 * 60.0) as usize {
            b.step(1.0 / 60.0, &[], view);
            if b.flock.is_empty() {
                gone = true;
                continue;
            }
            seen = true;
            assert_eq!(b.flock.len(), FLOCK);
            for f in &b.flock {
                assert!(f.z > view.3 + 20.0, "up in the sky");
                // somewhere in front of the camera, not behind it
                let ahead = (f.x - view.0) * view.2.cos() + (f.y - view.1) * view.2.sin();
                assert!(ahead > 40.0, "ahead: {ahead}");
            }
        }
        assert!(seen && gone, "passes come and go");
    }
}
