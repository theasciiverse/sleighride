//! Drawing. The scene is painted into a pixel canvas two pixels per cell
//! (upper half block: foreground on top, background below) and flushed into
//! the ratatui buffer; the HUD goes on top as ordinary cells, and the
//! minimap in Braille.
//!
//! The ground is a height field drawn the way Comanche (NovaLogic, 1992)
//! drew its terrain: for each screen column, step out from the camera,
//! project each sample of ground by its height, and draw the part of the
//! column it shows that nothing nearer has covered. On flat ground that is
//! exactly the old Mode 7 floor; with hills you get crests, dips and things
//! hiding behind them. Each pixel's ground colour comes from the track's
//! lookup map (road, kerb, powder, lake...) rather than a stored texture,
//! and each pixel remembers its depth so billboards and racers are hidden
//! behind hills. Pines and snow are billboards, scaled by their distance.

use crate::kart::Kart;
use crate::model::{Model, Pose, Ride, FRAMES, VOX};
use crate::race::{Race, Racer, ShotKind};
use crate::roster::ROSTER;
use crate::track::Theme;
use std::sync::OnceLock;
use crate::track::{Surface, Track};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

pub type Rgb = (u8, u8, u8);

/// Draw distance, in world units.
const FAR: f32 = 360.0;
/// Camera distance behind the sleigh.
pub const CAM_BACK: f32 = 12.0;
const TREE_H: f32 = 11.0;

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Rgb>,
    /// Depth of the ground at each pixel (infinite for sky and haze).
    depth: Vec<f32>,
    /// Depth of whatever is being drawn now: pixels where the ground is
    /// nearer are left alone. Minus infinity draws over everything.
    pub clip: f32,
}

impl Canvas {
    pub fn new(cols: u16, rows: u16) -> Canvas {
        let (w, h) = (cols as usize, rows as usize * 2);
        Canvas { w, h, px: vec![(0, 0, 0); w * h], depth: vec![f32::INFINITY; w * h], clip: f32::NEG_INFINITY }
    }

    fn at(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let i = y as usize * self.w + x as usize;
            (self.clip < self.depth[i]).then_some(i)
        } else {
            None
        }
    }

    pub(crate) fn put(&mut self, x: i32, y: i32, c: Rgb) {
        if let Some(i) = self.at(x, y) {
            self.px[i] = c;
        }
    }

    pub(crate) fn blend(&mut self, x: i32, y: i32, c: Rgb, k: f32) {
        if let Some(i) = self.at(x, y) {
            self.px[i] = mix(self.px[i], c, k);
        }
    }

    pub fn flush(&self, buf: &mut Buffer, area: Rect) {
        for row in 0..(self.h / 2).min(area.height as usize) {
            for col in 0..self.w.min(area.width as usize) {
                let t = self.px[row * 2 * self.w + col];
                let b = self.px[(row * 2 + 1) * self.w + col];
                if let Some(cell) = buf.cell_mut((area.x + col as u16, area.y + row as u16)) {
                    cell.set_char('▀').set_fg(Color::Rgb(t.0, t.1, t.2)).set_bg(Color::Rgb(b.0, b.1, b.2));
                }
            }
        }
    }

    /// Binary PPM, for screenshots.
    pub fn ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.w, self.h).into_bytes();
        for p in &self.px {
            out.extend([p.0, p.1, p.2]);
        }
        out
    }
}

pub fn mix(a: Rgb, b: Rgb, k: f32) -> Rgb {
    let k = k.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * k) as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

fn shade(c: Rgb, d: i32) -> Rgb {
    let f = |x: u8| (x as i32 + d).clamp(0, 255) as u8;
    (f(c.0), f(c.1), f(c.2))
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2F) ^ (y as u32).wrapping_mul(0x1656_67B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^ (h >> 13)
}

/// Smooth value noise, roughly 0..1.
fn noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let v = |a: i32, b: i32| (hash(a, b) & 0xffff) as f32 / 65535.0;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let top = v(xi, yi) + (v(xi + 1, yi) - v(xi, yi)) * sx;
    let bot = v(xi, yi + 1) + (v(xi + 1, yi + 1) - v(xi, yi + 1)) * sx;
    top + (bot - top) * sy
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    /// The height the camera hangs above (the followed racer's, eased).
    pub z: f32,
    /// How far the view tips up (positive) or down to follow the slope.
    pub pitch: f32,
}

impl Camera {
    pub fn behind(k: &Kart) -> Camera {
        Camera { x: k.x - k.heading.cos() * CAM_BACK, y: k.y - k.heading.sin() * CAM_BACK, angle: k.heading, z: k.z, pitch: 0.0 }
    }

