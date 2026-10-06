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
    /// How far you can see, and the colour things fade into (both shrink
    /// toward white in a blizzard).
    far: f32,
    haze: Rgb,
}

impl View {
    pub fn new(cv: &Canvas) -> View {
        let (w, h) = (cv.w as f32, cv.h as f32);
        let f = (w * 0.62).min(h * 1.7);
        let hz = (h * 0.34).round();
        // the sleigh's runners sit near the bottom of the screen, with room
        // below for the view to tip on a climb
        let ground = h * 0.86 - hz;
        View { w, hz, f, cam_h: ground * CAM_BACK / f, eye: 0.0, far: FAR, haze: (200, 210, 230) }
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
    if th.indoors {
        return ceiling(cv, v, cam, th, time);
    }
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

/// Indoors: dark rafters and beams overhead, warm lamps hanging between.
fn ceiling(cv: &mut Canvas, v: &View, cam: &Camera, th: &Theme, time: f32) {
    let hz = (v.hz.max(0.0) as usize).min(cv.h);
    for y in 0..hz {
        let t = y as f32 / v.hz.max(1.0);
        for x in 0..cv.w {
            let a = cam.angle + ((x as f32 - v.w / 2.0) / v.f).atan();
            // rafters run across every so many degrees; a beam band lower down
            let rafter = (a * 9.0).rem_euclid(1.0) < 0.12;
            let beam = (t - 0.62).abs() < 0.05;
            let mut c = mix(th.sky[0], th.sky[1], t * t);
            if rafter || beam {
                c = mix(c, (24, 14, 8), 0.7);
            }
            cv.px[y * cv.w + x] = c;
        }
    }
    // lamps hanging at fixed bearings, glowing and swaying a touch
    for k in 0..12 {
        let a = k as f32 / 12.0 * std::f32::consts::TAU + 0.26;
        let rel = wrap(a - cam.angle);
        if rel.abs() > 1.2 {
            continue;
        }
        let x = v.w / 2.0 + rel.tan() * v.f;
        let y = v.hz * (0.45 + 0.05 * (time * 0.8 + k as f32).sin());
        let r = (v.hz * 0.18).max(2.0);
        for dy in -(r as i32)..=(r as i32) {
            for dx in -(r as i32 * 2)..=(r as i32 * 2) {
                let d = ((dx as f32 / 2.0).powi(2) + (dy as f32).powi(2)).sqrt() / r;
                if d < 1.0 {
                    cv.blend(x as i32 + dx, y as i32 + dy, (255, 210, 130), 0.5 * (1.0 - d));
                }
            }
        }
        cv.put(x as i32, y as i32, (255, 244, 200));
        for q in 0..(y as i32) {
            cv.put(x as i32, q, (30, 20, 14));
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
    if !track.lake_spans.is_empty() && track.out_on_lake(i) {
        // out on the frozen lake
        for &(hx, hy, r) in &track.holes {
            let e = (x - hx).hypot(y - hy);
            if e < r {
                // open water, dark, with a slow ripple
                let ripple = ((e * 2.2 - hash(0, 0) as f32 * 0.0).sin() * 0.5 + 0.5) * 0.25;
                return mix((16, 34, 70), (40, 76, 120), ripple);
            }
            if e < r + 0.8 + (hash(x as i32, y as i32) % 3) as f32 * 0.3 {
                // the broken white rim
                return (236, 244, 252);
            }
        }
        if d <= half {
            // the ice road: clear blue, dusted with snow in the ruts
            let mut c = mix((126, 172, 210), (150, 194, 226), noise(x / 7.0, y / 7.0));
            for rut in [-6.5f32, -3.5, 3.5, 6.5] {
                if (side - rut).abs() < 0.7 {
                    c = mix(c, (232, 240, 250), 0.6);
                }
            }
            return shade(c, fine / 2);
        }
        return lake(th, x, y);
    }
    if !track.inlets.is_empty() && track.in_inlet(i) {
        // the inlet: an arm of the lake's cream cutting right across, out
        // over the powder on both sides so you see it coming past the ramp
        let swirl = noise(x / 4.0 + y / 9.0, y / 4.0 - x / 9.0);
        let mut c = mix(th.lake[0], th.lake[1], swirl);
        if hash(x as i32, y as i32) % 29 == 0 {
            c = mix(c, th.lake[2], 0.7);
        }
        return c;
    }
    if track.theme.indoors && d <= half {
        if !track.belts.is_empty() && track.on_belt(i) {
            // a rubber belt with chevrons running along it
            let run = ((i as f32 + BELT_T.with(|b| b.get()) * 30.0) / 3.0) as i32;
            let chev = ((run + (side.abs() * 0.8) as i32) % 3) == 0;
            return if chev { (230, 190, 40) } else { (48, 46, 52) };
        }
        if i < 3 || i > track.len() - 1 {
            let k = ((x * 0.5).floor() as i32 + (y * 0.5).floor() as i32) & 1;
            return if k == 0 { (24, 24, 32) } else { (240, 240, 245) };
        }
        // polished floorboards running along the lap
        let board = ((side + 20.0) / 2.2) as i32;
        let joint = ((i as i32 + board * 7) % 13) == 0;
        let seam = ((side + 20.0) / 2.2).fract() < 0.1;
        let shade_k = (hash(board, 3) % 5) as i32 * 4 - 8;
        let c = shade(th.road, shade_k);
        return if seam || joint { shade(c, -40) } else { shade(c, fine / 2) };
    }
    if d <= half {
        // the start line: a checker across the road
        if i < 3 || i > track.len() - 1 {
            let k = ((x * 0.5).floor() as i32 + (y * 0.5).floor() as i32) & 1;
            return if k == 0 { (24, 24, 32) } else { (240, 240, 245) };
        }
        if let Some(up) = track.on_kick(i) {
            // a wooden ramp, with warning chevrons at the lip
            let plank = ((up * 22.0) as i32 % 2 == 0) as i32 as f32;
            let mut c = mix((206, 162, 112), (184, 140, 94), plank);
            if up > 0.8 {
                let chevron = ((side.abs() * 0.6 + up * 30.0) as i32) % 2 == 0;
                c = if chevron { (240, 196, 50) } else { (40, 34, 40) };
            }
            return shade(c, fine);
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
    let haze = v.haze;
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
        while t < v.far * 1.2 && ybuf > 0.0 {
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
                    let fog = (lit.1 / v.far).min(1.0).powf(1.6);
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

/// A giant candy cane: a striped pole with its hook turned one way or the
/// other (`seed` picks which).
fn cane(cv: &mut Canvas, sx: f32, sy: f32, scale: f32, size: f32, seed: f32, fog: f32, haze: Rgb) {
    let h = TREE_H * size * scale * 1.05;
    if h < 1.0 {
        return;
    }
    let w = (h * 0.09).max(1.0);
    let hook = h * 0.17;
    let dir = if (seed * 0.37).sin() > 0.0 { 1.0 } else { -1.0 };
    let stripe = |u: f32, along: f32| -> Rgb {
        let k = ((along + u * 0.6) / (w * 1.1)).floor() as i32;
        let c = if k % 2 == 0 { (226, 34, 52) } else { (250, 248, 250) };
        mix(c, haze, fog)
    };
    // the straight shaft
    let top = sy - h + hook;
    for y in top as i32..sy as i32 {
        for x in (sx - w / 2.0) as i32..(sx + w / 2.0).ceil() as i32 {
            let u = x as f32 - sx;
            let shade = if u > 0.0 { 0.82 } else { 1.0 };
            let c = stripe(u, sy - y as f32);
            cv.put(x, y, (((c.0 as f32) * shade) as u8, ((c.1 as f32) * shade) as u8, ((c.2 as f32) * shade) as u8));
        }
    }
    // the hook: a half ring over the top
    let (cx, cy) = (sx + dir * hook, top);
    let steps = (hook * 8.0) as i32 + 8;
    for k in 0..=steps {
        let a = std::f32::consts::PI * k as f32 / steps as f32;
        let (px, py) = (cx - dir * a.cos() * hook, cy - a.sin() * hook);
        for t in 0..w.ceil() as i32 {
            let r = hook - w / 2.0 + t as f32;
            let (qx, qy) = (cx - dir * a.cos() * r, cy - a.sin() * r);
            cv.put(qx as i32, qy as i32, stripe(t as f32, h - hook + a * hook));
        }
        let _ = (px, py);
    }
}

/// A workshop toy standing by the track: a stack of wrapped presents or a
/// tall nutcracker soldier.
#[allow(clippy::too_many_arguments)]
fn toy(cv: &mut Canvas, sx: f32, sy: f32, scale: f32, size: f32, seed: f32, fog: f32, haze: Rgb) {
    let f = |c: Rgb| mix(c, haze, fog);
    let u = scale * size;
    if u < 0.2 {
        return;
    }
    let pick = (seed * 0.731).sin() > 0.0;
    if pick {
        // presents: three boxes stacked, ribbons across
        let papers = [(214, 38, 46), (52, 120, 220), (40, 160, 80), (240, 196, 50)];
        let mut base = sy;
        for k in 0..3 {
            let (w, h) = ((7.0 - k as f32 * 1.8) * u, (3.6 - k as f32 * 0.6) * u);
            let off = ((seed + k as f32) * 2.3).sin() * u;
            let paper = papers[((seed as i32).unsigned_abs() as usize + k) % 4];
            let ribbon = if paper == (240, 196, 50) { (214, 38, 46) } else { (250, 230, 120) };
            for y in (base - h) as i32..base as i32 {
                for x in (sx + off - w / 2.0) as i32..(sx + off + w / 2.0) as i32 {
                    let rib = (x as f32 - sx - off).abs() < u * 0.5 || (y as f32 - (base - h / 2.0)).abs() < u * 0.4;
                    cv.put(x, y, f(if rib { ribbon } else { paper }));
                }
            }
            base -= h;
        }
    } else {
        // a nutcracker: black boots, white trousers, red coat, a tall black
        // hat with a gold plume
        let w = 2.6 * u;
        let parts: [(f32, f32, Rgb); 6] = [(0.0, 1.4, (24, 20, 28)), (1.4, 4.2, (246, 244, 240)), (4.2, 8.0, (200, 30, 44)), (8.0, 10.0, (236, 190, 150)), (10.0, 13.5, (24, 20, 28)), (13.5, 14.5, (250, 206, 60))];
        for (a, b, c) in parts {
            for y in (sy - b * u) as i32..(sy - a * u) as i32 {
                let half = if c == (24, 20, 28) && a > 9.0 { w * 0.45 } else { w / 2.0 };
                for x in (sx - half) as i32..(sx + half) as i32 {
                    cv.put(x, y, f(c));
                }
            }
        }
        for y in (sy - 7.6 * u) as i32..(sy - 4.6 * u) as i32 {
            cv.put((sx - w / 2.0 - u * 0.5) as i32, y, f((250, 206, 60)));
            cv.put((sx + w / 2.0 + u * 0.3) as i32, y, f((250, 206, 60)));
        }
    }
}

/// A gingerbread house: brown walls with icing trim, a snowy pitched roof
/// with an icing zigzag, candy buttons and warm lit windows.
#[allow(clippy::too_many_arguments)]
fn house(cv: &mut Canvas, sx: f32, sy: f32, scale: f32, size: f32, seed: f32, fog: f32, haze: Rgb, time: f32) {
    let w = 9.0 * size * scale;
    let wall = 5.5 * size * scale;
    let roof = 4.5 * size * scale;
    if w < 2.0 {
        return;
    }
    let f = |c: Rgb| mix(c, haze, fog);
    let (x0, x1) = (sx - w / 2.0, sx + w / 2.0);
    let top = sy - wall;
    let buttons = [(226, 40, 60), (60, 170, 90), (250, 206, 60)];
    // the walls
    for y in top as i32..sy as i32 {
        for x in x0 as i32..x1 as i32 {
            let u = (x as f32 - x0) / w;
            let mut c = if u > 0.55 { (150, 92, 46) } else { (176, 110, 56) };
            // icing along the bottom and the corners
            if (sy - y as f32) < scale * 0.5 || u < 0.04 || u > 0.96 {
                c = (250, 250, 252);
            }
            cv.put(x, y, f(c));
        }
    }
    // two warm windows and a door
    let flicker = 0.85 + 0.15 * (time * 3.0 + seed).sin();
    let win = |cv: &mut Canvas, u0: f32, u1: f32, v0: f32, v1: f32, c: Rgb| {
        for y in (top + wall * v0) as i32..(top + wall * v1) as i32 {
            for x in (x0 + w * u0) as i32..(x0 + w * u1) as i32 {
                cv.put(x, y, f(c));
            }
        }
    };
    let lit = ((255.0 * flicker) as u8, (214.0 * flicker) as u8, 120);
    win(cv, 0.12, 0.3, 0.25, 0.55, lit);
    win(cv, 0.7, 0.88, 0.25, 0.55, lit);
    win(cv, 0.42, 0.58, 0.45, 1.0, (110, 60, 30));
    // candy buttons in a row
    for k in 0..4 {
        let bx = x0 + w * (0.15 + k as f32 * 0.23);
        let by = top + wall * 0.12;
        let r = (scale * 0.35).max(0.5);
        for dy in -(r as i32)..=r as i32 {
            for dx in -(r as i32)..=r as i32 {
                cv.put(bx as i32 + dx, by as i32 + dy, f(buttons[(k + seed as usize) % 3]));
            }
        }
    }
    // the roof: snow on top, an icing zigzag along the eaves
    for row in 0..roof as i32 {
        let k = row as f32 / roof;
        let half = w * 0.58 * k;
        let y = top - roof + row as f32;
        for x in (sx - half) as i32..(sx + half) as i32 {
            let zig = ((x as f32 - sx).abs() / (scale * 0.8)).fract() < 0.5;
            let c = if k > 0.85 && zig { (250, 250, 252) } else if k < 0.6 { (244, 248, 255) } else { (120, 70, 40) };
            cv.put(x, y as i32, f(c));
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

struct Critters {
    penguin: Model,
    bear: [Model; 4],
    bear_reared: Model,
    cat: [Model; 3],
    lad: Model,
    /// Spectators: four looks, arms down and up.
    fans: [[Model; 2]; 4],
}

fn critters() -> &'static Critters {
    static C: OnceLock<Critters> = OnceLock::new();
    C.get_or_init(|| Critters { penguin: Model::penguin(), bear: std::array::from_fn(|f| Model::bear(f, false)), bear_reared: Model::bear(0, true), cat: std::array::from_fn(Model::yule_cat), lad: Model::yule_lad(), fans: std::array::from_fn(|l| [Model::fan(l as u8, false), Model::fan(l as u8, true)]) })
}

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
        Ride::Sleigh | Ride::Toboggan | Ride::Broom => 16.0,
        Ride::Goat => 9.0,
    };
    let f = ((odo / cycle).fract() * FRAMES as f32) as usize % FRAMES;
    let rise = match ride {
        Ride::Reindeer => [0.0, 0.8, 1.6, 0.8][f],
        Ride::Skis => [0.0, 0.5, 0.0, 0.5][f],
        Ride::Goat => [0.0, 0.6, 1.2, 0.6][f],
        // the broom flies low, bobbing
        Ride::Broom => 9.0 + [0.0, 0.6, 1.0, 0.6][f],
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
            fog: (z / v.far).min(1.0).powf(1.6),
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
    if r.clipped > 0.0 {
        let s = (pose.px * 2.0).max(2.0);
        let since = crate::race::CLIP_TIME - r.clipped;
        if since < 0.5 {
            // Santa's mitten swinging in from the right, round the ear
            let k = since / 0.5;
            let x = head.0 + (1.0 - k) * 12.0 * s - 1.0 * s;
            let y = head.1 + 4.0 * s - (k * std::f32::consts::PI).sin() * 3.0 * s;
            mitten(cv, x, y, s as i32);
        }
        // stars going round the dazed head
        let st = (s / 2.0).max(1.0) as i32;
        for j in 0..4 {
            let a = time * 6.0 + j as f32 * std::f32::consts::FRAC_PI_2;
            let (x, y) = (head.0 + a.cos() * 5.0 * s, head.1 - 1.0 * s + a.sin() * 1.5 * s);
            let c = if j % 2 == 0 { (255, 230, 90) } else { (255, 255, 255) };
            for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                for q in 0..st {
                    for w in 0..st {
                        cv.put(x as i32 + dx * st + q, y as i32 + dy * st + w, c);
                    }
                }
            }
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

/// A bird on the wing, `size` pixels per unit; its wings beat with `flap`.
fn bird(cv: &mut Canvas, x: f32, y: f32, size: f32, flap: f32, c: Rgb) {
    let span = (size * 2.6).clamp(2.5, 16.0);
    // wings up, level, down, level
    let lift = match (flap.fract() * 4.0) as u32 {
        0 => -0.7,
        1 | 3 => 0.0,
        _ => 0.45,
    } * span;
    let steps = (span * 2.0) as i32 + 1;
    for k in 0..=steps {
        let f = k as f32 / steps as f32;
        // each wing bows a little
        let dy = lift * f + (f * (1.0 - f)) * span * 0.3;
        cv.put((x - span * f) as i32, (y + dy) as i32, c);
        cv.put((x + span * f) as i32, (y + dy) as i32, c);
    }
    cv.put(x as i32, y as i32 + 1, c);
}

/// A crow sitting on a branch: a dark body and a beak.
fn perched(cv: &mut Canvas, x: f32, y: f32, size: f32, c: Rgb) {
    let s = (size * 1.0).clamp(1.5, 6.0) as i32;
    for dy in -s..=0 {
        for dx in -s..=s {
            if dx * dx + dy * dy * 2 <= s * s + 1 {
                cv.put(x as i32 + dx, y as i32 + dy, c);
            }
        }
    }
    cv.put(x as i32 + s + 1, y as i32 - s / 2, (210, 170, 60));
}

/// Santa's big white mitten with its red cuff.
fn mitten(cv: &mut Canvas, x: f32, y: f32, s: i32) {
    const M: [&str; 6] = [".WWW.WW", "WWWWWWW", "WWWWWWW", "WWWWWW.", ".WWWW..", ".RRRR.."];
    for (ry, row) in M.iter().enumerate() {
        for (rx, ch) in row.bytes().enumerate() {
            let c = match ch {
                b'W' => (250, 250, 252),
                b'R' => (214, 38, 46),
                _ => continue,
            };
            for py in 0..s {
                for px in 0..s {
                    cv.put(x as i32 + rx as i32 * s + px, y as i32 + ry as i32 * s + py, c);
                }
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
    pub birds: &'a crate::birds::Birds,
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
    Crow(usize),
    Flyer(usize),
    Pole,
    Twister(usize),
    Penguin(usize),
    Bear,
    Cat,
    Lad(usize),
    Cottage(usize),
    Fan(usize),
}

thread_local! {
    /// Time for things on the ground that move, like conveyor belts.
    static BELT_T: std::cell::Cell<f32> = const { std::cell::Cell::new(0.0) };
}

pub fn draw_scene(cv: &mut Canvas, s: &Scene) {
    BELT_T.with(|b| b.set(s.time));
    let v = View::new(cv);
    let r = s.race;
    let th = &r.track.theme;
    // a blizzard closes in: shorter sight, everything fading to white
    let bliz = r.weather.blizzard;
    let haze = mix(th.haze, (238, 242, 250), bliz * 0.8);
    let mut v = v;
    v.eye = (s.cam.z + v.cam_h).max(r.track.height(s.cam.x, s.cam.y) + 1.5);
    // tipping the view up a slope lowers the horizon, and over a crest
    // raises it (a shear, as Comanche did, rather than a true rotation)
    v.hz += (v.f * s.cam.pitch.tan() * 0.7).clamp(-(cv.h as f32) * 0.12, cv.h as f32 * 0.12);
    v.far = FAR * (1.0 - 0.7 * bliz);
    v.haze = haze;
    let v = v;
    sky(cv, &v, &s.cam, th, s.time);
    if bliz > 0.0 {
        let rows = (v.hz.max(0.0) as usize).min(cv.h);
        for p in cv.px[..rows * cv.w].iter_mut() {
            *p = mix(*p, haze, bliz * 0.75);
        }
    }
    terrain(cv, &v, &s.cam, &r.track);
    // everything standing in the world, drawn far to near
    let mut things: Vec<(f32, f32, f32, Thing)> = Vec::new();
    let depth = |x: f32, y: f32| (x - s.cam.x) * s.cam.angle.cos() + (y - s.cam.y) * s.cam.angle.sin();
    for t in &r.track.trees {
        if (t.x - s.cam.x).abs() > v.far || (t.y - s.cam.y).abs() > v.far {
            continue;
        }
        let z = depth(t.x, t.y);
        if z > 1.0 && z < v.far {
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
    for (i, p) in r.wildlife.penguins.iter().enumerate() {
        if p.gait != crate::wildlife::Gait::Gone {
            things.push((depth(p.x, p.y), p.x, p.y, Thing::Penguin(i)));
        }
    }
    if let Some(b) = &r.wildlife.bear {
        things.push((depth(b.x, b.y), b.x, b.y, Thing::Bear));
    }
    if let Some(c) = &r.folklore.cat {
        things.push((depth(c.x, c.y), c.x, c.y, Thing::Cat));
    }
    for (i, l) in r.folklore.lads.iter().enumerate() {
        if !matches!(l.state, crate::folklore::Lad::Hiding(_)) {
            things.push((depth(l.x, l.y), l.x, l.y, Thing::Lad(i)));
        }
    }
    for (i, tw) in r.weather.twisters.iter().enumerate() {
        things.push((depth(tw.x, tw.y), tw.x, tw.y, Thing::Twister(i)));
    }
    for (i, &(cx, cy, _, _)) in r.track.cottages.iter().enumerate() {
        things.push((depth(cx, cy), cx, cy, Thing::Cottage(i)));
    }
    for (i, f) in r.crowd.fans.iter().enumerate() {
        things.push((depth(f.x, f.y), f.x, f.y, Thing::Fan(i)));
    }
    for &(px, py) in &r.track.poles {
        things.push((depth(px, py), px, py, Thing::Pole));
    }
    for (i, c) in s.birds.crows.iter().enumerate() {
        if !matches!(c.state, crate::birds::Perch::Away(_)) {
            things.push((depth(c.x, c.y), c.x, c.y, Thing::Crow(i)));
        }
    }
    for (i, f) in s.birds.flock.iter().enumerate() {
        things.push((depth(f.x, f.y), f.x, f.y, Thing::Flyer(i)));
    }
    // scenery right up against the camera only looks huge and odd; racers
    // stay, as kart racers draw them
    things.retain(|t| t.0 < v.far * 1.4 && (t.0 > 5.0 || matches!(t.3, Thing::Racer(_) | Thing::Santa) && t.0 > 1.0));
    things.sort_by(|a, b| b.0.total_cmp(&a.0));
    let tr = &r.track;
    for (z, x, y, thing) in things {
        let ground = tr.height(x, y);
        let Some((sx, sy, scale)) = v.project(&s.cam, x, y, ground) else { continue };
        let fog = (z / v.far).min(1.0).powf(1.6);
        // hidden wherever the ground in front is nearer
        cv.clip = z - 1.5;
        match thing {
            Thing::Tree(size) => {
                let w = TREE_H * size * scale * 0.6;
                if sx + w > 0.0 && sx - w < v.w {
                    if th.indoors {
                        toy(cv, sx, sy, scale, size, x + y * 3.0, fog, haze);
                    } else if th.canes {
                        cane(cv, sx, sy, scale, size, x, fog, haze);
                    } else if th.houses && hash(x as i32, y as i32) % 5 < 3 {
                        house(cv, sx, sy, scale, size, x + y, fog, haze, s.time);
                    } else {
                        pine(cv, sx, sy, scale, size, fog, haze);
                    }
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
            Thing::Twister(i) => {
                // a spinning funnel of snow, narrow at the ground, wide above
                let tw = &r.weather.twisters[i];
                let layers = 26;
                for k in 0..layers {
                    let up = k as f32 / layers as f32;
                    let hgt = up * 16.0;
                    let rad = 1.0 + up * up * 5.5;
                    let sway = (s.time * 1.3 + up * 4.0).sin() * up * 1.5;
                    let Some((ly, _, lsc)) = v.project(&s.cam, x, y, ground + hgt).map(|(_, yy, sc)| (yy, 0.0, sc)) else { continue };
                    let dots = 10 + (up * 14.0) as i32;
                    for j in 0..dots {
                        let a = tw.spin * (1.6 - up) + j as f32 / dots as f32 * std::f32::consts::TAU;
                        // the near half bright, the far half dimmer
                        let front = a.sin() > 0.0;
                        let px = sx + (a.cos() * rad + sway) * lsc;
                        let py = ly + a.sin() * rad * 0.25 * lsc;
                        let c = if front { (250, 252, 255) } else { (200, 212, 232) };
                        cv.blend(px as i32, py as i32, c, 0.85 * (1.0 - fog));
                        if lsc > 3.0 {
                            cv.blend(px as i32 + 1, py as i32, c, 0.6 * (1.0 - fog));
                        }
                    }
                }
            }
            Thing::Penguin(i) => {
                let p = &r.wildlife.penguins[i];
                if let Some((mut pose, sc)) = pose_at(&v, &s.cam, x, y, ground, p.heading, 1.0, haze) {
                    shadow(cv, pose.sx, pose.sy, sc, 1.0);
                    match p.gait {
                        crate::wildlife::Gait::Sliding(t) => {
                            // flat on its belly, spinning a little
                            pose.nose = -1.45;
                            pose.yaw += t * 3.0;
                        }
                        _ => pose.roll = (p.step * std::f32::consts::TAU).sin() * 0.25,
                    }
                    critters().penguin.draw(cv, &pose);
                }
            }
            Thing::Bear => {
                let b = r.wildlife.bear.as_ref().unwrap();
                let c = critters();
                let m = if b.rear > 0.0 { &c.bear_reared } else { &c.bear[(b.step * 4.0) as usize % 4] };
                if let Some((pose, sc)) = pose_at(&v, &s.cam, x, y, ground, b.heading, 1.0, haze) {
                    shadow(cv, pose.sx, pose.sy, sc, 4.0);
                    m.draw(cv, &pose);
                }
            }
            Thing::Cottage(i) => {
                let (_, _, size, _) = r.track.cottages[i];
                shadow(cv, sx, sy, scale, 9.0 * size);
                house(cv, sx, sy, scale, size, i as f32 * 7.0 + 3.0, fog, haze, s.time);
            }
            Thing::Cat => {
                let c = r.folklore.cat.unwrap();
                let (m, wiggle) = match c.state {
                    crate::folklore::Cat::Sitting(_) => (&critters().cat[0], 0.0),
                    crate::folklore::Cat::Crouching(t) => (&critters().cat[1], (t * 18.0).sin() * 0.08),
                    crate::folklore::Cat::Pouncing(_) => (&critters().cat[2], 0.0),
                };
                if let Some((sx, sy, sc)) = v.project(&s.cam, x, y, ground) {
                    shadow(cv, sx, sy, sc, 6.0);
                }
                if let Some((mut pose, _)) = pose_at(&v, &s.cam, x, y, c.z, c.heading, m.size, haze) {
                    pose.yaw += wiggle;
                    m.draw(cv, &pose);
                }
            }
            Thing::Lad(i) => {
                let l = &r.folklore.lads[i];
                if let Some((mut pose, _)) = pose_at(&v, &s.cam, x, y, ground, l.heading, 1.0, haze) {
                    match l.state {
                        // lurking: only his hood and eyes above the snow
                        crate::folklore::Lad::Lurking => pose.sy += pose.px * 8.0,
                        _ => pose.sy -= ((s.time * 14.0).sin().abs()) * pose.px * 2.0,
                    }
                    critters().lad.draw(cv, &pose);
                }
            }
            Thing::Fan(i) => {
                let f = &r.crowd.fans[i];
                // workshop elves are small, but not that small
                if let Some((mut pose, _)) = pose_at(&v, &s.cam, x, y, ground, f.heading, 2.4, haze) {
                    let up = f.cheer > 0.0;
                    if up {
                        // bouncing up and down with excitement
                        pose.sy -= ((s.time * 12.0 + i as f32).sin().abs()) * pose.px * 2.0;
                    }
                    critters().fans[f.look as usize % 4][up as usize].draw(cv, &pose);
                }
            }
            Thing::Pole => {
                // a striped pole with a pennant snapping in the wind
                let h = (6.0 * scale) as i32;
                let w = (scale * 0.25).max(1.0) as i32;
                for k in 0..h {
                    let c = if (k * 6 / h.max(1)) % 2 == 0 { (220, 40, 50) } else { (245, 245, 250) };
                    for q in 0..w {
                        cv.put(sx as i32 + q, sy as i32 - k, mix(c, haze, fog));
                    }
                }
                let flap = (s.time * 8.0 + x).sin() * 0.3;
                let len = (2.2 * scale) as i32;
                for k in 0..len {
                    let thick = ((len - k) as f32 / len as f32 * scale * 0.7) as i32;
                    for q in 0..thick.max(1) {
                        cv.put(sx as i32 + w + k, sy as i32 - h + q + (flap * k as f32) as i32, mix((250, 206, 60), haze, fog));
                    }
                }
            }
            Thing::Crow(i) => {
                let c = &s.birds.crows[i];
                if let Some((bx, by, sc)) = v.project(&s.cam, c.x, c.y, c.z) {
                    let col = mix((30, 26, 36), haze, fog);
                    match c.state {
                        crate::birds::Perch::Sitting => perched(cv, bx, by, sc, col),
                        _ => bird(cv, bx, by, sc * 1.1, c.flap, col),
                    }
                }
            }
            Thing::Flyer(i) => {
                let f = &s.birds.flock[i];
                if let Some((bx, by, sc)) = v.project(&s.cam, f.x, f.y, f.z) {
                    bird(cv, bx, by, sc * 1.3, f.flap, mix(th.birds, haze, fog * 0.6));
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
    if bliz > 0.0 {
        driving_snow(cv, &s.cam, r.weather.wind, bliz, s.time);
    }
}

/// Snow driving across the view in a blizzard: streaks blown along the
/// wind as it crosses the screen, falling as they go.
fn driving_snow(cv: &mut Canvas, cam: &Camera, wind: (f32, f32), k: f32, time: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    // the wind's sideways part as seen from the camera
    let across = wind.0 * -cam.angle.sin() + wind.1 * cam.angle.cos();
    let (dx, dy) = (across * 1.6 + 0.3, 0.9);
    let n = ((w * h) / 18.0 * k) as u32;
    let len = 3.0 + k * 3.0;
    for j in 0..n {
        let hj = hash(j as i32, 77) as u32;
        let speed = 0.6 + (hj % 100) as f32 / 120.0;
        let x0 = (hj % 4096) as f32 / 4096.0 * w + time * dx * 60.0 * speed;
        let y0 = ((hj >> 12) % 4096) as f32 / 4096.0 * h + time * dy * 60.0 * speed;
        let (x, y) = (x0.rem_euclid(w), y0.rem_euclid(h));
        for q in 0..len as i32 {
            let f = q as f32 / len;
            cv.blend((x - dx * f * len) as i32, (y - dy * f * len) as i32, (250, 252, 255), 0.7 * (1.0 - f));
        }
    }
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
/// Big text centred across the canvas around row `cy`; lines split on
/// '|' stack downward, and the scale shrinks until the widest line fits.
pub fn big_text(cv: &mut Canvas, text: &str, cy: i32, scale: i32, fill: Rgb) {
    let lines: Vec<&str> = text.split('|').collect();
    let mut scale = scale.max(1);
    while scale > 1 && lines.iter().any(|l| big_width(l, scale) + 4 > cv.w as i32) {
        scale -= 1;
    }
    let step = 7 * scale;
    let top = cy - (lines.len() as i32 * step) / 2;
    for (i, l) in lines.iter().enumerate() {
        let width = big_width(l, scale);
        big_text_at(cv, l, (cv.w as i32 - width) / 2, top + i as i32 * step, scale, fill);
    }
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
        Surface::Cream => "cream",
        Surface::BlackIce => "black ice",
        Surface::Conveyor => "conveyor",
    }
}
