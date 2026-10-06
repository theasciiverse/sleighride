//! Screens and the HUD around a race.
//!
//! The title (splash) screen runs a race on autopilot behind the logo, with
//! the racers turning on a showcase and a course picker. A race runs the
//! countdown, three laps and a results card; after the finish the autopilot
//! takes the sleigh round a cool-down lap.

use crate::input::{Cmd, Input};
use crate::kart::{Controls, Event, DT};
use crate::model::{Model, Pose, FRAMES};
use crate::race::{Item, Note, Race, COUNTDOWN, ITEMS, LAPS};
use crate::render::{self, accent_style, big_text, big_text_at, big_width, panel_style, put_text, Camera, Canvas, Particle, Rgb, Scene};
use crate::roster::{ROSTER, SANTA_COLOUR};
use crate::track::{courses, Surface};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;
use std::sync::OnceLock;
use std::time::Instant;

/// The name on the splash screen.
pub const TITLE: &str = "SLEIGHRIDE";

/// The story, a page at a time.
const STORY: [&str; 4] = [
    "Christmas morning, six o'clock. Every present delivered, every chimney climbed. Santa kicks off his boots, sinks into his armchair by the workshop fire, and is snoring before his cocoa has cooled.",
    "The elves have worked every single day since last January. Bob holds up a jangling ring of keys: the big sleigh, the reindeer stables, the snowmobile shed. \"One lap of the lake,\" he whispers. \"He'll never know.\"",
    "Three laps later Santa wakes to an empty garage. Only the snowplough is left. His cocoa is cold. His boots are on the wrong feet.",
    "He has a list. He is checking it twice. And whoever is out in front is clearly the ringleader, and is getting a good clip round the ear.",
];
/// Words a second as the story fades in, and how long each word takes.
const WORDS_PER_SEC: f32 = 4.0;
const WORD_FADE: f32 = 0.8;

/// Seconds until the whole page is showing.
fn page_time(page: usize) -> f32 {
    STORY[page].split(' ').count() as f32 / WORDS_PER_SEC + WORD_FADE
}

/// Difficulty: its name and the rivals' pace (a share of their top speed).
/// The rivals drift perfectly, so at full pace they are a hard race; a
/// little slower gives a person a fair one.
pub const DIFFICULTY: [(&str, f32); 3] = [("Easy", 0.93), ("Normal", 0.96), ("Hard", 1.0)];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Title,
    /// Why Santa is after you; once per run, before the racer choice.
    Story,
    /// Choose your racer.
    Select,
    Race,
}

pub struct Game {
    pub phase: Phase,
    pub course: usize,
    /// Roster index of the racer the player drives.
    pub pick: usize,
    /// Index into DIFFICULTY.
    pub level: usize,
    /// The story has been shown this run; and which page is up.
    story_seen: bool,
    page: usize,
    pub race: Race,
    cam: Camera,
    particles: Vec<Particle>,
    pub paused: bool,
    help: bool,
    pub exit: bool,
    /// A short message in the middle of the screen, how long it stays, its
    /// colour.
    flash: Option<(String, f32, Rgb)>,
    /// Seconds on this screen.
    t: f32,
    seed: u32,
}

pub fn fmt_time(t: f32) -> String {
    let m = (t / 60.0) as u32;
    let s = t - m as f32 * 60.0;
    format!("{m}:{s:05.2}")
}

fn ordinal(n: u8) -> String {
    let suffix = match n {
        1 => "ST",
        2 => "ND",
        3 => "RD",
        _ => "TH",
    };
    format!("{n}{suffix}")
}

impl Game {
    pub fn new(phase: Phase, course: usize, pick: usize, level: usize) -> Game {
        let seed = 0x5EED ^ (course as u32 + 1) * 7919;
        let mut race = Race::new((courses()[course].1)(), (phase == Phase::Race).then_some(pick), seed);
        race.rival_pace = DIFFICULTY[level].1;
        let cam = Camera::behind(&race.racers[pick].kart);
        let mut g = Game { phase, course, pick, level, story_seen: false, page: 0, race, cam, particles: Vec::new(), paused: false, help: false, exit: false, flash: None, t: 0.0, seed: seed | 1 };
        if phase != Phase::Race {
            // start the attract race already under way
            for _ in 0..(8.0 / DT) as usize {
                g.step_race(Controls::default());
            }
        }
        g
    }

    fn switch(&mut self, p: Phase, course: usize, input: &mut Input) {
        log::info!("phase {:?} -> {p:?} on {} as {}, {}", self.phase, courses()[course].0, ROSTER[self.pick].name, DIFFICULTY[self.level].0);
        let seen = self.story_seen;
        *self = Game::new(p, course, self.pick, self.level);
        self.story_seen = seen;
        input.clear();
    }

    /// Between the title and the racer choice: same race behind, new screen.
    fn screen(&mut self, p: Phase) {
        log::info!("phase {:?} -> {p:?}", self.phase);
        self.phase = p;
        self.t = 0.0;
    }

