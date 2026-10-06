//! Autopilot: the rival helpers, the title-screen race, the lap after the
//! finish, and Santa's hunting.
//!
//! Each rival keeps to its own side of the road (so the field spreads out),
//! drifts the long bends, starts on the 1 give or take its nerves, and
//! uses items when they're likely to do something.

use crate::kart::Controls;
use crate::race::{Item, Race, COUNTDOWN};

#[derive(Clone, Copy, Debug)]
pub struct Ai {
    /// Preferred distance right of the centreline.
    pub offset: f32,
    /// How far off a perfect start this one is, seconds.
    pub start_err: f32,
}

impl Ai {
    pub fn new(who: usize, seed: u32) -> Ai {
        let h = (who as u32 + 1).wrapping_mul(0x9E37_79B9) ^ seed.wrapping_mul(0x85EB_CA6B);
        let r = |k: u32| ((h.rotate_left(k) >> 8) & 0xffff) as f32 / 65535.0;
        Ai { offset: (r(3) - 0.5) * 10.0, start_err: (r(11) - 0.5) * 0.7 }
    }
}

fn wrap(mut a: f32) -> f32 {
    while a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    }
    while a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

/// Steer at a point on the line `offset` right of the centreline,
/// sliding the long bends.
fn follow(r: &Race, x: f32, y: f32, heading: f32, speed: f32, drift: i8, idx: usize, offset: f32) -> Controls {
    let look = 10 + (speed.max(0.0) * 0.35) as isize;
    let ti = r.track.ahead(idx, look);
    let (tx, ty) = r.track.pts[ti];
    let (dx, dy) = r.track.tangent(ti);
    let (px, py) = (tx - dy * offset, ty + dx * offset);
    let err = wrap((py - y).atan2(px - x) - heading);
    // how hard the road bends further on decides whether to slide
    let far = r.track.ahead(idx, look + 40);
    let bend = wrap(r.track.heading_at(far) - r.track.heading_at(idx));
    let slide = if drift != 0 {
        bend * drift as f32 > 0.12 && err * (drift as f32) > -0.3
    } else {
        bend.abs() > 0.55 && speed > 35.0
    };
    let steer = if drift != 0 && slide {
        (err * 3.0 * drift as f32 - 0.2).clamp(-1.0, 1.0) * drift as f32
    } else {
        (err * 3.0).clamp(-1.0, 1.0)
    };
    Controls { steer, gas: true, drift: slide, ..Default::default() }
}

pub fn drive(r: &Race, i: usize) -> Controls {
    let me = &r.racers[i];
    let ai = me.ai.unwrap_or(Ai { offset: 0.0, start_err: 0.0 });
    let k = &me.kart;
    if r.counting_down() {
        // start holding the gas as the 1 appears, give or take nerves
        return Controls { gas: r.clock >= COUNTDOWN - 1.0 + ai.start_err, ..Default::default() };
    }
    let mut offset = ai.offset;
    for t in &r.traps {
        // steer round dropped presents in the next stretch
        let ti = r.track.idx(t.x, t.y);
        if let Some(ti) = ti {
            let ahead = (ti as isize - me.idx as isize).rem_euclid(r.track.len() as isize);
            if ahead < 40 {
                let (cx, cy) = r.track.pts[ti];
                let (dx, dy) = r.track.tangent(ti);
                let side = (t.x - cx) * -dy + (t.y - cy) * dx;
                if (side - offset).abs() < 4.0 {
                    offset = if side > 0.0 { side - 6.0 } else { side + 6.0 };
                }
            }
        }
    }
    // pull out to pass anyone just ahead on this line
    for (j, o) in r.racers.iter().enumerate() {
        if j == i {
            continue;
        }
        let (dx, dy) = (o.kart.x - k.x, o.kart.y - k.y);
        let along = dx * k.heading.cos() + dy * k.heading.sin();
        let side = -dx * k.heading.sin() + dy * k.heading.cos();
        if along > 0.0 && along < 14.0 && side.abs() < 3.5 && o.kart.speed < k.speed + 3.0 {
            // where they are across the road, roughly; go round on the side
            // with more road
            let theirs = offset + side;
            offset = if theirs < 0.0 { theirs + 5.5 } else { theirs - 5.5 };
            break;
        }
    }
    let mut c = follow(r, k.x, k.y, k.heading, k.speed, k.drift, me.idx, offset.clamp(-10.0, 10.0));
    if me.roulette > 0.0 {
        return c;
    }
    let Some(item) = me.item else { return c };
    let near = |dir: f32, lo: f32, hi: f32| -> bool {
        r.racers.iter().enumerate().any(|(j, o)| {
            if j == i {
                return false;
            }
            let (dx, dy) = (o.kart.x - k.x, o.kart.y - k.y);
            let along = dx * k.heading.cos() + dy * k.heading.sin();
            let side = -dx * k.heading.sin() + dy * k.heading.cos();
            along * dir > lo && along * dir < hi && side.abs() < 5.0
        })
    };
    // a little hash of the clock so rivals don't all fire on the same frame
    let jitter = ((r.clock * 7.0 + i as f32 * 1.3).sin() * 0.5 + 0.5) < 0.08;
    c.item = match item {
        Item::Snowball => near(1.0, 5.0, 45.0) || jitter && me.place > 3,
        Item::Present if me.place == 1 => {
            if near(-1.0, 2.0, 30.0) {
                c.brake = true;
                true
            } else {
                false
            }
        }
        Item::Present => near(1.0, 25.0, 70.0),
        Item::Tinsel => me.place > 1,
        Item::Peppermint => c.steer.abs() < 0.3 && !c.drift,
        Item::Mistletoe => near(1.0, 0.0, 30.0) || near(-1.0, 0.0, 15.0),
    };
    if c.item && c.brake {
        // a tap of the brake to drop it, not a stop
        c.gas = true;
    }
    c
}

/// Santa: follow the road at full tilt, then go straight for the target.
pub fn drive_santa(r: &Race, target: usize) -> Controls {
    let s = &r.santa.kart;
    let t = &r.racers[target].kart;
    let d = (t.x - s.x).hypot(t.y - s.y);
    if d < 45.0 {
        // aim a little ahead of where the target is going
        let lead = d / 80.0;
        let (ax, ay) = (t.x + t.travel.cos() * t.speed * lead, t.y + t.travel.sin() * t.speed * lead);
        let err = wrap((ay - s.y).atan2(ax - s.x) - s.heading);
        return Controls { steer: (err * 3.5).clamp(-1.0, 1.0), gas: true, ..Default::default() };
    }
    let mut c = follow(r, s.x, s.y, s.heading, s.speed, 0, r.santa.idx, 0.0);
    c.drift = false;
    c
}

/// Santa after a catch: on along the road, easing off.
pub fn drive_santa_on(r: &Race) -> Controls {
    let s = &r.santa.kart;
    let mut c = follow(r, s.x, s.y, s.heading, s.speed, 0, r.santa.idx, 6.0);
    c.drift = false;
    c.gas = s.speed < 45.0;
    c
}