    /// Ease toward sitting behind the sleigh, looking between where the nose
    /// points and where it is going, tipped to the lie of the road ahead.
    pub fn follow(&mut self, k: &Kart, track: &Track, dt: f32) {
        let (c, s) = (self.angle.cos(), self.angle.sin());
        let ahead = track.height(k.x + c * 22.0, k.y + s * 22.0);
        let behind = track.height(k.x - c * 6.0, k.y - s * 6.0);
        let slope = ((ahead - behind) / 28.0).clamp(-0.5, 0.5);
        if !k.air {
            self.pitch += (slope.atan() - self.pitch) * (4.0 * dt).min(1.0);
        }
        let mut d = (k.heading + wrap(k.travel - k.heading) * 0.5) - self.angle;
        d = wrap(d);
        self.angle = wrap(self.angle + d * (5.0 * dt).min(1.0));
        self.x = k.x - self.angle.cos() * CAM_BACK;
        self.y = k.y - self.angle.sin() * CAM_BACK;
        // follow the ground closely, a jump more lazily so the sleigh rises
        // on screen
        let rate = if k.air { 2.5 } else { 9.0 };
        self.z += (k.z - self.z) * (rate * dt).min(1.0);
    }
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

/// Screen geometry for a canvas size.
#[derive(Clone, Copy)]
pub struct View {
    w: f32,
    /// Horizon row, in pixels.
    hz: f32,
    /// Focal length in pixels (pixels are square).
    f: f32,
    /// Camera height above the ground it follows.
    cam_h: f32,
    /// Eye height in the world, set per frame.
    eye: f32,
}

impl View {
    pub fn new(cv: &Canvas) -> View {
        let (w, h) = (cv.w as f32, cv.h as f32);
        let f = (w * 0.62).min(h * 1.7);
        let hz = (h * 0.34).round();
        // the sleigh's runners sit near the bottom of the screen, with room
        // below for the view to tip on a climb
        let ground = h * 0.86 - hz;
        View { w, hz, f, cam_h: ground * CAM_BACK / f, eye: 0.0 }
    }