    fn rand(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed & 0xffff) as f32 / 65535.0
    }

    /// The racer the camera follows: the player, or their pick on the menus.
    fn me(&self) -> usize {
        self.race.player().unwrap_or(self.pick)
    }

    pub fn tick(&mut self, input: &mut Input) {
        let cmds = std::mem::take(&mut input.cmds);
        let n = courses().len();
        for c in cmds {
            log::debug!("cmd {c:?}");
            match (self.phase, c) {
                (_, Cmd::Exit) => self.exit = true,
                (_, Cmd::Help) => self.help = !self.help,
                (_, Cmd::Back) | (_, Cmd::Confirm) if self.help => self.help = false,
                (Phase::Title, Cmd::Confirm) if !self.story_seen => {
                    self.page = 0;
                    self.screen(Phase::Story);
                }
                (Phase::Title, Cmd::Confirm) => self.screen(Phase::Select),
                (Phase::Story, Cmd::Confirm) => {
                    if self.t < page_time(self.page) {
                        // show the whole page first
                        self.t = 1e3;
                    } else if self.page + 1 < STORY.len() {
                        self.page += 1;
                        self.t = 0.0;
                    } else {
                        self.story_seen = true;
                        self.screen(Phase::Select);
                    }
                }
                (Phase::Story, Cmd::Back) => {
                    self.story_seen = true;
                    self.screen(Phase::Select);
                }
                (Phase::Title, Cmd::Back) => self.exit = true,
                (Phase::Select, Cmd::Confirm) => return self.switch(Phase::Race, self.course, input),
                (Phase::Select, Cmd::Back) => self.screen(Phase::Title),
                (Phase::Select, Cmd::Left) => self.pick = (self.pick + ROSTER.len() - 1) % ROSTER.len(),
                (Phase::Select, Cmd::Right) => self.pick = (self.pick + 1) % ROSTER.len(),
                (Phase::Title, Cmd::Left) => return self.switch(Phase::Title, (self.course + n - 1) % n, input),
                (Phase::Title, Cmd::Right) => return self.switch(Phase::Title, (self.course + 1) % n, input),
                (Phase::Title, Cmd::Up) => self.level = (self.level + 1).min(DIFFICULTY.len() - 1),
                (Phase::Title, Cmd::Down) => self.level = self.level.saturating_sub(1),
                (Phase::Race, Cmd::Pause) => self.paused = !self.paused,
                (Phase::Race, Cmd::Back) if self.paused => return self.switch(Phase::Title, self.course, input),
                (Phase::Race, Cmd::Back) => self.paused = true,
                (Phase::Race, Cmd::Restart) => return self.switch(Phase::Race, self.course, input),
                (Phase::Race, Cmd::Confirm) if self.paused => self.paused = false,
                (Phase::Race, Cmd::Confirm) if self.player_done() => return self.switch(Phase::Race, self.course, input),
                _ => {}
            }
        }
        if self.paused || self.help && self.phase == Phase::Race {
            return;
        }
        let c = if self.phase == Phase::Race { input.controls(Instant::now()) } else { Controls::default() };
        self.step_race(c);
    }

    fn player_done(&self) -> bool {
        self.race.player().is_some_and(|p| self.race.racers[p].finished.is_some())
    }

    fn step_race(&mut self, c: Controls) {
        self.t += DT;
        let me = self.me();
        let before = self.race.racers[me].last;
        self.race.step(c);
        let after = self.race.racers[me].last;
        if self.phase == Phase::Race && after != before {
            log::debug!("controls {after:?}");
        }
        self.cam.follow(&self.race.racers[me].kart, &self.race.track, DT);
        self.react(me);
        self.spawn_particles();
        for p in &mut self.particles {
            p.x += p.vx * DT;
            p.y += p.vy * DT;
            p.z += p.vz * DT;
            p.vz -= 30.0 * DT;
            if p.z < 0.0 {
                p.z = 0.0;
                p.vz = 0.0;
            }
            p.life -= DT * 1.8;
        }
        self.particles.retain(|p| p.life > 0.0);
        if let Some((_, t, _)) = &mut self.flash {
            *t -= DT;
            if *t <= 0.0 {
                self.flash = None;
            }
        }
    }

    /// Messages and effects for what just happened.
    fn react(&mut self, me: usize) {
        let playing = self.phase == Phase::Race;
        let events = self.race.racers[me].kart.events.clone();
        for e in events {
            if playing {
                log::info!("event {e:?}");
            }
            let (text, col): (&str, Rgb) = match e {
                Event::RocketStart => ("ROCKET START!", (255, 210, 80)),
                Event::GoodStart => ("GOOD START", (180, 230, 255)),
                Event::Stall => ("STALLED", (240, 90, 80)),
                _ => continue,
            };
            self.flash = Some((text.into(), 1.2, col));
        }
        let notes = self.race.notes.clone();
        for n in notes {
            match n {
                Note::Burst(x, y) => self.confetti(x, y),
                Note::SantaComing if playing => {
                    let text = if self.race.santa.visits <= 1 { "SANTA'S AWAKE!" } else { "SANTA'S BACK!" };
                    self.flash = Some((text.into(), 2.0, SANTA_COLOUR));
                }
                Note::SantaCaught(i) if i == me && playing => self.flash = Some(("YOU'RE ON|SANTA'S|NAUGHTY LIST!".into(), 2.2, SANTA_COLOUR)),
                Note::Kiss(a, b) if (a == me || b == me) && playing => self.flash = Some(("SMOOCH!".into(), 1.0, (255, 110, 160))),
                Note::Got(i, item) if i == me && playing => log::info!("got {item:?}"),
                _ => {}
            }
        }
    }

    fn confetti(&mut self, x: f32, y: f32) {
        const COLS: [Rgb; 5] = [(230, 50, 70), (250, 210, 70), (60, 200, 110), (90, 150, 255), (250, 250, 250)];
        for k in 0..24 {
            let (a, b, c) = (self.rand(), self.rand(), self.rand());
            let ang = a * std::f32::consts::TAU;
            self.particles.push(Particle { x, y, z: 1.5, vx: ang.cos() * (6.0 + b * 10.0), vy: ang.sin() * (6.0 + b * 10.0), vz: 8.0 + c * 12.0, life: 1.0 + c * 0.6, colour: COLS[k % 5] });
        }
    }

    fn spawn_particles(&mut self) {
        let (cx, cy) = (self.cam.x, self.cam.y);
        for i in 0..self.race.racers.len() {
            let k = self.race.racers[i].kart.clone();
            if (k.x - cx).hypot(k.y - cy) > 160.0 {
                continue;
            }
            let (c, s) = (k.heading.cos(), k.heading.sin());
            let spray = k.speed.abs() > 12.0 && (k.drift != 0 || k.surface == Surface::Powder);
            let tier = k.tier();
            let emit = |g: &mut Game, side: f32, colour: Rgb, up: f32, n: usize| {
                for _ in 0..n {
                    let (r1, r2, r3) = (g.rand(), g.rand(), g.rand());
                    let bx = k.x - c * 1.8 + -s * side * 1.4;
                    let by = k.y - s * 1.8 + c * side * 1.4;
                    let out = side * (4.0 + r1 * 8.0);
                    g.particles.push(Particle {
                        x: bx,
                        y: by,
                        z: 0.2,
                        // mostly carried along with the racer, so it hangs at its sides
                        vx: -c * (2.0 + r2 * 4.0) + -s * out + k.travel.cos() * k.speed * 0.85,
                        vy: -s * (2.0 + r2 * 4.0) + c * out + k.travel.sin() * k.speed * 0.85,
                        vz: up * (0.6 + r3),
                        life: 0.5 + r3 * 0.4,
                        colour,
                    });
                }
            };
            if spray {
                emit(self, 1.0, (250, 252, 255), 9.0, 2);
                emit(self, -1.0, (250, 252, 255), 9.0, 2);
            }
            if tier > 0 {
                emit(self, -(k.drift as f32), render::tier_colour(tier, self.race.clock), 5.0, 1);
            }
            if k.boost > 0.0 {
                emit(self, 0.3, (255, 220, 120), 3.0, 1);
                emit(self, -0.3, (255, 240, 200), 3.0, 1);
            }
        }
        if self.particles.len() > 900 {
            let extra = self.particles.len() - 900;
            self.particles.drain(0..extra);
        }
    }

    fn scene_canvas(&self, cols: u16, rows: u16) -> Canvas {
        let mut cv = Canvas::new(cols, rows);
        let scene = Scene { race: &self.race, cam: self.cam, particles: &self.particles, time: self.race.clock };
        render::draw_scene(&mut cv, &scene);
        cv
    }

    fn overlay(&self, cv: &mut Canvas) {
        let big = ((cv.w as i32) / 70).clamp(1, 4);
        match self.phase {
            Phase::Title => self.draw_title_canvas(cv, big),
            Phase::Story => {}
            Phase::Select => self.draw_select_canvas(cv, big),
            Phase::Race => self.draw_race_canvas(cv, big),
        }
    }

    pub fn draw(&self, f: &mut Frame, input: &Input) {
        let area = f.area();
        let mut cv = if self.phase == Phase::Story {
            // the story has its own pictures, not the race
            let mut cv = Canvas::new(area.width, area.height);
            crate::story::draw(&mut cv, self.page, self.t);
            cv
        } else {
            self.scene_canvas(area.width, area.height)
        };
        self.overlay(&mut cv);
        let buf = f.buffer_mut();
        cv.flush(buf, area);
        match self.phase {
            Phase::Title => self.draw_title_hud(buf, area, input),
            Phase::Story => self.draw_story(buf, area),
            Phase::Select => self.draw_select_hud(buf, area),
            Phase::Race => self.draw_race_hud(buf, area, input),
        }
        if self.help {
            draw_help(buf, area, input);
        }
    }

    fn draw_title_canvas(&self, cv: &mut Canvas, big: i32) {
        dim(cv);
        big_text(cv, TITLE, (cv.h as i32) / 7, big + 1, (236, 60, 70));
        // the showcase: each racer in turn, turning slowly on a lit stage
        let k = ((self.t / 3.0) as usize) % ROSTER.len();
        stage(cv, k, self.t, cv.w as f32 / 2.0, cv.h as f32 * 0.64);
    }

    fn draw_select_canvas(&self, cv: &mut Canvas, big: i32) {
        dim(cv);
        big_text(cv, "CHOOSE YOUR RACER", (cv.h as i32) / 8, big, (250, 210, 70));
        stage(cv, self.pick, self.t, cv.w as f32 * 0.38, cv.h as f32 * 0.66);
    }

    fn draw_story(&self, buf: &mut Buffer, area: Rect) {
        let width = (area.width as usize).saturating_sub(10).clamp(20, 60);
        // wrap by words into lines, keeping each word's place in the page
        let mut lines: Vec<Vec<(usize, &str)>> = vec![Vec::new()];
        let mut len = 0;
        for (k, word) in STORY[self.page].split(' ').enumerate() {
            if len > 0 && len + 1 + word.chars().count() > width {
                lines.push(Vec::new());
                len = 0;
            }
            len += word.chars().count() + if len > 0 { 1 } else { 0 };
            lines.last_mut().unwrap().push((k, word));
        }
        let band = (lines.len() + 2) as u16;
        let y0 = (area.height as f32 * 0.66) as u16;
        let y0 = y0.min(area.height.saturating_sub(band + 2));
        let bg = (14u8, 18u8, 44u8);
        for y in 0..band {
            put_text(buf, 0, y0 + y, &" ".repeat(area.width as usize), panel_style());
        }
        for (i, line) in lines.iter().enumerate() {
            let text_len: usize = line.iter().map(|(_, w)| w.chars().count()).sum::<usize>() + line.len().saturating_sub(1);
            let mut x = area.width.saturating_sub(text_len as u16) / 2;
            for (k, word) in line {
                let a = ((self.t - *k as f32 / WORDS_PER_SEC) / WORD_FADE).clamp(0.0, 1.0);
                let c = render::mix(bg, (240, 244, 255), a);
                put_text(buf, x, y0 + 1 + i as u16, word, Style::default().fg(Color::Rgb(c.0, c.1, c.2)).bg(Color::Rgb(bg.0, bg.1, bg.2)));
                x += word.chars().count() as u16 + 1;
            }
        }
        let foot = format!("{} / {}    Enter: next    Esc: skip", self.page + 1, STORY.len());
        put_text(buf, area.width.saturating_sub(foot.len() as u16) / 2, area.height.saturating_sub(1), &foot, accent_style());
    }

    fn draw_select_hud(&self, buf: &mut Buffer, area: Rect) {
        let who = &ROSTER[self.pick];
        let c = who.colour();
        let x = (area.width as f32 * 0.58) as u16;
        let y = (area.height as f32 * 0.30) as u16;
        put_text(buf, x, y, &format!(" {} ", who.name), Style::default().fg(Color::Rgb(c.0, c.1, c.2)).bg(Color::Rgb(14, 18, 44)).add_modifier(Modifier::BOLD));
        put_text(buf, x, y + 1, &format!(" on {} ", who.ride_name), panel_style());
        // stats as bars, scaled across the roster
        let stat = |f: fn(&crate::kart::Stats) -> f32| {
            let v = f(&who.stats);
            let (lo, hi) = ROSTER.iter().map(|w| f(&w.stats)).fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(x), b.max(x)));
            2 + ((v - lo) / (hi - lo).max(1e-3) * 8.0).round() as usize
        };
        let rows: [(&str, usize); 4] = [("SPEED   ", stat(|s| s.top)), ("ACCEL   ", stat(|s| s.accel)), ("HANDLING", stat(|s| s.turn + s.grip * 0.1)), ("WEIGHT  ", stat(|s| s.weight))];
        for (i, (label, n)) in rows.iter().enumerate() {
            let yy = y + 3 + i as u16;
            put_text(buf, x, yy, &format!(" {label} "), panel_style());
            for j in 0..10u16 {
                let on = (j as usize) < *n;
                put_text(buf, x + 11 + j, yy, "█", panel_style().fg(if on { Color::Rgb(c.0, c.1, c.2) } else { Color::Rgb(50, 56, 90) }));
            }
        }
        let keys = "Left/Right: choose    Enter: race    Esc: back";
        put_text(buf, area.width.saturating_sub(keys.len() as u16) / 2, area.height.saturating_sub(3), keys, accent_style());
        let pos = format!("{} of {}", self.pick + 1, ROSTER.len());
        put_text(buf, area.width.saturating_sub(pos.len() as u16) / 2, area.height.saturating_sub(4), &pos, panel_style());
    }

    fn draw_title_hud(&self, buf: &mut Buffer, area: Rect, input: &Input) {
        let k = ((self.t / 3.0) as usize) % ROSTER.len();
        let who = &ROSTER[k];
        let name = format!(" {} on {} ", who.name, who.ride_name);
        let y_name = (area.height as f32 * 0.70) as u16;
        let c = who.colour();
        put_text(buf, area.width.saturating_sub(name.len() as u16) / 2, y_name, &name, Style::default().fg(Color::Rgb(c.0, c.1, c.2)).bg(Color::Rgb(14, 18, 44)).add_modifier(Modifier::BOLD));
        let tag = ["Christmas morning. Santa's asleep. The elves have found the keys.", "He is NOT going to be happy."];
        let y0 = (area.height as f32 * 0.30) as u16;
        for (i, l) in tag.iter().enumerate() {
            put_text(buf, area.width.saturating_sub(l.chars().count() as u16) / 2, y0 + i as u16, l, panel_style());
        }
        let course = format!("<  {}  >", courses()[self.course].0);
        let y1 = area.height.saturating_sub(4);
        put_text(buf, area.width.saturating_sub(course.len() as u16) / 2, y1, &course, accent_style());
        let level = format!("Rivals:  {}  (Up/Down)", DIFFICULTY[self.level].0);
        put_text(buf, area.width.saturating_sub(level.len() as u16) / 2, y1.saturating_sub(1), &level, panel_style());
        let keys = "Enter or Space: choose racer    Left/Right: course    H: controls    Esc: quit";
        put_text(buf, area.width.saturating_sub(keys.len() as u16) / 2, y1 + 1, keys, panel_style());
        if !input.exact {
            let l = "(no kitty keyboard here: Up starts the gas, Down stops it, Space toggles drift)";
            put_text(buf, area.width.saturating_sub(l.len() as u16) / 2, y1 + 2, l, panel_style());
        }
    }

    fn draw_race_canvas(&self, cv: &mut Canvas, big: i32) {
        let ch = cv.h as i32;
        let r = &self.race;
        let me = self.me();
        let p = &r.racers[me];
        if r.counting_down() {
            let n = (COUNTDOWN - r.clock).ceil() as i32;
            big_text(cv, &n.to_string(), ch * 2 / 5, big * 2, (255, 210, 80));
        } else if r.time() < 0.8 {
            big_text(cv, "GO!", ch * 2 / 5, big * 2, (90, 230, 120));
        } else if p.finished.is_some() {
            big_text(cv, "FINISH", ch / 4, big + 1, (255, 210, 80));
        } else if p.wrong_way > 0.8 && (r.clock * 3.0) as i32 % 2 == 0 {
            big_text(cv, "WRONG WAY", ch / 4, big, (240, 80, 70));
        } else if r.santa.active && r.santa.target == Some(me) && self.flash.is_none() && (r.clock * 2.5) as i32 % 2 == 0 {
            big_text(cv, "SANTA'S AFTER YOU!", ch / 4, big, SANTA_COLOUR);
        } else if p.lap == LAPS && r.lap_time(me) < 2.0 && !p.lap_times.is_empty() {
            big_text(cv, "FINAL LAP", ch / 4, big, (255, 210, 80));
        }
        if let Some((text, _, col)) = &self.flash {
            if !r.counting_down() && r.time() >= 0.8 {
                // long messages sit high, just under the item slot, so
                // the scene stays in view
                let lines = text.split('|').count() as i32;
                let cy = if lines > 1 { item_box_px(cv.h) + 6 + lines * 7 * big / 2 } else { ch * 2 / 5 };
                big_text(cv, text, cy, big, *col);
            }
        }
        // item slot, top centre
        let box_px = item_box_px(cv.h);
        let (bx, by) = ((cv.w as i32 - box_px) / 2, 2);
        for y in by..by + box_px {
            for x in bx..bx + box_px {
                let edge = y == by || y == by + box_px - 1 || x == bx || x == bx + box_px - 1;
                cv.blend(x, y, if edge { (250, 210, 70) } else { (14, 18, 44) }, if edge { 1.0 } else { 0.8 });
            }
        }
        let shown = if p.roulette > 0.0 { Some(ITEMS[((r.clock * 14.0) as usize) % ITEMS.len()]) } else { p.item };
        if let Some(item) = shown {
            icon(cv, item, bx + box_px / 2, by + box_px / 2, ((box_px - 4) / 8).max(1));
        }
        // place, bottom right
        if !r.counting_down() {
            let text = ordinal(p.place);
            let col = match p.place {
                1 => (255, 210, 70),
                2 => (210, 220, 236),
                3 => (226, 150, 90),
                _ => (236, 240, 255),
            };
            let s = big + 1;
            big_text_at(cv, &text, cv.w as i32 - big_width(&text, s) - 3 * s, cv.h as i32 - 7 * s, s, col);
        }
    }

    fn draw_race_hud(&self, buf: &mut Buffer, area: Rect, input: &Input) {
        let r = &self.race;
        let me = self.me();
        let p = &r.racers[me];
        let k = &p.kart;
        put_text(buf, 1, 0, &format!(" LAP {}/{} ", p.lap.min(LAPS), LAPS), accent_style());
        put_text(buf, 1, 1, &format!(" {} ", fmt_time(p.finished.unwrap_or(r.time()))), panel_style());
        for (i, t) in p.lap_times.iter().enumerate() {
            let best = Some(*t) == r.best_lap(me);
            put_text(buf, 1, 2 + i as u16, &format!(" {} {} ", i + 1, fmt_time(*t)), if best { accent_style() } else { panel_style() });
        }
        // standings, down the left
        let y0 = 6;
        if area.height > y0 + 8 {
            for place in 1..=r.racers.len() as u8 {
                let Some(i) = r.in_place(place) else { continue };
                let rc = &r.racers[i];
                let c = ROSTER[rc.who].colour();
                let st = if i == me { accent_style() } else { Style::default().fg(Color::Rgb(c.0, c.1, c.2)).bg(Color::Rgb(14, 18, 44)) };
                put_text(buf, 1, y0 + place as u16 - 1, &format!(" {place} {:<8}", ROSTER[rc.who].name), st);
            }
        }
        if let (Some(item), true) = (p.item, p.roulette == 0.0) {
            let below = (item_box_px(area.height as usize * 2) / 2) as u16 + 1;
            let label = if p.ammo > 1 { format!(" {} x{} ", item.name(), p.ammo) } else { format!(" {} ", item.name()) };
            put_text(buf, area.width.saturating_sub(label.len() as u16) / 2, below + 1, &label, accent_style());
        }
        let kmh = (k.speed.abs() * 1.8) as i32;
        put_text(buf, 1, area.height.saturating_sub(1), &format!(" {kmh:3} km/h "), panel_style());
        if k.drift != 0 || k.boost > 0.0 {
            let filled = ((k.charge / crate::kart::TIERS[2]).min(1.0) * 9.0) as usize;
            let mut x = 13;
            put_text(buf, x, area.height.saturating_sub(1), if k.boost > 0.0 { " BOOST " } else { " DRIFT " }, accent_style());
            x += 7;
            for i in 0..9 {
                let on = i < filled && k.drift != 0;
                let col = match i / 3 {
                    0 => Color::Rgb(110, 200, 255),
                    1 => Color::Rgb(255, 200, 60),
                    _ => Color::Rgb(120, 240, 170),
                };
                put_text(buf, x + i as u16, area.height.saturating_sub(1), "█", panel_style().fg(if on { col } else { Color::Rgb(60, 66, 100) }));
            }
        }
        if r.santa.active {
            let d = (r.santa.kart.x - k.x).hypot(r.santa.kart.y - k.y) * 0.5;
            let who = r.santa.target.map_or("", |t| ROSTER[r.racers[t].who].name);
            let l = format!(" SANTA {d:.0} m away, after {who} ");
            put_text(buf, area.width.saturating_sub(l.len() as u16) / 2, area.height.saturating_sub(1), &l, Style::default().fg(Color::Rgb(255, 230, 230)).bg(Color::Rgb(150, 20, 30)).add_modifier(Modifier::BOLD));
        }
        let mw = (area.width / 6).clamp(10, 28);
        let mh = (mw / 2).max(5);
        if area.width > mw + 30 && area.height > mh + 8 {
            let mut marks: Vec<(f32, f32, Rgb, char)> = vec![(k.x, k.y, (255, 255, 255), '●')];
            if r.santa.active {
                marks.push((r.santa.kart.x, r.santa.kart.y, SANTA_COLOUR, 'S'));
            }
            for (i, rc) in r.racers.iter().enumerate() {
                if i != me {
                    marks.push((rc.kart.x, rc.kart.y, ROSTER[rc.who].colour(), '•'));
                }
            }
            render::minimap(buf, Rect::new(area.width - mw - 1, 0, mw, mh), &r.track, &marks);
            put_text(buf, area.width - mw - 1, mh, &format!("{:^w$}", r.track.name, w = mw as usize), panel_style());
        }
        if self.paused {
            draw_box(buf, area, &["PAUSED", "", "Enter/P  resume", "R        restart", "Esc      quit to title", "H        controls"]);
        }
        if let Some(total) = p.finished {
            if r.time() - total > 1.5 {
                let mut lines = vec![format!("FINISHED {}", ordinal(p.place)), String::new()];
                for place in 1..=r.racers.len() as u8 {
                    let Some(i) = r.in_place(place) else { continue };
                    let rc = &r.racers[i];
                    let time = rc.finished.map_or("racing...".to_string(), fmt_time);
                    let mark = if i == me { "  <" } else { "" };
                    lines.push(format!("{place}  {:<9}{time}{mark}", ROSTER[rc.who].name));
                }
                lines.push(String::new());
                for (i, t) in p.lap_times.iter().enumerate() {
                    let mark = if Some(*t) == r.best_lap(me) { "  best" } else { "" };
                    lines.push(format!("Lap {}   {}{mark}", i + 1, fmt_time(*t)));
                }
                lines.push(String::new());
                lines.push(match p.caught {
                    0 => "Santa never caught you.".to_string(),
                    1 => "Santa caught you once.".to_string(),
                    n => format!("Santa caught you {n} times."),
                });
                lines.push(santa_says(p.place, p.caught).to_string());
                lines.push(String::new());
                lines.push("Enter: race again    Esc: title".into());
                let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                draw_box(buf, area, &refs);
            }
        }
        if !input.exact && r.counting_down() {
            put_text(buf, 1, area.height.saturating_sub(2), " gas latches: Up on, Down off; Space toggles drift ", panel_style());
        }
    }

    pub fn snapshot(&self) {
        let r = &self.race;
        let me = self.me();
        let p = &r.racers[me];
        let k = &p.kart;
        let order: Vec<&str> = (1..=r.racers.len() as u8).filter_map(|pl| r.in_place(pl)).map(|i| ROSTER[r.racers[i].who].name).collect();
        log::debug!(
            "snapshot phase {:?} course {} paused {} clock {:.2} lap {} place {} idx {}/{} pos ({:.1},{:.1}) heading {:.2} speed {:.1} surface {} drift {} charge {:.2} boost {:.2} stun {:.2} item {:?}x{} santa {} order {:?} controls {:?}",
            self.phase,
            r.track.name,
            self.paused,
            r.clock,
            p.lap,
            p.place,
            p.idx,
            r.track.len(),
            k.x,
            k.y,
            k.heading,
            k.speed,
            render::surface_name(k.surface),
            k.drift,
            k.charge,
            k.boost,
            k.stun,
            p.item,
            p.ammo,
            if r.santa.active { format!("after {:?}", r.santa.target) } else { "away".into() },
            order,
            p.last
        );
    }

    /// The scene with its pixel overlays (no HUD text), for a screenshot.
    pub fn shot(&self, cols: u16, rows: u16) -> Canvas {
        let mut cv = if self.phase == Phase::Story {
            let mut cv = Canvas::new(cols, rows);
            crate::story::draw(&mut cv, self.page, self.t);
            cv
        } else {
            self.scene_canvas(cols, rows)
        };
        // SLEIGHRIDE_SCENE_ONLY=1: the world alone, no banners
        if std::env::var("SLEIGHRIDE_SCENE_ONLY").is_err() {
            self.overlay(&mut cv);
        }
        cv
    }

    /// Show a story page, for screenshots.
    pub fn show_story(&mut self, page: usize, t: f32) {
        self.phase = Phase::Story;
        self.page = page.min(STORY.len() - 1);
        self.t = t;
    }

    /// Advance on autopilot, for screenshots and tests.
    pub fn run_auto(&mut self, seconds: f32) {
        self.race.autopilot = true;
        for _ in 0..(seconds / DT) as usize {
            self.step_race(Controls::default());
        }
    }
}

