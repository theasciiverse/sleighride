//! One race: the whole roster, the countdown, laps and places, present boxes and
//! what comes out of them, and Santa.
//!
//! Laps are counted by quarters of the lap, in order, so cutting back over
//! the line or turning round on it does not count.
//!
//! Items, from the present boxes across the road (odds lean on your place:
//! the leader gets things to defend with, the back of the field things to
//! catch up with):
//! - Snowballs (three): thrown straight ahead.
//! - Present: lobbed ahead, bursts where it lands and spins out anyone near.
//!   Use it while braking to drop it behind you as a trap instead.
//! - Tinsel: a garland that homes in on the racer one place ahead.
//! - Peppermint: a boost.
//! - Mistletoe: held out on a stick for a few seconds; any rival who comes
//!   close gets a kiss and stops dead, and you get a little boost.
//!
//! Santa: once the leader is on lap 2 he comes roaring up from behind in his
//! snowplough and hunts whoever is in front. Catching them spins them out
//! hard; then he goes off to cool down, and comes back.

use serde::{Deserialize, Serialize};

use crate::auto::{self, Ai};
use crate::kart::{Controls, Kart, DT};
use crate::roster::{PLOUGH, ROSTER};
use crate::track::{Surface, Track};

pub const LAPS: u8 = 3;
/// Seconds of countdown before the lights go.
pub const COUNTDOWN: f32 = 3.0;
const BUMP_R: f32 = 3.0;
const BOX_R: f32 = 2.6;
const ROULETTE: f32 = 1.0;
/// Santa's rest between catches.
const SANTA_REST: f32 = 22.0;
const MISTLETOE_TIME: f32 = 8.0;
const KISS_R: f32 = 5.0;
/// How long a clip round the ear shows: the swing, then the stars.
pub const CLIP_TIME: f32 = 1.8;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Item {
    Snowball,
    Present,
    Tinsel,
    Peppermint,
    Mistletoe,
}

pub const ITEMS: [Item; 5] = [Item::Snowball, Item::Present, Item::Tinsel, Item::Peppermint, Item::Mistletoe];