    /// Screen position of a point at height `z`, and pixels per unit.
    fn project(&self, cam: &Camera, x: f32, y: f32, z: f32) -> Option<(f32, f32, f32)> {
        let (rx, ry) = (x - cam.x, y - cam.y);
        let (c, s) = (cam.angle.cos(), cam.angle.sin());
        let depth = rx * c + ry * s;
        if depth < 1.0 {
            return None;
        }
        let side = rx * -s + ry * c;
        let scale = self.f / depth;
        Some((self.w / 2.0 + side * scale, self.hz + (self.eye - z) * scale, scale))
    }
}

fn sky(cv: &mut Canvas, v: &View, cam: &Camera, th: &Theme, time: f32) {
    let hz = v.hz as usize;
    for y in 0..hz.min(cv.h) {
        let t = y as f32 / v.hz.max(1.0);
        let base = mix(th.sky[0], th.sky[1], t * t);
        for x in 0..cv.w {
            let a = cam.angle + ((x as f32 - v.w / 2.0) / v.f).atan();
            let mut c = base;
            if th.aurora {
                // aurora: a slow green ribbon high in the sky
                let wave = (a * 3.0 + time * 0.15).sin() * 0.06 + (a * 7.0 - time * 0.1).sin() * 0.03;
                let band = 1.0 - ((t - 0.32 - wave) / 0.09).abs();
                if band > 0.0 {
                    let shimmer = 0.6 + 0.4 * (a * 40.0 + time * 0.8).sin();
                    c = mix(c, (60, 220, 150), band * band * 0.55 * shimmer);
                }
            }
            // stars, pinned to direction so they turn with the view
            let sa = (a.rem_euclid(std::f32::consts::TAU) * 600.0) as i32;
            if t < 0.6 && hash(sa, y as i32) % 997 == 0 {
                c = mix(c, (255, 255, 240), 0.8 - t);
            }
            // two ranges of mountains, the far one paler
            let far = 0.5 + 0.3 * (a * 2.0 + 1.0).sin() + 0.2 * (a * 5.0 + 2.0).sin() + 0.1 * (a * 11.0).sin();
            let near = 0.4 + 0.3 * (a * 3.0 + 4.0).sin() + 0.2 * (a * 8.0 + 1.0).sin() + 0.1 * (a * 17.0 + 3.0).sin();
            let up = v.hz - y as f32;
            let far_h = far * v.hz * 0.6;
            let near_h = near * v.hz * 0.36;
            if up < near_h {
                let snow = up > v.hz * 0.22;
                c = if snow { mix(th.hills[1], (250, 250, 255), 0.75) } else { mix(mix(th.hills[1], th.haze, 0.2), th.hills[1], up / near_h.max(1.0)) };
            } else if up < far_h {
                let snow = up > v.hz * 0.36;
                c = if snow { mix(th.hills[0], (250, 250, 255), 0.6) } else { mix(th.hills[0], shade(th.hills[0], -26), up / far_h.max(1.0)) };
            }
            cv.px[y * cv.w + x] = c;
        }
    }
}

fn ground_colour(track: &Track, x: f32, y: f32) -> Rgb {
    let th = &track.theme;
    let Some((i, d, side)) = track.probe(x, y) else {
        return lake(th, x, y);
    };
    let half = track.half;
    let bank = track.bank_from();
    let fine = ((hash(x.floor() as i32, y.floor() as i32) & 0xff) as i32 - 128) / 24;
    if d <= half {
        // the start line: a checker across the road
        if i < 3 || i > track.len() - 1 {
            let k = ((x * 0.5).floor() as i32 + (y * 0.5).floor() as i32) & 1;
            return if k == 0 { (24, 24, 32) } else { (240, 240, 245) };
        }
        if track.on_ice(x, y) {
            // glassy, with long streaks that catch the light
            let streak = noise(x / 1.5 + y / 6.0, y / 1.5 - x / 6.0);
            return mix(th.ice, (250, 254, 255), (streak - 0.55).max(0.0) * 3.0);
        }
        let mut c: Rgb = th.road;
        // bands along the lap give the speed something to show against
        if (i / 8) % 2 == 0 {
            c = shade(c, -7);
        }
        // runner ruts, worn in by the field
        for rut in [-6.5f32, -3.5, 3.5, 6.5] {
            if (side - rut).abs() < 0.55 {
                c = shade(c, -22);
            }
        }
        return shade(c, fine);
    }
    if d <= half + 2.5 {
        return th.kerb[(i / 4) % 2];
    }
    if d <= bank {
        // deep powder: bright, with soft drifts and the odd glint
        let drift = noise(x / 9.0, y / 9.0);
        let mut c = mix(th.powder[0], th.powder[1], drift);
        if hash(x as i32 * 3, y as i32 * 7) % 211 == 0 {
            c = (255, 255, 255);
        }
        return shade(c, fine / 2);
    }
    if d <= bank + 6.0 {
        // the snowbank ridge
        let k = 1.0 - ((d - bank - 3.0) / 3.0).abs();
        return mix(th.bank[0], th.bank[1], k);
    }
    lake(th, x, y)
}

fn lake(th: &Theme, x: f32, y: f32) -> Rgb {
    let n = noise(x / 22.0, y / 22.0);
    let mut c = mix(th.lake[0], th.lake[1], n);
    if th.flecks {
        // nutmeg on the eggnog
        if hash(x as i32, y as i32) % 37 == 0 {
            c = mix(c, th.lake[2], 0.7);
        }
    } else {
        // cracks in the ice
        let crack = noise(x / 6.0 + 50.0, y / 6.0);
        if (crack - 0.5).abs() < 0.02 {
            c = th.lake[2];
        }
    }
    c
}

/// The ground, column by column, front to back.
fn terrain(cv: &mut Canvas, v: &View, cam: &Camera, track: &Track) {
    let (c, s) = (cam.angle.cos(), cam.angle.sin());
    let haze = track.theme.haze;
    let (w, h) = (cv.w, cv.h);
    for y in (v.hz.max(0.0) as usize)..h {
        for x in 0..w {
            cv.px[y * w + x] = haze;
        }
    }
    // the sun, low in the south-west: slopes facing it are lit
    let (lx, ly, lz) = (-0.45f32, 0.55, 0.7);
    for col in 0..w {
        let a = (col as f32 + 0.5 - v.w / 2.0) / v.f;
        let mut ybuf = h as f32;
        let mut t = 2.0f32;
        let mut prev: Option<(f32, f32, f32, f32)> = None;
        while t < FAR * 1.2 && ybuf > 0.0 {
            let (wx, wy) = (cam.x + c * t - s * a * t, cam.y + s * t + c * a * t);
            let ht = track.height(wx, wy);
            let sy = v.hz + (v.eye - ht) * v.f / t;
            // light by the slope here
            let (gx, gy) = (track.height(wx + 1.0, wy) - track.height(wx - 1.0, wy), track.height(wx, wy + 1.0) - track.height(wx, wy - 1.0));
            let lum = ((-gx * 0.5 * lx - gy * 0.5 * ly + lz) / ((gx * gx + gy * gy) * 0.25 + 1.0).sqrt() / lz).clamp(0.55, 1.3);
            if sy < ybuf {
                let (p0, p1) = (sy.ceil().max(0.0) as i32, (ybuf.ceil() as i32).min(h as i32));
                let (qx, qy, qt, ql) = prev.unwrap_or((wx, wy, t, lum));
                for py in p0..p1 {
                    // between this sample (far) and the last one (near)
                    let k = ((py as f32 + 0.5 - sy) / (ybuf - sy).max(1e-3)).clamp(0.0, 1.0);
                    let (px, pz, pt, pl) = (wx + (qx - wx) * k, wy + (qy - wy) * k, t + (qt - t) * k, lum + (ql - lum) * k);
                    let base = ground_colour(track, px, pz);
                    let lit = (scale_rgb(base, pl), pt);
                    let fog = (lit.1 / FAR).min(1.0).powf(1.6);
                    let i = py as usize * w + col;
                    cv.px[i] = mix(lit.0, haze, fog);
                    cv.depth[i] = pt;
                }
                ybuf = sy;
            }
            prev = Some((wx, wy, t, lum));
            t += 0.3 + t * 0.012;
        }
    }
}

fn scale_rgb(c: Rgb, k: f32) -> Rgb {
    let f = |x: u8| (x as f32 * k).min(255.0) as u8;
    (f(c.0), f(c.1), f(c.2))
}

fn pine(cv: &mut Canvas, sx: f32, sy: f32, scale: f32, size: f32, fog: f32, haze: Rgb) {
    let h = TREE_H * size * scale;
    let w = h * 0.55;
    if h < 1.0 {
        return;
    }
    let trunk = (h * 0.12).max(1.0);
    let top = sy - h;
    let (y0, y1) = (top.floor() as i32, sy.ceil() as i32);
    for y in y0..y1 {
        let fy = (y as f32 + 0.5 - top) / h;
        if fy > 1.0 - trunk / h {
            let tw = (w * 0.09).max(0.5);
            for x in (sx - tw).floor() as i32..(sx + tw).ceil() as i32 {
                cv.put(x, y, mix((86, 58, 40), haze, fog));
            }
            continue;
        }
        // three tiers, each widening downward
        let tier = (fy * 3.0).min(2.999);
        let within = tier.fract();
        let half = w * 0.5 * (0.35 + 0.65 * (tier.floor() + 1.0) / 3.0) * (0.3 + 0.7 * within);
        let x0 = (sx - half).floor() as i32;
        let x1 = (sx + half).ceil() as i32;
        for x in x0..x1 {
            let edge = (x as f32 + 0.5 - sx).abs() / half.max(0.5);
            let snowy = within < 0.35 || edge > 0.8 && within < 0.6;
            let c = if snowy { (232, 240, 252) } else if x as f32 > sx { (22, 70, 56) } else { (34, 96, 70) };
            cv.put(x, y, mix(c, haze, fog));
        }
    }
}

/// A grain of snow or a spark, in the world.
#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub life: f32,
    pub colour: Rgb,
}