/// Santa's verdict on the results card.
fn santa_says(place: u8, caught: u8) -> &'static str {
    match (place, caught) {
        (1, 0) => "Santa: \"...Fine. FINE. You're still polishing the runners.\"",
        (1, _) => "Santa: \"First place AND the naughty list. Impressive.\"",
        (_, 2..) => "Santa: \"Coal. All of you. Coal.\"",
        (6, _) => "Santa: \"Last? With MY reindeer? Back to the stables.\"",
        _ => "Santa: \"Back in the workshop by breakfast, the lot of you.\"",
    }
}

fn item_box_px(canvas_h: usize) -> i32 {
    ((canvas_h as f32) * 0.13).clamp(10.0, 40.0) as i32
}

/// Showcase models for the menus: every animation frame of each racer.
fn showcase() -> &'static Vec<[Model; FRAMES]> {
    static M: OnceLock<Vec<[Model; FRAMES]>> = OnceLock::new();
    M.get_or_init(|| ROSTER.iter().map(|w| Model::frames(w.ride, &w.elf)).collect())
}

/// Darken the race behind a menu.
fn dim(cv: &mut Canvas) {
    for p in cv.px.iter_mut() {
        *p = render::mix(*p, (10, 12, 34), 0.55);
    }
}

/// A racer turning slowly on a lit snow stage centred on (sx, sy), going
/// through its animation as if moving at a canter.
fn stage(cv: &mut Canvas, who: usize, t: f32, sx: f32, sy: f32) {
    let s = (cv.h as f32 / 75.0).max(0.7);
    let (rx, ry) = (s * 26.0, s * 6.0);
    for y in (sy - ry * 6.0) as i32..(sy + ry * 1.6) as i32 {
        for x in (sx - rx * 1.4) as i32..(sx + rx * 1.4) as i32 {
            let (dx, dy) = ((x as f32 - sx) / rx, (y as f32 - sy) / ry);
            let floor = dx * dx + dy * dy;
            if floor <= 1.0 {
                // the stage: packed snow, brighter toward the middle
                cv.put(x, y, render::mix((236, 242, 255), (150, 164, 206), floor));
            } else if dy < 0.0 && dx.abs() < 1.0 + dy * -0.04 {
                // a soft beam of light from above
                cv.blend(x, y, (255, 246, 214), 0.10 * (1.0 - dx.abs()));
            }
        }
    }
    let (frame, rise) = render::frame_of(ROSTER[who].ride, t * 30.0);
    let pose = Pose { sx, sy: sy - rise * s, px: s, yaw: t * 0.9, pitch: 0.28, roll: 0.0, nose: 0.0, fog: 0.0, haze: (0, 0, 0) };
    showcase()[who][frame].draw(cv, &pose);
}