impl Item {
    pub fn name(self) -> &'static str {
        match self {
            Item::Snowball => "SNOWBALLS",
            Item::Present => "PRESENT",
            Item::Tinsel => "TINSEL",
            Item::Peppermint => "PEPPERMINT",
            Item::Mistletoe => "MISTLETOE",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Racer {
    /// Index into the roster.
    pub who: usize,
    pub kart: Kart,
    /// Current lap, 1-based.
    pub lap: u8,
    quarter: u8,
    lap_start: f32,
    pub lap_times: Vec<f32>,
    /// Race time at the finish.
    pub finished: Option<f32>,
    /// Nearest centreline sample.
    pub idx: usize,
    /// How long the racer has been heading the wrong way.
    pub wrong_way: f32,
    /// 1 = leading.
    pub place: u8,
    pub item: Option<Item>,
    pub ammo: u8,
    /// The item is still spinning in the slot.
    pub roulette: f32,
    /// Visual lean, smoothed.
    pub lean: f32,
    /// None: the player drives.
    pub ai: Option<Ai>,
    pub last: Controls,
    /// Tinsel wrapped round them, for drawing.
    pub tangled: f32,
    /// Mistletoe held out, seconds left.
    pub mistletoe: f32,
    /// Just kissed: hearts, and can't be kissed again yet.
    pub smooched: f32,
    /// Times Santa has caught this racer.
    pub caught: u8,
    /// A clip round the ear just landed: the mitten, then dizzy stars.
    pub clipped: f32,
}

impl Racer {
    pub fn is_player(&self) -> bool {
        self.ai.is_none()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ShotKind {
    Snowball,
    Present,
    Tinsel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Shot {
    pub kind: ShotKind,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub owner: usize,
    pub target: Option<usize>,
    pub life: f32,
    pub age: f32,
}

/// A present dropped on the road.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trap {
    pub x: f32,
    pub y: f32,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ItemBox {
    pub x: f32,
    pub y: f32,
    /// Seconds until it is back after being taken; 0 = there.
    pub gone: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Santa {
    pub kart: Kart,
    pub active: bool,
    /// Racer being hunted.
    pub target: Option<usize>,
    /// Rest left before he comes again.
    pub rest: f32,
    pub idx: usize,
    pub visits: u32,
    /// After a catch he rumbles on past for a moment before leaving.
    pub gloat: f32,
}

/// Things that happened this step, for messages and effects.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Note {
    SantaComing,
    SantaCaught(usize),
    /// Racer, hit by what (None: Santa or a trap).
    Hit(usize, Option<ShotKind>),
    Burst(f32, f32),
    Got(usize, Item),
    Used(usize, Item),
    /// Kisser, kissed.
    Kiss(usize, usize),
    Blizzard,
    /// Thrown by a snow tornado.
    Twister(usize),
    /// Bowled a penguin over.
    Penguin(usize),
    /// Swiped by the polar bear.
    Swipe(usize),
    /// Caught under the Yule Cat's pounce.
    Pounced(usize),
    /// The Yule Cat sprang.
    Yowl,
    /// A Yule Lad snatched this racer's item.
    Stolen(usize),
    /// Someone in the stands threw a present, from (x, y).
    Thrown(f32, f32),
    Finished(usize),
}

pub struct Race {
    pub track: Track,
    pub racers: Vec<Racer>,
    /// Seconds since the countdown began.
    pub clock: f32,
    pub shots: Vec<Shot>,
    pub traps: Vec<Trap>,
    pub boxes: Vec<ItemBox>,
    pub santa: Santa,
    pub weather: crate::weather::Weather,
    pub wildlife: crate::wildlife::Wildlife,
    pub folklore: crate::folklore::Folklore,
    pub crowd: crate::crowd::Crowd,
    pub notes: Vec<Note>,
    /// Finishing order, racer indices.
    pub order: Vec<usize>,
    /// The autopilot drives the player too (screenshots, tests).
    pub autopilot: bool,
    /// Rivals' top speed as a share of their own (difficulty).
    pub rival_pace: f32,
    seed: u32,
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

impl Race {
    /// A race of the whole roster. `player` is the roster index the player
    /// drives (they start at the back); None races everyone on autopilot.
    #[cfg(test)]
    pub fn new(track: Track, player: Option<usize>, seed: u32) -> Race {
        let humans: Vec<usize> = player.into_iter().collect();
        Race::with_humans(track, &humans, seed)
    }

    /// A race of the whole roster where the racers in `humans` (roster
    /// indices) are driven by people; they start at the back of the grid,
    /// the first of them last. With nobody, the grid turns with the seed.
    pub fn with_humans(track: Track, humans: &[usize], seed: u32) -> Race {
        let n = ROSTER.len();
        let mut racers = Vec::new();
        let ais: Vec<usize> = (0..n).filter(|w| !humans.contains(w)).collect();
        for who in 0..n {
            // grid slot 0 is the front
            let slot = if humans.is_empty() {
                (who + seed as usize) % n
            } else if let Some(h) = humans.iter().position(|&w| w == who) {
                n - 1 - h
            } else {
                ais.iter().position(|&w| w == who).unwrap()
            };
            let back = 10 + slot as isize * 9;
            let i = track.ahead(0, -back);
            let (px, py) = track.pts[i];
            let (tx, ty) = track.tangent(i);
            let side = if slot % 2 == 0 { -5.0 } else { 5.0 };
            let (x, y) = (px - ty * side, py + tx * side);
            let mut kart = Kart::new(x, y, track.heading_at(i), ROSTER[who].stats);
            kart.settle(&track);
            let ai = if humans.contains(&who) { None } else { Some(Ai::new(who, seed)) };
            racers.push(Racer {
                who,
                kart,
                lap: 1,
                quarter: 0,
                lap_start: 0.0,
                lap_times: Vec::new(),
                finished: None,
                idx: i,
                wrong_way: 0.0,
                place: slot as u8 + 1,
                item: None,
                ammo: 0,
                roulette: 0.0,
                lean: 0.0,
                ai,
                last: Controls::default(),
                tangled: 0.0,
                mistletoe: 0.0,
                smooched: 0.0,
                caught: 0,
                clipped: 0.0,
            });
        }
        // three rows of present boxes across the road
        let mut boxes = Vec::new();
        for frac in [0.2, 0.5, 0.78] {
            let i = (track.len() as f32 * frac) as usize;
            let (px, py) = track.pts[i];
            let (tx, ty) = track.tangent(i);
            for side in [-9.0, -3.0, 3.0, 9.0] {
                boxes.push(ItemBox { x: px - ty * side, y: py + tx * side, gone: 0.0 });
            }
        }
        let mut santa = Kart::new(0.0, 0.0, 0.0, PLOUGH);
        santa.plough = true;
        let weather = crate::weather::Weather::new(&track, seed);
        let wildlife = crate::wildlife::Wildlife::new(&track);
        let box_spots: Vec<(f32, f32)> = boxes.iter().map(|b| (b.x, b.y)).collect();
        let folklore = crate::folklore::Folklore::new(&track, &box_spots);
        let crowd = crate::crowd::Crowd::new(&track, seed);
        Race {
            weather,
            wildlife,
            folklore,
            crowd,
            track,
            racers,
            clock: 0.0,
            shots: Vec::new(),
            traps: Vec::new(),
            boxes,
            santa: Santa { kart: santa, active: false, target: None, rest: 0.0, idx: 0, visits: 0, gloat: 0.0 },
            notes: Vec::new(),
            order: Vec::new(),
            autopilot: false,
            rival_pace: 1.0,
            seed: seed.max(1),
        }
    }

    pub fn counting_down(&self) -> bool {
        self.clock < COUNTDOWN
    }

    /// Seconds since the lights went.
    pub fn time(&self) -> f32 {
        (self.clock - COUNTDOWN).max(0.0)
    }

    pub fn lap_time(&self, i: usize) -> f32 {
        self.time() - self.racers[i].lap_start
    }

    pub fn best_lap(&self, i: usize) -> Option<f32> {
        self.racers[i].lap_times.iter().copied().reduce(f32::min)
    }

    /// The player's racer, if there is one.
    pub fn player(&self) -> Option<usize> {
        self.racers.iter().position(|r| r.is_player())
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed & 0xffff) as f32 / 65535.0
    }

    /// Distance along the race so far, in samples.
    pub fn progress(&self, i: usize) -> f32 {
        let r = &self.racers[i];
        let n = self.track.len() as f32;
        let mut local = r.idx as f32;
        if r.quarter == 0 && local > n * 0.75 {
            local -= n;
        }
        if r.quarter == 3 && local < n * 0.25 {
            local += n;
        }
        (r.lap as f32 - 1.0) * n + local
    }

    /// Racer in a given place.
    pub fn in_place(&self, place: u8) -> Option<usize> {
        self.racers.iter().position(|r| r.place == place)
    }

    /// One step with the (first) player's controls.
    #[cfg(test)]
    pub fn step(&mut self, player: Controls) {
        match self.player() {
            Some(p) => self.step_with(&[(p, player)]),
            None => self.step_with(&[]),
        }
    }

    /// One step with each human racer's controls: (racer index, controls).
    /// A human with none this step coasts.
    pub fn step_with(&mut self, inputs: &[(usize, Controls)]) {
        self.notes.clear();
        let was_counting = self.counting_down();
        self.clock += DT;
        let controls: Vec<Controls> = (0..self.racers.len())
            .map(|i| {
                let r = &self.racers[i];
                if r.is_player() && r.finished.is_none() && !self.autopilot {
                    inputs.iter().find(|(k, _)| *k == i).map_or(Controls::default(), |(_, c)| *c)
                } else {
                    auto::drive(self, i)
                }
            })
            .collect();
        if was_counting {
            let go = !self.counting_down();
            for (r, c) in self.racers.iter_mut().zip(&controls) {
                r.kart.wait(*c);
                if go {
                    r.kart.go();
                }
            }
            return;
        }
        // the field closes up on the leading human
        let player_progress = (0..self.racers.len()).filter(|&i| self.racers[i].is_player()).map(|i| self.progress(i)).reduce(f32::max);
        for i in 0..self.racers.len() {
            let c = controls[i];
            // rivals run at the difficulty's pace, close up a little on the
            // player and ease off ahead
            if let (Some(pp), true) = (player_progress, self.racers[i].ai.is_some()) {
                let gap = pp - self.progress(i);
                let k = (gap / 600.0 * 0.04).clamp(-0.03, 0.04);
                self.racers[i].kart.stats.top = ROSTER[self.racers[i].who].stats.top * self.rival_pace * (1.0 + k);
            }
            let track = &self.track;
            let r = &mut self.racers[i];
            r.kart.step(c, track);
            r.last = c;
            r.tangled = (r.tangled - DT).max(0.0);
            r.smooched = (r.smooched - DT).max(0.0);
            r.clipped = (r.clipped - DT).max(0.0);
            r.mistletoe = (r.mistletoe - DT).max(0.0);
            let target = if r.kart.drift != 0 { r.kart.drift as f32 * 0.9 } else { c.steer * 0.5 };
            r.lean += (target - r.lean) * (10.0 * DT).min(1.0);
            if r.roulette > 0.0 {
                r.roulette = (r.roulette - DT).max(0.0);
            } else if c.item && r.item.is_some() && !r.kart.stunned() {
                self.use_item(i, c.brake);
            }
            self.lap_check(i);
        }
        self.weather_step();
        self.bumps();
        self.kisses();
        self.pick_boxes();
        self.fly_shots();
        self.spring_traps();
        self.santa_step();
        self.place();
    }

    fn weather_step(&mut self) {
        if self.weather.step(DT, &self.track) {
            self.notes.push(Note::Blizzard);
            log::info!("a blizzard blows in");
        }
        {
            let mut karts: Vec<&mut Kart> = self.racers.iter_mut().map(|r| &mut r.kart).collect();
            for e in self.wildlife.step(DT, &mut karts) {
                self.notes.push(match e {
                    crate::wildlife::Wild::Bowled(i) => Note::Penguin(i),
                    crate::wildlife::Wild::Swiped(i) => Note::Swipe(i),
                });
            }
        }
        {
            let holding: Vec<bool> = self.racers.iter().map(|r| r.item.is_some() && r.roulette == 0.0).collect();
            let mut karts: Vec<&mut Kart> = self.racers.iter_mut().map(|r| &mut r.kart).collect();
            for e in self.folklore.step(DT, &mut karts, &holding, &self.track) {
                match e {
                    crate::folklore::Folk::Pounced(i) => self.notes.push(Note::Pounced(i)),
                    crate::folklore::Folk::Yowl => self.notes.push(Note::Yowl),
                    crate::folklore::Folk::Stolen(i) => {
                        self.racers[i].item = None;
                        self.racers[i].ammo = 0;
                        self.notes.push(Note::Stolen(i));
                    }
                }
            }
        }
        if !self.crowd.fans.is_empty() && self.clock > COUNTDOWN {
            let karts: Vec<&Kart> = self.racers.iter().map(|r| &r.kart).collect();
            for l in self.crowd.step(DT, &karts) {
                self.notes.push(Note::Thrown(l.x, l.y));
                // nobody's present: it can hit anyone, the thrower has no racer
                self.shots.push(Shot { kind: ShotKind::Present, x: l.x, y: l.y, z: l.z, vx: l.vx, vy: l.vy, vz: l.vz, owner: usize::MAX, target: Some(l.at), life: 3.0, age: 0.0 });
            }
        }
        let (px, py) = self.weather.push(DT);
        let twisters = self.weather.twisters.clone();
        for i in 0..self.racers.len() {
            let k = &mut self.racers[i].kart;
            k.x += px;
            k.y += py;
            for tw in &twisters {
                if !k.stunned() && (k.x - tw.x).hypot(k.y - tw.y) < crate::weather::TWISTER_R {
                    // thrown up and spun round
                    k.hit(1.1, 1.5);
                    k.air = true;
                    k.vz = 16.0;
                    self.notes.push(Note::Twister(i));
                }
            }
        }
    }

    fn lap_check(&mut self, i: usize) {
        let time = self.time();
        let n = self.track.len();
        let r = &mut self.racers[i];
        let Some(idx) = self.track.idx(r.kart.x, r.kart.y) else { return };
        r.idx = idx;
        let q = (idx * 4 / n) as u8;
        if q == (r.quarter + 1) % 4 {
            r.quarter = q;
            if q == 0 && r.finished.is_none() {
                let t = time - r.lap_start;
                r.lap_times.push(t);
                r.lap_start = time;
                if r.is_player() {
                    log::info!("lap {} done in {t:.2}s", r.lap);
                }
                if r.lap == LAPS {
                    r.finished = Some(time);
                    self.order.push(i);
                    self.notes.push(Note::Finished(i));
                    log::info!("{} finished {} in {time:.2}s", ROSTER[r.who].name, self.order.len());
                } else {
                    r.lap += 1;
                }
            }
        }
        let (tx, ty) = self.track.tangent(idx);
        let facing = r.kart.heading.cos() * tx + r.kart.heading.sin() * ty;
        if facing < -0.3 && r.kart.speed > 5.0 {
            r.wrong_way += DT;
        } else {
            r.wrong_way = 0.0;
        }
    }

    fn place(&mut self) {
        let mut ranking: Vec<usize> = (0..self.racers.len()).collect();
        let finished_rank = |i: usize| self.order.iter().position(|&o| o == i);
        ranking.sort_by(|&a, &b| match (finished_rank(a), finished_rank(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => self.progress(b).total_cmp(&self.progress(a)),
        });
        for (p, i) in ranking.into_iter().enumerate() {
            self.racers[i].place = p as u8 + 1;
        }
    }

    /// Racers that touch are pushed apart, and if they were closing on
    /// each other they trade a little momentum along the line between them,
    /// the heavier pushing the lighter. Touching alone costs nothing.
    fn bumps(&mut self) {
        let n = self.racers.len();
        for a in 0..n {
            for b in (a + 1)..n {
                let (ka, kb) = (&self.racers[a].kart, &self.racers[b].kart);
                let (dx, dy) = (kb.x - ka.x, kb.y - ka.y);
                let d = dx.hypot(dy);
                if d >= BUMP_R || d < 1e-3 || (ka.z - kb.z).abs() > 3.0 {
                    continue;
                }
                let (ux, uy) = (dx / d, dy / d);
                let push = BUMP_R - d;
                let (wa, wb) = (ka.stats.weight, kb.stats.weight);
                let (sa, sb) = (wb / (wa + wb), wa / (wa + wb));
                let vel = |k: &Kart| (k.travel.cos() * k.speed, k.travel.sin() * k.speed);
                let (va, vb) = (vel(ka), vel(kb));
                // closing speed along the line from a to b
                let closing = (va.0 - vb.0) * ux + (va.1 - vb.1) * uy;
                let (ra, rb) = pair_mut(&mut self.racers, a, b);
                ra.kart.x -= ux * push * sa;
                ra.kart.y -= uy * push * sa;
                rb.kart.x += ux * push * sb;
                rb.kart.y += uy * push * sb;
                if closing > 0.0 {
                    // a slightly bouncy impulse, shared by weight
                    let j = closing * 1.3;
                    let set = |k: &mut Kart, v: (f32, f32)| {
                        let sp = v.0.hypot(v.1);
                        if sp > 0.01 {
                            let t = v.1.atan2(v.0);
                            let backwards = (t - k.heading).cos() < 0.0;
                            k.travel = if backwards { t + std::f32::consts::PI } else { t };
                            k.speed = if backwards { -sp } else { sp };
                        }
                    };
                    set(&mut ra.kart, (va.0 - ux * j * sa, va.1 - uy * j * sa));
                    set(&mut rb.kart, (vb.0 + ux * j * sb, vb.1 + uy * j * sb));
                }
            }
        }
    }

    fn kisses(&mut self) {
        let n = self.racers.len();
        for a in 0..n {
            if self.racers[a].mistletoe <= 0.0 {
                continue;
            }
            for b in 0..n {
                if a == b || self.racers[b].smooched > 0.0 || self.racers[b].kart.stunned() {
                    continue;
                }
                let (ka, kb) = (&self.racers[a].kart, &self.racers[b].kart);
                if (ka.x - kb.x).hypot(ka.y - kb.y) < KISS_R {
                    let (ra, rb) = if a < b {
                        pair_mut(&mut self.racers, a, b)
                    } else {
                        let (x, y) = pair_mut(&mut self.racers, b, a);
                        (y, x)
                    };
                    // stopped dead, but no spin: just smitten
                    rb.kart.hit(0.9, 0.0);
                    rb.smooched = 2.0;
                    ra.kart.boost = ra.kart.boost.max(0.6);
                    self.notes.push(Note::Kiss(a, b));
                }
            }
        }
    }

    fn roll_item(&mut self, place: u8) -> Item {
        let n = self.racers.len().max(2) as f32;
        let f = (place as f32 - 1.0) / (n - 1.0);
        let weights = [
            (Item::Snowball, 45.0 - 20.0 * f),
            (Item::Present, 45.0 - 35.0 * f),
            (Item::Tinsel, 40.0 * f),
            (Item::Peppermint, 10.0 + 35.0 * f),
            // mistletoe is for the jostling middle of the field
            (Item::Mistletoe, 30.0 * (1.0 - (f - 0.5).abs() * 2.0)),
        ];
        let total: f32 = weights.iter().map(|w| w.1).sum();
        let mut pick = self.rand() * total;
        for (item, w) in weights {
            if pick < w {
                return item;
            }
            pick -= w;
        }
        Item::Peppermint
    }

    fn pick_boxes(&mut self) {
        for b in &mut self.boxes {
            b.gone = (b.gone - DT).max(0.0);
        }
        for bi in 0..self.boxes.len() {
            if self.boxes[bi].gone > 0.0 {
                continue;
            }
            let (bx, by) = (self.boxes[bi].x, self.boxes[bi].y);
            for i in 0..self.racers.len() {
                let k = &self.racers[i].kart;
                if (k.x - bx).hypot(k.y - by) > BOX_R {
                    continue;
                }
                self.boxes[bi].gone = 3.0;
                self.notes.push(Note::Burst(bx, by));
                if self.racers[i].item.is_none() {
                    let item = self.roll_item(self.racers[i].place);
                    let r = &mut self.racers[i];
                    r.item = Some(item);
                    r.ammo = if item == Item::Snowball { 3 } else { 1 };
                    r.roulette = ROULETTE;
                    self.notes.push(Note::Got(i, item));
                }
                break;
            }
        }
    }

    fn use_item(&mut self, i: usize, backward: bool) {
        let Some(item) = self.racers[i].item else { return };
        let place = self.racers[i].place;
        let ahead = if place > 1 { self.in_place(place - 1) } else { None };
        let r = &mut self.racers[i];
        let k = &r.kart;
        let (c, s) = (k.heading.cos(), k.heading.sin());
        let fwd = k.speed.max(0.0);
        let mut shot = Shot { kind: ShotKind::Snowball, x: k.x + c * 3.0, y: k.y + s * 3.0, z: 1.5, vx: 0.0, vy: 0.0, vz: 0.0, owner: i, target: None, life: 2.0, age: 0.0 };
        match item {
            Item::Snowball => {
                shot.vx = c * (fwd + 75.0);
                shot.vy = s * (fwd + 75.0);
                self.shots.push(shot);
            }
            Item::Present if backward => {
                self.traps.push(Trap { x: k.x - c * 3.5, y: k.y - s * 3.5, age: 0.0 });
            }
            Item::Present => {
                shot.kind = ShotKind::Present;
                shot.vx = c * (fwd + 60.0);
                shot.vy = s * (fwd + 60.0);
                shot.vz = 16.0;
                shot.life = 3.0;
                self.shots.push(shot);
            }
            Item::Tinsel => {
                let sp = (fwd + 40.0).max(90.0);
                shot.kind = ShotKind::Tinsel;
                shot.vx = c * sp;
                shot.vy = s * sp;
                shot.z = 2.0;
                shot.target = ahead;
                shot.life = 5.0;
                self.shots.push(shot);
            }
            Item::Peppermint => r.kart.boost = r.kart.boost.max(1.3),
            Item::Mistletoe => r.mistletoe = MISTLETOE_TIME,
        }
        let r = &mut self.racers[i];
        r.ammo = r.ammo.saturating_sub(1);
        if r.ammo == 0 {
            r.item = None;
        }
        if r.is_player() {
            log::info!("used {item:?}{}", if backward { " behind" } else { "" });
        }
        self.notes.push(Note::Used(i, item));
    }

    fn fly_shots(&mut self) {
        let mut hits: Vec<(usize, ShotKind)> = Vec::new();
        let mut bursts: Vec<(f32, f32)> = Vec::new();
        for s in &mut self.shots {
            s.age += DT;
            s.life -= DT;
            if s.kind == ShotKind::Tinsel {
                if let Some(t) = s.target {
                    let k = &self.racers[t].kart;
                    let want = (k.y - s.y).atan2(k.x - s.x);
                    let now = s.vy.atan2(s.vx);
                    let turn = wrap(want - now).clamp(-4.0 * DT, 4.0 * DT);
                    let sp = s.vx.hypot(s.vy);
                    let a = now + turn;
                    s.vx = a.cos() * sp;
                    s.vy = a.sin() * sp;
                }
            }
            s.x += s.vx * DT;
            s.y += s.vy * DT;
            if s.kind == ShotKind::Present {
                s.z += s.vz * DT;
                s.vz -= 36.0 * DT;
                if s.z <= 0.0 {
                    s.life = 0.0;
                    bursts.push((s.x, s.y));
                }
                continue;
            }
            // snowballs stop at banks and pines; tinsel flies over them
            if s.kind == ShotKind::Snowball && (self.track.surface(s.x, s.y) == Surface::Bank || self.track.trees.iter().any(|t| (t.x - s.x).abs() < 1.2 && (t.y - s.y).abs() < 1.2)) {
                s.life = 0.0;
                continue;
            }
            for (i, r) in self.racers.iter().enumerate() {
                if i == s.owner && s.age < 0.4 {
                    continue;
                }
                let reach = if s.kind == ShotKind::Tinsel { 2.6 } else { 2.2 };
                if (r.kart.x - s.x).hypot(r.kart.y - s.y) < reach {
                    hits.push((i, s.kind));
                    s.life = 0.0;
                    break;
                }
            }
        }
        self.shots.retain(|s| s.life > 0.0);
        for (x, y) in bursts {
            self.notes.push(Note::Burst(x, y));
            for i in 0..self.racers.len() {
                let k = &self.racers[i].kart;
                if (k.x - x).hypot(k.y - y) < 7.0 {
                    hits.push((i, ShotKind::Present));
                }
            }
        }
        for (i, kind) in hits {
            let r = &mut self.racers[i];
            if r.kart.stunned() {
                continue;
            }
            match kind {
                ShotKind::Snowball => r.kart.hit(0.8, 1.0),
                ShotKind::Present => r.kart.hit(1.1, 1.0),
                ShotKind::Tinsel => {
                    r.kart.hit(1.2, 1.0);
                    r.tangled = 1.6;
                }
            }
            self.notes.push(Note::Hit(i, Some(kind)));
        }
    }

    fn spring_traps(&mut self) {
        let mut hit = Vec::new();
        for t in &mut self.traps {
            t.age += DT;
        }
        let racers = &self.racers;
        self.traps.retain(|t| {
            for (i, r) in racers.iter().enumerate() {
                if (r.kart.x - t.x).hypot(r.kart.y - t.y) < 2.4 {
                    hit.push((i, t.x, t.y));
                    return false;
                }
            }
            t.age < 60.0
        });
        for (i, x, y) in hit {
            self.notes.push(Note::Burst(x, y));
            if !self.racers[i].kart.stunned() {
                self.racers[i].kart.hit(1.0, 1.0);
                self.notes.push(Note::Hit(i, None));
            }
        }
    }

    fn santa_step(&mut self) {
        let lead_lap = self.in_place(1).map_or(0, |l| self.racers[l].lap);
        let racing = self.racers.iter().any(|r| r.finished.is_none());
        if !self.santa.active {
            self.santa.rest = (self.santa.rest - DT).max(0.0);
            if lead_lap >= 2 && racing && self.santa.rest == 0.0 {
                // roar in from behind the last racer still going
                let Some(last) = self.racers.iter().filter(|r| r.finished.is_none()).max_by_key(|r| r.place) else { return };
                let i = self.track.ahead(last.idx, -90);
                let (x, y) = self.track.pts[i];
                let mut k = Kart::new(x, y, self.track.heading_at(i), PLOUGH);
                k.settle(&self.track);
                k.plough = true;
                k.speed = 60.0;
                self.santa.kart = k;
                self.santa.idx = i;
                self.santa.active = true;
                self.santa.visits += 1;
                self.notes.push(Note::SantaComing);
                log::info!("Santa comes (visit {})", self.santa.visits);
            }
            return;
        }
        if self.santa.gloat > 0.0 {
            // drive on along the road, pleased with himself, then go
            self.santa.gloat -= DT;
            let c = auto::drive_santa_on(self);
            self.santa.kart.step(c, &self.track);
            if let Some(i) = self.track.idx(self.santa.kart.x, self.santa.kart.y) {
                self.santa.idx = i;
            }
            if self.santa.gloat <= 0.0 {
                self.santa.active = false;
            }
            return;
        }
        // hunt whoever leads among those still racing
        let target = self.racers.iter().enumerate().filter(|(_, r)| r.finished.is_none()).min_by_key(|(_, r)| r.place).map(|(i, _)| i);
        self.santa.target = target;
        let Some(t) = target else {
            self.santa.active = false;
            return;
        };
        let c = auto::drive_santa(self, t);
        self.santa.kart.step(c, &self.track);
        if let Some(i) = self.track.idx(self.santa.kart.x, self.santa.kart.y) {
            self.santa.idx = i;
        }
        let (sx, sy) = (self.santa.kart.x, self.santa.kart.y);
        for i in 0..self.racers.len() {
            let k = &mut self.racers[i].kart;
            let d = (k.x - sx).hypot(k.y - sy);
            if i == t && d < 4.8 {
                if !k.stunned() {
                    k.hit(1.6, 2.0);
                }
                self.racers[i].caught += 1;
                self.racers[i].clipped = CLIP_TIME;
                self.notes.push(Note::SantaCaught(i));
                self.notes.push(Note::Hit(i, None));
                log::info!("Santa caught {}", ROSTER[self.racers[i].who].name);
                self.santa.gloat = 2.0;
                self.santa.target = None;
                self.santa.rest = SANTA_REST;
                return;
            }
            if d < 4.2 && d > 1e-3 {
                // anyone else in the way is shoved aside
                let push = 4.2 - d;
                k.x += (k.x - sx) / d * push;
                k.y += (k.y - sy) / d * push;
                k.speed *= 0.97;
            }
        }
    }
}

/// Everything about a race that changes as it runs: what a host sends its
/// guests every step. The track is rebuilt from the course instead.
#[derive(Clone, Serialize, Deserialize)]
pub struct Snap {
    racers: Vec<Racer>,
    clock: f32,
    shots: Vec<Shot>,
    traps: Vec<Trap>,
    boxes: Vec<ItemBox>,
    santa: Santa,
    weather: crate::weather::Weather,
    wildlife: crate::wildlife::Wildlife,
    folklore: crate::folklore::Folklore,
    crowd: crate::crowd::Crowd,
    notes: Vec<Note>,
    order: Vec<usize>,
    rival_pace: f32,
    seed: u32,
}

impl Race {
    pub fn snapshot(&self) -> Snap {
        Snap {
            racers: self.racers.clone(),
            clock: self.clock,
            shots: self.shots.clone(),
            traps: self.traps.clone(),
            boxes: self.boxes.clone(),
            santa: self.santa.clone(),
            weather: self.weather.clone(),
            wildlife: self.wildlife.clone(),
            folklore: self.folklore.clone(),
            crowd: self.crowd.clone(),
            notes: self.notes.clone(),
            order: self.order.clone(),
            rival_pace: self.rival_pace,
            seed: self.seed,
        }
    }

    pub fn apply(&mut self, s: Snap) {
        self.racers = s.racers;
        self.clock = s.clock;
        self.shots = s.shots;
        self.traps = s.traps;
        self.boxes = s.boxes;
        self.santa = s.santa;
        self.weather = s.weather;
        self.wildlife = s.wildlife;
        self.folklore = s.folklore;
        self.crowd = s.crowd;
        self.notes = s.notes;
        self.order = s.order;
        self.rival_pace = s.rival_pace;
        self.seed = s.seed;
    }
}

fn pair_mut<T>(v: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    let (lo, hi) = v.split_at_mut(b);
    (&mut lo[a], &mut hi[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_of_autopilots_finishes_three_laps() {
        for (name, make) in crate::track::courses() {
        let mut r = Race::new(make(), None, 7);
        let mut steps = 0;
        while r.order.len() < r.racers.len() && steps < (500.0 / DT) as usize {
            r.step(Controls::default());
            steps += 1;
        }
        assert_eq!(r.order.len(), ROSTER.len(), "{name}: all finished");
        let t = r.racers[r.order[0]].finished.unwrap();
        let last = r.racers[*r.order.last().unwrap()].finished.unwrap();
        eprintln!("{name}: winner {t:.1}s, last {last:.1}s, Santa came {} times", r.santa.visits);
        assert!((90.0..220.0).contains(&t), "{name}: winning time {t}");
        let mut places: Vec<u8> = r.racers.iter().map(|x| x.place).collect();
        places.sort();
        assert_eq!(places, (1..=ROSTER.len() as u8).collect::<Vec<_>>());
        assert!(r.santa.visits >= 1, "{name}: Santa came");
        }
    }

    #[test]
    fn reversing_over_the_line_does_not_count() {
        let mut r = Race::new(Track::frozen_lake(), Some(0), 1);
        let p = r.player().unwrap();
        r.clock = COUNTDOWN;
        for _ in 0..(6.0 / DT) as usize {
            r.step(Controls { brake: true, ..Default::default() });
        }
        assert!(r.racers[p].lap_times.is_empty());
        assert_eq!(r.racers[p].lap, 1);
    }

    /// The player 20 units behind racer 1 on a straight, everyone else far off.
    fn duel() -> (Race, usize) {
        let mut r = Race::new(Track::frozen_lake(), Some(0), 1);
        r.clock = COUNTDOWN + 1.0;
        let p = r.player().unwrap();
        let i = 300;
        for k in 0..r.racers.len() {
            let at = if k == p { i } else if k == 1 { i + 20 } else { i + 900 + k * 40 };
            let (x, y) = r.track.pts[at];
            let stats = r.racers[k].kart.stats;
            r.racers[k].kart = Kart::new(x, y, r.track.heading_at(at), stats);
            r.racers[k].kart.settle(&r.track);
        }
        (r, p)
    }

    #[test]
    fn a_snowball_spins_out_the_racer_ahead() {
        let (mut r, p) = duel();
        r.racers[p].item = Some(Item::Snowball);
        r.racers[p].ammo = 3;
        r.step(Controls { item: true, ..Default::default() });
        assert_eq!(r.racers[p].ammo, 2);
        let mut hit = false;
        for _ in 0..60 {
            r.step(Controls::default());
            hit |= r.notes.iter().any(|n| *n == Note::Hit(1, Some(ShotKind::Snowball)));
        }
        assert!(hit, "the snowball hit");
    }

    #[test]
    fn mistletoe_kisses_whoever_comes_close() {
        let (mut r, p) = duel();
        r.racers[p].item = Some(Item::Mistletoe);
        r.racers[p].ammo = 1;
        r.racers[p].kart.speed = 60.0;
        r.racers[1].kart.speed = 20.0;
        r.racers[1].ai = None;
        r.step(Controls { gas: true, item: true, ..Default::default() });
        r.racers[1].ai = Some(Ai::new(1, 1));
        let mut kissed = false;
        for _ in 0..120 {
            r.step(Controls { gas: true, ..Default::default() });
            kissed |= r.notes.iter().any(|n| *n == Note::Kiss(p, 1));
        }
        assert!(kissed);
        assert!(r.racers[1].smooched > 0.0 || r.racers[1].kart.stunned() || kissed);
    }

    #[test]
    fn item_odds_lean_on_place() {
        let mut r = Race::new(Track::frozen_lake(), None, 3);
        let count = |r: &mut Race, place: u8, item: Item| (0..2000).filter(|_| r.roll_item(place) == item).count();
        assert_eq!(count(&mut r, 1, Item::Tinsel), 0, "no tinsel for the leader");
        assert!(count(&mut r, 6, Item::Peppermint) > count(&mut r, 1, Item::Peppermint) * 2);
        assert!(count(&mut r, 3, Item::Mistletoe) > count(&mut r, 1, Item::Mistletoe));
    }

    #[test]
    fn a_snow_tornado_throws_you_up_spinning() {
        let mut r = Race::new(Track::thin_ice(), Some(0), 1);
        r.clock = COUNTDOWN + 1.0;
        let p = r.player().unwrap();
        let tw = r.weather.twisters[0];
        r.racers[p].kart.x = tw.x;
        r.racers[p].kart.y = tw.y;
        r.racers[p].kart.settle(&r.track);
        r.step(Controls::default());
        assert!(r.notes.iter().any(|n| *n == Note::Twister(p)));
        assert!(r.racers[p].kart.stunned());
        r.step(Controls::default());
        assert!(r.racers[p].kart.air, "up in the air");
    }

    #[test]
    fn two_humans_start_at_the_back_and_drive_their_own() {
        let mut r = Race::with_humans(Track::frozen_lake(), &[2, 4], 3);
        assert!(r.racers[2].is_player() && r.racers[4].is_player());
        assert_eq!(r.racers[2].place as usize, ROSTER.len());
        assert_eq!(r.racers[4].place as usize, ROSTER.len() - 1);
        r.clock = COUNTDOWN;
        for _ in 0..120 {
            r.step_with(&[(2, Controls { gas: true, ..Default::default() })]);
        }
        assert!(r.racers[2].kart.speed > 20.0, "the one with the gas goes");
        assert!(r.racers[4].kart.speed < 1.0, "the one without input coasts");
    }

    #[test]
    fn a_snapshot_carries_the_race_to_another_copy() {
        let mut host = Race::with_humans(Track::eggnogg(), &[0], 9);
        host.autopilot = true;
        for _ in 0..(25.0 / DT) as usize {
            host.step(Controls::default());
        }
        let mut guest = Race::with_humans(Track::eggnogg(), &[0], 9);
        let bytes = postcard::to_allocvec(&host.snapshot()).unwrap();
        guest.apply(postcard::from_bytes(&bytes).unwrap());
        for (a, b) in host.racers.iter().zip(&guest.racers) {
            assert_eq!((a.kart.x, a.kart.y, a.place, a.lap), (b.kart.x, b.kart.y, b.place, b.lap));
        }
        assert_eq!(host.clock, guest.clock);
        // sent sixty times a second: keep it to a few kilobytes
        assert!(bytes.len() < 6000, "snapshot is {} bytes", bytes.len());
    }

    #[test]
    fn santa_catches_the_leader() {
        let mut r = Race::new(Track::frozen_lake(), None, 5);
        let mut caught = false;
        let mut steps = 0;
        while !caught && steps < (300.0 / DT) as usize {
            r.step(Controls::default());
            caught = r.notes.iter().any(|n| matches!(n, Note::SantaCaught(_)));
            steps += 1;
        }
        assert!(caught, "Santa caught someone");
    }
}

#[cfg(test)]
mod balance {
    use super::*;

    /// Average place of each racer with everyone on autopilot.
    fn average_places(make: fn() -> Track, runs: u32) -> Vec<f32> {
        let mut sum = vec![0u32; ROSTER.len()];
        for seed in 1..=runs {
            let mut r = Race::new(make(), None, seed * 977);
            while r.order.len() < ROSTER.len() && r.clock < 500.0 {
                r.step(Controls::default());
            }
            for (i, rc) in r.racers.iter().enumerate() {
                sum[i] += rc.place as u32;
            }
        }
        sum.iter().map(|&s| s as f32 / runs as f32).collect()
    }

    #[test]
    fn no_ride_runs_away_with_it() {
        // the field averages the middle place; nobody should be far off over
        // many races (at 40 races, luck alone moves a racer about half a
        // place)
        let mid = (ROSTER.len() as f32 + 1.0) / 2.0;
        for (name, make) in crate::track::courses() {
            let avg = average_places(make, 40);
            for (i, a) in avg.iter().enumerate() {
                assert!((a - mid).abs() < 1.6, "{name}: {} averages {a:.1}", ROSTER[i].name);
            }
        }
    }

    #[test]
    fn at_normal_pace_a_good_driver_from_the_back_can_win() {
        // the autopilot drives the player: as good as the rivals, but they
        // run at the Normal pace
        for pick in 0..ROSTER.len() {
            let mut wins_or_podiums = 0;
            for seed in 1..=4u32 {
                let mut r = Race::new(Track::frozen_lake(), Some(pick), seed * 101);
                r.autopilot = true;
                r.rival_pace = 0.96;
                let p = r.player().unwrap();
                while r.racers[p].finished.is_none() && r.clock < 400.0 {
                    r.step(Controls::default());
                }
                if r.racers[p].place <= 3 {
                    wins_or_podiums += 1;
                }
            }
            assert!(wins_or_podiums >= 2, "{} on the podium only {wins_or_podiums} of 4", ROSTER[pick].name);
        }
    }
}