fn particles(cv: &mut Canvas, v: &View, cam: &Camera, track: &Track, ps: &[Particle]) {
    for p in ps {
        if let Some((sx, sy, scale)) = v.project(cam, p.x, p.y, track.height(p.x, p.y) + p.z) {
            cv.clip = v.f / scale - 0.5;
            let y = sy;
            let r = (scale * 0.25).clamp(0.5, 2.0);
            let k = p.life.min(1.0);
            for dy in 0..r.ceil() as i32 {
                for dx in 0..r.ceil() as i32 {
                    cv.blend(sx as i32 + dx, y as i32 + dy, p.colour, k);
                }
            }
        }
    }
    cv.clip = f32::NEG_INFINITY;
}

/// Spark colour for a drift tier; the third tier cycles like the aurora.
pub fn tier_colour(tier: u8, time: f32) -> Rgb {
    match tier {
        1 => (110, 200, 255),
        2 => (255, 196, 56),
        _ => {
            let h = time * 7.0;
            ((150.0 + 100.0 * h.sin()) as u8, 230, (170.0 + 80.0 * (h * 1.3).cos()) as u8)
        }
    }
}

/// The voxel models, built once: every animation frame of each racer in
/// roster order, then Santa's plough.
fn models() -> &'static Vec<[Model; FRAMES]> {
    static MODELS: OnceLock<Vec<[Model; FRAMES]>> = OnceLock::new();
    MODELS.get_or_init(|| {
        let mut v: Vec<[Model; FRAMES]> = ROSTER.iter().map(|w| Model::frames(w.ride, &w.elf)).collect();
        v.push(Model::frames(Ride::Plough, &ROSTER[0].elf));
        v
    })
}
const PLOUGH_MODEL: usize = ROSTER.len();

fn gift() -> &'static Model {
    static GIFT: OnceLock<Model> = OnceLock::new();
    GIFT.get_or_init(|| Model::gift((206, 36, 52), (250, 214, 70)))
}

/// Animation frame for a ride that has travelled `odo` units, and how far
/// the whole model rises on that frame (a galloping reindeer's bound), in
/// voxels.
pub fn frame_of(ride: Ride, odo: f32) -> (usize, f32) {
    // units travelled per full cycle
    let cycle = match ride {
        Ride::Reindeer => 11.0,
        Ride::Skis => 10.0,
        Ride::Snowmobile | Ride::Plough => 3.0,
        Ride::Sleigh | Ride::Toboggan => 16.0,
    };
    let f = ((odo / cycle).fract() * FRAMES as f32) as usize % FRAMES;
    let rise = match ride {
        Ride::Reindeer => [0.0, 0.8, 1.6, 0.8][f],
        Ride::Skis => [0.0, 0.5, 0.0, 0.5][f],
        _ => 0.0,
    };
    (f, rise)
}

