//! Christmas folklore that gets in the way.
//!
//! - The Yule Cat (Jolakotturinn, Iceland) sits by the road, huge and
//!   black with glowing eyes. Now and then it crouches, wiggles, and
//!   pounces low across the road: anyone under it spins out.
//! - The Yule Lads (Iceland's thirteen pranksters) lurk in the snow by the
//!   present boxes. Pass close holding an item and one pops out, snatches
//!   it and runs off giggling; then he hides again for a while.
//! - Ded Moroz (Grandfather Frost, Russia) stands by the road with his
//!   staff. Every so often he raises it (the warning), then strikes the
//!   ground: frost spreads across the road ahead of him into a patch of ice
//!   that melts away again after a few seconds.
//! - Snegurochka (the Snow Maiden, his granddaughter) stands by the road
//!   further on and gives a present to anyone in the back half of the
//!   field who comes past empty-handed: the Yule Lads' opposite.

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
/// Ded Moroz: the wait between strikes, the raised staff, the frost
/// spreading, holding, and melting.
const FROST_WAIT: f32 = 11.0;
const FROST_RAISE: f32 = 1.2;
const FROST_SPREAD: f32 = 0.8;
const FROST_HOLD: f32 = 7.0;
const FROST_MELT: f32 = 2.0;
pub const FROST_R: f32 = 10.0;
/// Snegurochka: how close you pass her, and how long before she gives the
/// same racer another.
const GIFT_REACH: f32 = 9.0;
const GIFT_AGAIN: f32 = 25.0;

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

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Frost {
    Waiting(f32),
    /// Staff raised: the warning.
    Raising(f32),
    Spreading(f32),
    Holding(f32),
    Melting(f32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Moroz {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    pub state: Frost,
    /// Where the frost spreads from, on the road ahead of him.
    pub centre: (f32, f32),
}

impl Moroz {
    /// The ice he has laid: centre and radius, if any.
    pub fn frost(&self) -> Option<(f32, f32, f32)> {
        let r = match self.state {
            Frost::Spreading(t) => FROST_R * t / FROST_SPREAD,
            Frost::Holding(_) => FROST_R,
            Frost::Melting(t) => FROST_R * (1.0 - t / FROST_MELT),
            _ => return None,
        };
        (r > 0.5).then_some((self.centre.0, self.centre.1, r))
    }

    pub fn raised(&self) -> bool {
        matches!(self.state, Frost::Raising(_) | Frost::Spreading(_))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Maiden {
    pub x: f32,
    pub y: f32,
    pub heading: f32,
    /// Seconds since she last gave a present (for drawing her arm out).
    pub gave: f32,
    /// Per racer: seconds before she'll give them another.
    wait: Vec<f32>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Folk {
    /// Racer caught under the cat.
    Pounced(usize),
    /// The cat sprang.
    Yowl,
    /// Racer's item snatched.
    Stolen(usize),
    /// Ded Moroz struck his staff: frost is spreading.
    Frost,
    /// Snegurochka gave racer a present.
    Gift(usize),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Folklore {
    pub cat: Option<YuleCat>,
    pub lads: Vec<YuleLad>,
    pub moroz: Option<Moroz>,
    pub maiden: Option<Maiden>,
}

/// A spot `off` units to the side of the road at fraction `f` (positive is
/// left of the direction of travel), facing the road; and the sample.
fn roadside(track: &Track, f: f32, off: f32) -> (f32, f32, f32, usize) {
    let i = (f * track.len() as f32) as usize % track.len();
    let (px, py) = track.pts[i];
    let (tx, ty) = track.tangent(i);
    let (x, y) = (px - ty * off, py + tx * off);
    (x, y, (py - y).atan2(px - x), i)
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
        let moroz = track.ded_moroz.map(|f| {
            let (x, y, heading, i) = roadside(track, f, track.half + 6.0);
            // the frost spreads over the road a little way past him
            let centre = track.pts[(i + 18) % track.len()];
            Moroz { x, y, heading, state: Frost::Waiting(FROST_WAIT * 0.4), centre }
        });
        let maiden = track.snegurochka.map(|f| {
            let (x, y, heading, _) = roadside(track, f, -(track.half + 4.0));
            Maiden { x, y, heading, gave: 99.0, wait: Vec::new() }
        });
        Folklore { cat, lads, moroz, maiden }
    }

    /// The ice Ded Moroz has laid, if any: centre and radius.
    pub fn frost(&self) -> Option<(f32, f32, f32)> {
        self.moroz.and_then(|m| m.frost())
    }

    /// Ded Moroz and Snegurochka. `wanting[i]`: racer i is in the back half
    /// of the field with nothing in hand.
    pub fn step_frost_and_gifts(&mut self, dt: f32, karts: &[&Kart], wanting: &[bool]) -> Vec<Folk> {
        let mut out = Vec::new();
        if let Some(m) = &mut self.moroz {
            m.state = match m.state {
                Frost::Waiting(t) if t + dt >= FROST_WAIT => Frost::Raising(0.0),
                Frost::Waiting(t) => Frost::Waiting(t + dt),
                Frost::Raising(t) if t + dt >= FROST_RAISE => {
                    out.push(Folk::Frost);
                    Frost::Spreading(0.0)
                }
                Frost::Raising(t) => Frost::Raising(t + dt),
                Frost::Spreading(t) if t + dt >= FROST_SPREAD => Frost::Holding(0.0),
                Frost::Spreading(t) => Frost::Spreading(t + dt),
                Frost::Holding(t) if t + dt >= FROST_HOLD => Frost::Melting(0.0),
                Frost::Holding(t) => Frost::Holding(t + dt),
                Frost::Melting(t) if t + dt >= FROST_MELT => Frost::Waiting(0.0),
                Frost::Melting(t) => Frost::Melting(t + dt),
            };
        }
        if let Some(g) = &mut self.maiden {
            g.gave += dt;
            g.wait.resize(karts.len(), 0.0);
            for (i, k) in karts.iter().enumerate() {
                g.wait[i] = (g.wait[i] - dt).max(0.0);
                if g.wait[i] == 0.0 && wanting.get(i).copied().unwrap_or(false) && (k.x - g.x).hypot(k.y - g.y) < GIFT_REACH {
                    g.wait[i] = GIFT_AGAIN;
                    g.gave = 0.0;
                    out.push(Folk::Gift(i));
                }
            }
        }
        out
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
    fn ded_moroz_warns_then_lays_frost_that_melts() {
        let t = Track::birch_wood();
        let mut f = Folklore::new(&t, &[]);
        let m = f.moroz.expect("he's on the Birch Wood");
        assert!(t.dist(m.x, m.y) > t.half, "he stands off the road");
        assert!(t.dist(m.centre.0, m.centre.1) < 1.0, "the frost is on the road");
        let (mut raised_first, mut frost, mut most, mut melted) = (false, false, 0.0f32, false);
        for _ in 0..(30.0 / DT) as usize {
            let ev = f.step_frost_and_gifts(DT, &[], &[]);
            if ev.contains(&Folk::Frost) {
                raised_first = true;
            }
            if let Some((_, _, r)) = f.frost() {
                assert!(raised_first, "a warning first");
                frost = true;
                most = most.max(r);
            } else if frost {
                melted = true;
            }
        }
        assert!(frost && melted, "frost came and went");
        assert_eq!(most, FROST_R);
    }

    #[test]
    fn snegurochka_gives_to_the_back_of_the_field() {
        let t = Track::birch_wood();
        let mut f = Folklore::new(&t, &[]);
        let g = f.maiden.clone().expect("she's on the Birch Wood");
        let k = Kart::new(g.x + 3.0, g.y, 0.0, crate::roster::ROSTER[0].stats);
        // a leader, or anyone holding an item, gets nothing
        assert!(f.step_frost_and_gifts(DT, &[&k], &[false]).is_empty());
        assert_eq!(f.step_frost_and_gifts(DT, &[&k], &[true]), vec![Folk::Gift(0)]);
        // and not again straight away
        assert!(f.step_frost_and_gifts(DT, &[&k], &[true]).is_empty());
        assert!(t.dist(g.x, g.y) > t.half, "she stands off the road");
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