/// 8 x 8 item icons.
fn icon(cv: &mut Canvas, item: Item, cx: i32, cy: i32, s: i32) {
    let art: [&str; 8] = match item {
        Item::Snowball => ["..WWWW..", ".WWWWWW.", "WWWWWWBW", "WWWWWWBW", "WWWWWBBW", "WWWWBBBW", ".WBBBBB.", "..BBBB.."],
        Item::Present => ["..Y..Y..", "...YY...", "RRRYYRRR", "RRRYYRRR", "YYYYYYYY", "RRRYYRRR", "RRRYYRRR", "RRRYYRRR"],
        Item::Tinsel => ["Y......Y", ".Y....R.", "..Y..R..", "...YR...", "...RY...", "..R..Y..", ".R....Y.", "R......Y"],
        Item::Peppermint => ["..WWWW..", ".WRRWRW.", "WRWWRWRW", "WRWRRWRW", "WRWWWWRW", "WWRRRRWW", ".WWWWWW.", "..WWWW.."],
        Item::Mistletoe => ["...RR...", "..R..R..", "...GG...", "GG.GG.GG", ".GGGGGG.", "..GWWG..", "...WW...", "...W...."],
    };
    let (x0, y0) = (cx - 4 * s, cy - 4 * s);
    for (ry, row) in art.iter().enumerate() {
        for (rx, ch) in row.bytes().enumerate() {
            let c: Rgb = match ch {
                b'W' => (250, 250, 255),
                b'B' => (170, 196, 236),
                b'R' => (222, 40, 56),
                b'Y' => (250, 206, 60),
                b'G' => (50, 160, 70),
                _ => continue,
            };
            for py in 0..s {
                for px in 0..s {
                    cv.put(x0 + rx as i32 * s + px, y0 + ry as i32 * s + py, c);
                }
            }
        }
    }
}