fn shadow(cv: &mut Canvas, sx: f32, sy: f32, scale: f32, w: f32) {
    let sw = w * scale * 0.5;
    let sh = (scale * 0.35).max(1.0);
    for dy in -(sh as i32)..=sh as i32 {
        for dx in (-sw) as i32..=sw as i32 {
            let e = (dx as f32 / sw).powi(2) + (dy as f32 / sh).powi(2);
            if e <= 1.0 {
                cv.blend(sx as i32 + dx, sy as i32 + dy, (40, 50, 80), 0.35);
            }
        }
    }
}

/// Pose for something standing at (x, y) at height `z`, heading `heading`.
fn pose_at(v: &View, cam: &Camera, x: f32, y: f32, z: f32, heading: f32, size: f32, haze: Rgb) -> Option<(Pose, f32)> {
    let (sx, sy, scale) = v.project(cam, x, y, z)?;
    let z = v.f / scale;
    let bearing = (y - cam.y).atan2(x - cam.x);
    Some((
        Pose {
            sx,
            sy,
            px: scale * VOX * size,
            yaw: wrap(heading - bearing),
            // flatter than the real camera, as kart sprites are drawn, so a
            // racer reads from behind rather than from above
            pitch: (v.cam_h / z).atan() * 0.55,
            roll: 0.0,
            nose: 0.0,
            fog: (z / FAR).min(1.0).powf(1.6),
            haze,
        },
        scale,
    ))
}

fn racer(cv: &mut Canvas, v: &View, cam: &Camera, track: &Track, r: &Racer, haze: Rgb, time: f32) {
    let k = &r.kart;
    let (frame, rise) = frame_of(ROSTER[r.who].ride, k.odo);
    let model = &models()[r.who][frame];
    let ground = track.height(k.x, k.y);
    if let Some((sx, sy, scale)) = v.project(cam, k.x, k.y, ground) {
        // the shadow stays on the ground, shrinking as you fly
        shadow(cv, sx, sy, scale, 3.4 / (1.0 + (k.z - ground).max(0.0) * 0.15));
    }
    let Some((mut pose, scale)) = pose_at(v, cam, k.x, k.y, k.z, k.heading, model.size, haze) else { return };
    let hop = if k.hop > 0.0 { (k.hop / 0.2 * std::f32::consts::PI).sin().max(0.0) } else { 0.0 };
    pose.sy -= hop * 0.6 * scale + rise * pose.px;
    // swung further into a drift the way kart racers show it, and right
    // round in a spin-out
    pose.yaw += r.lean * 0.45 + k.spin;
    pose.roll = r.lean * 0.16 + k.flip * std::f32::consts::TAU;
    // tilt with the slope, or with the flight
    let pitch = if k.air { (k.vz / k.speed.abs().max(10.0)).atan() * 0.7 } else { track.slope(k.x, k.y, k.heading).atan() };
    pose.nose = hop * 0.12 + pitch;
    // mistletoe on a stick, hung out over the front
    if r.mistletoe > 0.0 {
        let base = model.point([12.0, 0.0, 9.0], &pose);
        let tip = model.point([34.0, 0.0, 27.0], &pose);
        let n = ((tip.0 - base.0).abs().max((tip.1 - base.1).abs()) as i32).max(1);
        let thick = (pose.px * 0.8).max(1.0) as i32;
        for j in 0..=n {
            let t = j as f32 / n as f32;
            let (x, y) = (base.0 + (tip.0 - base.0) * t, base.1 + (tip.1 - base.1) * t);
            for d in 0..thick {
                cv.put(x as i32 + d, y as i32, (120, 80, 44));
            }
        }
        sprig(cv, tip.0, tip.1 + pose.px * 3.0, pose.px, time);
    }
    model.draw(cv, &pose);
    // drift sparks: a flickering burst at the back end of each runner
    let tier = k.tier();
    if tier > 0 {
        let col = tier_colour(tier, time);
        let frame = (time * 30.0) as i32;
        let n = 4 + tier as i32 * 4;
        let s = pose.px.max(1.0) as i32;
        for (ci, p) in model.sparks.iter().enumerate() {
            let (cx, cy) = model.point(*p, &pose);
            for j in 0..n {
                let h = hash(frame * 31 + j, ci as i32 + r.who as i32 * 7);
                let reach = (2 + tier as i32 * 2) * s;
                let dx = (h % (reach as u32 * 2 + 1)) as i32 - reach;
                let dy = -(((h >> 8) % (reach as u32 + 1)) as i32);
                let c = if (h >> 16) % 3 == 0 { (255, 255, 255) } else { col };
                cv.put(cx as i32 + dx, cy as i32 + dy, c);
                if s > 1 {
                    cv.put(cx as i32 + dx + 1, cy as i32 + dy, c);
                }
            }
        }
    }
    let head = model.point([12.0, 0.0, 27.0], &pose);
    if r.smooched > 0.0 {
        // little hearts drifting up
        for j in 0..3 {
            let rise = ((2.0 - r.smooched) * 1.5 + j as f32 * 0.33).fract();
            let x = head.0 + ((j as f32 * 2.1 + time * 3.0).sin() * 4.0 * pose.px.max(1.0));
            let y = head.1 - rise * 14.0 * pose.px.max(1.0);
            heart(cv, x, y, pose.px.max(1.0).min(3.0) as i32, 1.0 - rise);
        }
    }
    if r.tangled > 0.0 {
        let frame = (time * 24.0) as i32;
        let reach = (12.0 * pose.px).max(3.0) as i32;
        for j in 0..18 {
            let h = hash(frame * 13 + j, r.who as i32);
            let dx = (h % (reach as u32 * 2 + 1)) as i32 - reach;
            let dy = -(((h >> 8) % (reach as u32 * 2 + 1)) as i32);
            let c = if (h >> 20) % 2 == 0 { (250, 210, 70) } else { (230, 50, 70) };
            cv.put(head.0 as i32 + dx, (pose.sy as i32) + dy, c);
        }
    }
}

