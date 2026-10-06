//! Christmas folklore that gets in the way.
//!
//! - The Yule Cat (Jolakotturinn, Iceland) sits by the road, huge and
//!   black with glowing eyes. Now and then it crouches, wiggles, and
//!   pounces low across the road: anyone under it spins out.
//! - The Yule Lads (Iceland's thirteen pranksters) lurk in the snow by the
//!   present boxes. Pass close holding an item and one pops out, snatches
//!   it and runs off giggling; then he hides again for a while.

use crate::kart::Kart;
use crate::track::Track;
use serde::{Deserialize, Serialize};

/// The cat: sitting, crouching (the warning), the pounce.
const SIT: f32 = 9.0;
const CROUCH: f32 = 1.6;
const POUNCE: f32 = 1.1;
const PAW_R: f32 = 4.0;
/// The lads: how close an item tempts one, and how long until he's back.
const TEMPT: f32 = 6.5;
const HIDE: f32 = 14.0;
const RUN: f32 = 2.2;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Cat {
    Sitting(f32),
    Crouching(f32),
    /// In the air, seconds since it sprang.
    Pouncing(f32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct YuleCat {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    pub state: Cat,
    /// Where it sits on either side of the road; which side it's on now.
    sides: [(f32, f32); 2],
    on: usize,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Lad {
    Lurking,
    /// Running off with the loot, seconds since.
    Running(f32),
    /// Gone to ground, seconds left before he lurks again.
    Hiding(f32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct YuleLad {
    pub x: f32,
    pub y: f32,
    home: (f32, f32),
    away: (f32, f32),
    pub heading: f32,
    pub state: Lad,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Folk {
    /// Racer caught under the cat.
    Pounced(usize),
    /// The cat sprang.
    Yowl,
    /// Racer's item snatched.
    Stolen(usize),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Folklore {
    pub cat: Option<YuleCat>,
    pub lads: Vec<YuleLad>,
}

impl Folklore {
    pub fn new(track: &Track, boxes: &[(f32, f32)]) -> Folklore {
        let cat = track.yule_cat.map(|f| {
            let i = (f * track.len() as f32) as usize;
            let (px, py) = track.pts[i];
            let (tx, ty) = track.tangent(i);
            let off = track.half + 8.0;
            let sides = [(px + ty * off, py - tx * off), (px - ty * off, py + tx * off)];
            YuleCat { x: sides[0].0, y: sides[0].1, z: 0.0, heading: 0.0, state: Cat::Sitting(SIT * 0.6), sides, on: 0 }
        });
        let mut lads = Vec::new();
        if track.yule_lads {
            // one beside the outermost box of each row
            for row in boxes.chunks(4) {
                let Some(&(bx, by)) = row.last() else { continue };
                let Some(i) = track.idx(bx, by) else { continue };
                let (px, py) = track.pts[i];
                let (tx, ty) = track.tangent(i);
                let (sx, sy) = (bx - px, by - py);
                let l = sx.hypot(sy).max(0.1);
                let (ox, oy) = (sx / l, sy / l);
                let home = (px + ox * (track.half + 5.0) + tx * 6.0, py + oy * (track.half + 5.0) + ty * 6.0);
                let away = (home.0 + ox * 22.0, home.1 + oy * 22.0);
                lads.push(YuleLad { x: home.0, y: home.1, home, away, heading: oy.atan2(ox), state: Lad::Lurking });
            }
        }
        Folklore { cat, lads }
    }

    /// One step. `holding[i]`: whether racer i has an item to lose.
    pub fn step(&mut self, dt: f32, karts: &mut [&mut Kart], holding: &[bool], track: &Track) -> Vec<Folk> {
        let mut out = Vec::new();
        if let Some(c) = &mut self.cat {
            c.state = match c.state {
                Cat::Sitting(t) if t + dt >= SIT => Cat::Crouching(0.0),
                Cat::Sitting(t) => Cat::Sitting(t + dt),
                Cat::Crouching(t) if t + dt >= CROUCH => {
                    out.push(Folk::Yowl);
                    Cat::Pouncing(0.0)
                }
                Cat::Crouching(t) => Cat::Crouching(t + dt),
                Cat::Pouncing(t) if t + dt >= POUNCE => {
                    c.on = 1 - c.on;
                    Cat::Sitting(0.0)
                }
                Cat::Pouncing(t) => Cat::Pouncing(t + dt),
            };
            let (from, to) = (c.sides[c.on], c.sides[1 - c.on]);
            c.heading = (to.1 - from.1).atan2(to.0 - from.0);
            match c.state {
                Cat::Pouncing(t) => {
                    let k = t / POUNCE;
                    c.x = from.0 + (to.0 - from.0) * k;
                    c.y = from.1 + (to.1 - from.1) * k;
                    // a low, flat pounce: dangerous right across the road
                    c.z = track.height(c.x, c.y) + 3.0 * (k * std::f32::consts::PI).sin();
                    for (i, kart) in karts.iter_mut().enumerate() {
                        if !kart.stunned() && (kart.x - c.x).hypot(kart.y - c.y) < PAW_R {
                            kart.hit(1.0, 1.0);
                            out.push(Folk::Pounced(i));
                        }
                    }
                }
                _ => {
                    c.x = from.0;
                    c.y = from.1;
                    c.z = track.height(c.x, c.y);
                }
            }
        }
        for lad in &mut self.lads {
            lad.state = match lad.state {
                Lad::Lurking => {
                    let thief = karts.iter().enumerate().find(|(i, k)| holding.get(*i).copied().unwrap_or(false) && (k.x - lad.x).hypot(k.y - lad.y) < TEMPT);
                    if let Some((i, _)) = thief {
                        out.push(Folk::Stolen(i));
                        lad.heading = (lad.away.1 - lad.home.1).atan2(lad.away.0 - lad.home.0);
                        Lad::Running(0.0)
                    } else {
                        Lad::Lurking
                    }
                }
                Lad::Running(t) => {
                    let k = (t / RUN).min(1.0);
                    lad.x = lad.home.0 + (lad.away.0 - lad.home.0) * k;
                    lad.y = lad.home.1 + (lad.away.1 - lad.home.1) * k;
                    if t + dt >= RUN {
                        Lad::Hiding(HIDE)
                    } else {
                        Lad::Running(t + dt)
                    }
                }
                Lad::Hiding(left) if left - dt <= 0.0 => {
                    lad.x = lad.home.0;
                    lad.y = lad.home.1;
                    Lad::Lurking
                }
                Lad::Hiding(left) => Lad::Hiding(left - dt),
            };
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kart::DT;
    use crate::race::Race;

    fn boxes(t: Track) -> (Track, Vec<(f32, f32)>) {
        let r = Race::with_humans(t, &[], 1);
        let b = r.boxes.iter().map(|b| (b.x, b.y)).collect();
        (r.track, b)
    }

    #[test]
    fn the_yule_cat_warns_then_pounces_across_the_road() {
        let (t, b) = boxes(Track::eggnogg());
        let mut f = Folklore::new(&t, &b);
        let start = f.cat.unwrap();
        let mid = ((start.sides[0].0 + start.sides[1].0) / 2.0, (start.sides[0].1 + start.sides[1].1) / 2.0);
        let mut k = Kart::new(mid.0, mid.1, 0.0, crate::roster::ROSTER[0].stats);
        k.settle(&t);
        let (mut crouched, mut yowl, mut caught) = (false, false, false);
        for _ in 0..(20.0 / DT) as usize {
            // hold the kart in the middle of the road
            k.x = mid.0;
            k.y = mid.1;
            let ev = f.step(DT, &mut [&mut k], &[false], &t);
            crouched |= matches!(f.cat.unwrap().state, Cat::Crouching(_)) && !yowl;
            yowl |= ev.contains(&Folk::Yowl);
            caught |= ev.contains(&Folk::Pounced(0));
        }
        assert!(crouched, "a crouch first, as a warning");
        assert!(yowl && caught, "then the pounce, across the road");
        let c = f.cat.unwrap();
        assert!(t.dist(c.x, c.y) > t.half, "it sits off the road");
    }

    #[test]
    fn a_yule_lad_snatches_an_item_and_hides() {
        let (t, b) = boxes(Track::eggnogg());
        let mut f = Folklore::new(&t, &b);
        assert_eq!(f.lads.len(), 3, "one by each row of boxes");
        let lad = f.lads[0];
        let mut k = Kart::new(lad.x + 2.0, lad.y, 0.0, crate::roster::ROSTER[0].stats);
        // empty-handed: he stays put
        assert!(f.step(DT, &mut [&mut k], &[false], &t).is_empty());
        let ev = f.step(DT, &mut [&mut k], &[true], &t);
        assert_eq!(ev, vec![Folk::Stolen(0)]);
        for _ in 0..(3.0 / DT) as usize {
            f.step(DT, &mut [&mut k], &[true], &t);
        }
        assert!(matches!(f.lads[0].state, Lad::Hiding(_)), "gone to ground");
    }
}