fn draw_box(buf: &mut Buffer, area: Rect, lines: &[&str]) {
    let w = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16 + 6;
    let h = lines.len() as u16 + 2;
    let x0 = area.width.saturating_sub(w) / 2;
    let y0 = area.height.saturating_sub(h) / 2;
    for y in 0..h {
        put_text(buf, x0, y0 + y, &" ".repeat(w as usize), panel_style());
    }
    for (i, l) in lines.iter().enumerate() {
        let st = if i == 0 { accent_style() } else { panel_style() };
        put_text(buf, x0 + 3, y0 + 1 + i as u16, l, st);
    }
}

fn draw_help(buf: &mut Buffer, area: Rect, input: &Input) {
    let mut lines = vec![
        "CONTROLS",
        "",
        "Left/Right or A/D   steer",
        "Up or W             gas",
        "Down or S           brake, reverse",
        "Space or Shift      hold to hop and drift",
        "X, E or Ctrl        use your item (brake + item: drop a present behind)",
        "",
        "Drift: hold drift while steering. Sparks go blue, gold, then aurora;",
        "let go for a boost. Steer into the turn to charge faster.",
        "Deep powder slows you and stops sparks. Ice: hardly any grip.",
        "",
        "Items from the present boxes: snowballs (three), a present to lob,",
        "tinsel that chases the racer ahead, a peppermint boost, and",
        "mistletoe: hold it out and whoever comes close gets kissed.",
        "",
        "From lap 2 Santa comes after the leader in his snowplough.",
        "",
        "Start: hit the gas as the 1 appears for a rocket start.",
        "",
        "Pad: stick or d-pad steer, A gas, B brake, R drift, X/Y/L item",
        "P pause   R restart   Esc back   H close this",
    ];
    if !input.exact {
        lines.insert(7, "(this terminal: Up latches gas on, Down off, Space toggles drift)");
    }
    draw_box(buf, area, &lines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn screen_text(g: &Game, input: &Input) -> String {
        let mut term = Terminal::new(TestBackend::new(140, 44)).unwrap();
        term.draw(|f| g.draw(f, input)).unwrap();
        let buf = term.backend().buffer().clone();
        buf.content().iter().map(|c| c.symbol().to_string()).collect::<Vec<_>>().chunks(140).map(|r| r.concat()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn a_race_ends_on_the_results_card() {
        let mut g = Game::new(Phase::Race, 0, 0, 1);
        let input = Input::new(true);
        assert!(screen_text(&g, &input).contains("LAP 1/3"));
        while !g.player_done() && g.race.clock < 500.0 {
            g.run_auto(DT);
        }
        assert!(g.player_done(), "player finished on autopilot");
        g.run_auto(2.0);
        let text = screen_text(&g, &input);
        assert!(text.contains("FINISHED"), "{text}");
        assert!(text.contains("Santa:"), "{text}");
        assert!(text.contains("Bob"));
        assert_eq!(text.matches("Lap ").count(), 3);
    }

    #[test]
    fn choosing_a_racer_and_racing_as_them() {
        let mut g = Game::new(Phase::Title, 0, 0, 1);
        let mut input = Input::new(true);
        input.cmds.push(Cmd::Confirm);
        g.tick(&mut input);
        assert_eq!(g.phase, Phase::Story, "the story comes first");
        g.t = 30.0;
        let text = screen_text(&g, &input);
        assert!(text.contains("1 / 4"), "{text}");
        assert!(text.contains("snoring"), "{text}");
        // Enter fills a page, then turns it; four pages
        let mut presses = 0;
        while g.phase == Phase::Story && presses < 12 {
            input.cmds.push(Cmd::Confirm);
            g.tick(&mut input);
            presses += 1;
        }
        assert_eq!(g.phase, Phase::Select, "after {presses} presses");
        input.cmds.extend([Cmd::Right, Cmd::Right]);
        g.tick(&mut input);
        assert_eq!(ROSTER[g.pick].name, "Sprocket");
        assert!(screen_text(&g, &input).contains("Sprocket"));
        input.cmds.push(Cmd::Confirm);
        g.tick(&mut input);
        assert_eq!(g.phase, Phase::Race);
        let p = g.race.player().unwrap();
        assert_eq!(ROSTER[g.race.racers[p].who].name, "Sprocket");
        assert_eq!(g.race.racers[p].place, 6, "the player starts at the back");
        assert!(g.race.racers[0].ai.is_some(), "Bob races as a rival");
    }

    #[test]
    fn the_title_screen_shows_the_course_and_a_racer() {
        let mut g = Game::new(Phase::Title, 1, 0, 1);
        let mut input = Input::new(true);
        input.cmds.push(Cmd::Down);
        g.tick(&mut input);
        let text = screen_text(&g, &input);
        assert!(text.contains("Rivals:  Easy"), "{text}");
        assert!(text.contains("Eggnogg"), "{text}");
        assert!(text.contains(" on "));
    }
}