/// A sprig of mistletoe: leaves, white berries and a red bow.
fn sprig(cv: &mut Canvas, x: f32, y: f32, px: f32, time: f32) {
    let s = px.max(1.0);
    let sway = (time * 3.0).sin() * s;
    for (dx, dy, c) in [
        (-3.0, 0.0, (40, 140, 60)),
        (-2.0, 1.0, (30, 110, 50)),
        (3.0, 0.0, (40, 140, 60)),
        (2.0, 1.0, (30, 110, 50)),
        (0.0, 2.0, (40, 150, 64)),
        (-1.0, 3.0, (30, 110, 50)),
        (1.0, 3.0, (30, 110, 50)),
        (0.0, 4.0, (250, 250, 250)),
        (-1.0, 2.0, (250, 250, 250)),
        (1.0, 1.0, (250, 250, 250)),
        (0.0, -1.0, (220, 40, 50)),
        (-1.0, -2.0, (220, 40, 50)),
        (1.0, -2.0, (220, 40, 50)),
    ] {
        let (bx, by) = (x + sway + dx * s, y + dy * s);
        for py in 0..s as i32 {
            for qx in 0..s as i32 {
                cv.put(bx as i32 + qx, by as i32 + py, c);
            }
        }
    }
}

fn heart(cv: &mut Canvas, x: f32, y: f32, s: i32, k: f32) {
    const H: [&str; 4] = ["X.X", "XXX", "XXX", ".X."];
    for (ry, row) in H.iter().enumerate() {
        for (rx, ch) in row.bytes().enumerate() {
            if ch == b'X' {
                for py in 0..s {
                    for px in 0..s {
                        cv.blend(x as i32 + rx as i32 * s + px, y as i32 + ry as i32 * s + py, (255, 90, 140), k);
                    }
                }
            }
        }
    }
}

fn ball(cv: &mut Canvas, x: f32, y: f32, r: f32, c: Rgb, rim: Rgb) {
    let ri = r.ceil() as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= r {
                cv.put(x as i32 + dx, y as i32 + dy, if d > r - 1.0 && r > 1.5 { rim } else { c });
            }
        }
    }
}

pub struct Scene<'a> {
    pub race: &'a Race,
    pub cam: Camera,
    pub particles: &'a [Particle],
    pub time: f32,
}

enum Thing {
    Tree(f32),
    Racer(usize),
    Santa,
    Box,
    Shot(usize),
    Trap,
}

pub fn draw_scene(cv: &mut Canvas, s: &Scene) {
    let v = View::new(cv);
    let r = s.race;
    let th = &r.track.theme;
    let haze = th.haze;
    let mut v = v;
    v.eye = (s.cam.z + v.cam_h).max(r.track.height(s.cam.x, s.cam.y) + 1.5);
    // tipping the view up a slope lowers the horizon, and over a crest
    // raises it (a shear, as Comanche did, rather than a true rotation)
    v.hz += (v.f * s.cam.pitch.tan() * 0.7).clamp(-(cv.h as f32) * 0.12, cv.h as f32 * 0.12);
    let v = v;
    sky(cv, &v, &s.cam, th, s.time);
    terrain(cv, &v, &s.cam, &r.track);
    // everything standing in the world, drawn far to near
    let mut things: Vec<(f32, f32, f32, Thing)> = Vec::new();
    let depth = |x: f32, y: f32| (x - s.cam.x) * s.cam.angle.cos() + (y - s.cam.y) * s.cam.angle.sin();
    for t in &r.track.trees {
        if (t.x - s.cam.x).abs() > FAR || (t.y - s.cam.y).abs() > FAR {
            continue;
        }
        let z = depth(t.x, t.y);
        if z > 1.0 && z < FAR {
            things.push((z, t.x, t.y, Thing::Tree(t.size)));
        }
    }
    for (i, rc) in r.racers.iter().enumerate() {
        things.push((depth(rc.kart.x, rc.kart.y), rc.kart.x, rc.kart.y, Thing::Racer(i)));
    }
    if r.santa.active {
        let k = &r.santa.kart;
        things.push((depth(k.x, k.y), k.x, k.y, Thing::Santa));
    }
    for b in &r.boxes {
        if b.gone == 0.0 {
            things.push((depth(b.x, b.y), b.x, b.y, Thing::Box));
        }
    }
    for (i, sh) in r.shots.iter().enumerate() {
        things.push((depth(sh.x, sh.y), sh.x, sh.y, Thing::Shot(i)));
    }
    for t in &r.traps {
        things.push((depth(t.x, t.y), t.x, t.y, Thing::Trap));
    }
    things.retain(|t| t.0 > 1.0 && t.0 < FAR);
    things.sort_by(|a, b| b.0.total_cmp(&a.0));
    let tr = &r.track;
    for (z, x, y, thing) in things {
        let ground = tr.height(x, y);
        let Some((sx, sy, scale)) = v.project(&s.cam, x, y, ground) else { continue };
        let fog = (z / FAR).min(1.0).powf(1.6);
        // hidden wherever the ground in front is nearer
        cv.clip = z - 1.5;
        match thing {
            Thing::Tree(size) => {
                let w = TREE_H * size * scale * 0.6;
                if sx + w > 0.0 && sx - w < v.w {
                    pine(cv, sx, sy, scale, size, fog, haze);
                }
            }
            Thing::Racer(i) => racer(cv, &v, &s.cam, tr, &r.racers[i], haze, s.time),
            Thing::Santa => {
                let k = &r.santa.kart;
                let m = &models()[PLOUGH_MODEL][frame_of(Ride::Plough, k.odo).0];
                if let Some((pose, scale)) = pose_at(&v, &s.cam, k.x, k.y, k.z, k.heading, m.size, haze) {
                    shadow(cv, pose.sx, pose.sy, scale, 6.0);
                    m.draw(cv, &pose);
                    // the beacon flashes
                    if (s.time * 4.0) as i32 % 2 == 0 {
                        let (bx, by) = m.point([4.0, 0.0, 16.0], &pose);
                        ball(cv, bx, by, (pose.px * 2.2).max(1.5), (255, 200, 80), (255, 140, 30));
                    }
                }
            }
            Thing::Box => {
                let m = gift();
                let spin = s.time * 2.0 + x * 0.1;
                if let Some((mut pose, scale)) = pose_at(&v, &s.cam, x, y, ground, spin, 1.0, haze) {
                    shadow(cv, pose.sx, pose.sy, scale, 1.6);
                    pose.sy -= (1.2 + (s.time * 3.0 + x).sin() * 0.3) * scale;
                    m.draw(cv, &pose);
                }
            }
            Thing::Shot(i) => {
                let sh = &r.shots[i];
                let lift = sh.z * scale;
                match sh.kind {
                    ShotKind::Snowball => ball(cv, sx, sy - lift, (scale * 0.7).max(1.0), (252, 252, 255), (190, 200, 230)),
                    ShotKind::Tinsel => {
                        let frame = (s.time * 30.0) as i32;
                        let reach = (scale * 1.2).max(2.0) as i32;
                        for j in 0..14 {
                            let h = hash(frame * 7 + j, i as i32);
                            let dx = (h % (reach as u32 * 2 + 1)) as i32 - reach;
                            let dy = ((h >> 8) % (reach as u32 * 2 + 1)) as i32 - reach;
                            let c = if (h >> 16) % 2 == 0 { (250, 214, 70) } else { (230, 60, 80) };
                            cv.put(sx as i32 + dx, (sy - lift) as i32 + dy, c);
                        }
                    }
                    ShotKind::Present => {
                        let m = gift();
                        if let Some((mut pose, _)) = pose_at(&v, &s.cam, x, y, ground, s.time * 9.0, 0.7, haze) {
                            shadow(cv, pose.sx, pose.sy, scale, 1.0);
                            pose.sy -= lift;
                            pose.nose = s.time * 7.0;
                            m.draw(cv, &pose);
                        }
                    }
                }
            }
            Thing::Trap => {
                let m = gift();
                let wobble = (s.time * 6.0 + x).sin() * 0.15;
                if let Some((mut pose, _)) = pose_at(&v, &s.cam, x, y, ground, 0.6, 0.8, haze) {
                    pose.roll = wobble;
                    m.draw(cv, &pose);
                }
            }
        }
    }
    cv.clip = f32::NEG_INFINITY;
    particles(cv, &v, &s.cam, tr, s.particles);
}

/// 3 x 5 pixel font for big text.
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 3, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'K' => [5, 5, 6, 5, 5],
        'J' => [1, 1, 1, 5, 2],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'Q' => [2, 5, 5, 6, 3],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 5, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '?' => [7, 1, 2, 0, 2],
        '\'' => [2, 2, 0, 0, 0],
        '!' => [2, 2, 2, 0, 2],
        '-' => [0, 0, 7, 0, 0],
        ':' => [0, 2, 0, 2, 0],
        '.' => [0, 0, 0, 0, 2],
        _ => [0; 5],
    }
}

/// Big outlined pixel text, centred across the canvas around row `cy`.
pub fn big_text(cv: &mut Canvas, text: &str, cy: i32, scale: i32, fill: Rgb) {
    let width = big_width(text, scale);
    big_text_at(cv, text, (cv.w as i32 - width) / 2, cy - 5 * scale / 2, scale, fill);
}

pub fn big_width(text: &str, scale: i32) -> i32 {
    text.chars().count() as i32 * 4 * scale - scale
}

/// Big outlined pixel text with its top left at (x0, y0).
pub fn big_text_at(cv: &mut Canvas, text: &str, x0: i32, y0: i32, scale: i32, fill: Rgb) {
    let n = text.chars().count() as i32;
    let on = |gx: i32, gy: i32| -> bool {
        if gx < 0 || gy < 0 || gy >= 5 {
            return false;
        }
        let (ci, cx) = (gx / 4, gx % 4);
        if cx == 3 || ci >= n {
            return false;
        }
        let ch = text.chars().nth(ci as usize).unwrap();
        glyph(ch)[gy as usize] >> (2 - cx) & 1 == 1
    };
    for gy in -1..6 {
        for gx in -1..n * 4 {
            let lit = on(gx, gy);
            let near = !lit && (-1..=1).any(|dy| (-1..=1).any(|dx| on(gx + dx, gy + dy)));
            if !(lit || near) {
                continue;
            }
            let c = if lit { mix(fill, (255, 255, 255), 0.15 * (4 - gy).max(0) as f32 / 4.0) } else { (16, 20, 44) };
            for py in 0..scale {
                for px in 0..scale {
                    cv.put(x0 + gx * scale + px, y0 + gy * scale + py, c);
                }
            }
        }
    }
}

/// Minimap in Braille: the course in pale blue and a marker per racer.
pub fn minimap(buf: &mut Buffer, area: Rect, track: &Track, marks: &[(f32, f32, Rgb, char)]) {
    let (cols, rows) = (area.width as usize, area.height as usize);
    if cols < 4 || rows < 2 {
        return;
    }
    let (dw, dh) = (cols * 2, rows * 4);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in &track.pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    // a cell is 2x4 dots of roughly equal pitch; keep the course's aspect
    let k = ((dw - 1) as f32 / (x1 - x0)).min((dh - 1) as f32 / (y1 - y0));
    let ox = (dw as f32 - (x1 - x0) * k) / 2.0;
    let oy = (dh as f32 - (y1 - y0) * k) / 2.0;
    let to_dot = |x: f32, y: f32| ((((x - x0) * k + ox).max(0.0)) as usize, (((y - y0) * k + oy).max(0.0)) as usize);
    let mut bits = vec![0u8; cols * rows];
    for &(x, y) in track.pts.iter().step_by(3) {
        let (dx, dy) = to_dot(x, y);
        if dx < dw && dy < dh {
            const MAP: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
            bits[(dy / 4) * cols + dx / 2] |= MAP[dy % 4][dx % 2];
        }
    }
    for (i, b) in bits.iter().enumerate() {
        let (col, row) = ((i % cols) as u16, (i / cols) as u16);
        let ch = char::from_u32(0x2800 + *b as u32).unwrap_or(' ');
        if let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) {
            cell.set_char(ch).set_fg(Color::Rgb(150, 190, 240)).set_bg(Color::Rgb(14, 18, 44));
        }
    }
    // markers last, the first in the list on top
    for &(x, y, c, ch) in marks.iter().rev() {
        let (dx, dy) = to_dot(x, y);
        let (col, row) = ((dx / 2).min(cols - 1) as u16, (dy / 4).min(rows - 1) as u16);
        if let Some(cell) = buf.cell_mut((area.x + col, area.y + row)) {
            cell.set_char(ch).set_fg(Color::Rgb(c.0, c.1, c.2)).set_bg(Color::Rgb(14, 18, 44));
        }
    }
}

pub fn panel_style() -> Style {
    Style::default().fg(Color::Rgb(236, 240, 255)).bg(Color::Rgb(14, 18, 44))
}

pub fn accent_style() -> Style {
    Style::default().fg(Color::Rgb(255, 206, 84)).bg(Color::Rgb(14, 18, 44)).add_modifier(Modifier::BOLD)
}

pub fn put_text(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style) {
    let area = buf.area;
    if y >= area.y + area.height || x >= area.x + area.width {
        return;
    }
    let max = (area.x + area.width - x) as usize;
    let t: String = text.chars().take(max).collect();
    buf.set_string(x, y, t, style);
}

/// Classify for logging and the HUD.
pub fn surface_name(s: Surface) -> &'static str {
    match s {
        Surface::Road => "road",
        Surface::Kerb => "kerb",
        Surface::Powder => "powder",
        Surface::Bank => "bank",
        Surface::Ice => "ice",
    }
}
