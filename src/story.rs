//! The intro: one action-comic page whose three panels slam in one after
//! another (Santa out cold, the elves bursting out with everything that has
//! runners, Santa charging in his plough), each with a sound word in a
//! starburst. The scenes are drawn into the pixel canvas with a little life
//! in each (the fire, the steam, the snow, speed lines, the spray).

use crate::model::{Model, Pose, Ride};
use crate::render::{big_text_at, mix, Canvas, Rgb};
use crate::roster::ROSTER;
use std::sync::OnceLock;

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^ (h >> 13)
}

struct Cast {
    santa_chair: Model,
    sleigh: Model,
    reindeer: Model,
    snowmobile: Model,
    toboggan: Model,
    plough: Model,
}

fn cast() -> &'static Cast {
    static C: OnceLock<Cast> = OnceLock::new();
    C.get_or_init(|| Cast {
        santa_chair: Model::santa_armchair(),
        sleigh: Model::build(Ride::Sleigh, &ROSTER[0].elf, 0),
        reindeer: Model::build(Ride::Reindeer, &ROSTER[1].elf, 0),
        snowmobile: Model::build(Ride::Snowmobile, &ROSTER[2].elf, 0),
        toboggan: Model::build(Ride::Toboggan, &ROSTER[3].elf, 0),
        plough: Model::build(Ride::Plough, &ROSTER[0].elf, 0),
    })
}

fn rect(cv: &mut Canvas, x0: i32, y0: i32, x1: i32, y1: i32, c: Rgb) {
    for y in y0..y1 {
        for x in x0..x1 {
            cv.put(x, y, c);
        }
    }
}

/// Glow: blend toward `c` by `k` falling off over `r` pixels.
fn glow(cv: &mut Canvas, cx: f32, cy: f32, r: f32, c: Rgb, k: f32) {
    let ri = r as i32;
    for y in (cy as i32 - ri)..(cy as i32 + ri) {
        for x in (cx as i32 - ri)..(cx as i32 + ri) {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / r;
            if d < 1.0 {
                cv.blend(x, y, c, k * (1.0 - d) * (1.0 - d));
            }
        }
    }
}

fn pose(sx: f32, sy: f32, px: f32, yaw: f32) -> Pose {
    Pose { sx, sy, px, yaw, pitch: 0.22, roll: 0.0, nose: 0.0, fog: 0.0, haze: (0, 0, 0) }
}

/// Wooden walls and floor, with the floor starting at `floor`.
fn room(cv: &mut Canvas, floor: i32, warm: f32) {
    let (w, h) = (cv.w as i32, cv.h as i32);
    let plank = (w / 14).max(3);
    for y in 0..h {
        for x in 0..w {
            let c = if y < floor {
                let base = mix((92, 60, 38), (64, 40, 26), y as f32 / floor as f32);
                if x % plank == 0 {
                    mix(base, (40, 24, 16), 0.6)
                } else {
                    let grain = (hash(x / 2, y / 6) % 9) as f32 / 9.0;
                    mix(base, (104, 70, 44), grain * 0.25)
                }
            } else {
                // floorboards running away from us
                let row = ((y - floor) as f32).sqrt() as i32;
                let base = mix((70, 46, 30), (110, 74, 46), (y - floor) as f32 / (h - floor) as f32);
                if row % 2 == 0 { base } else { mix(base, (50, 32, 20), 0.3) }
            };
            cv.put(x, y, mix(c, (8, 8, 24), 0.35 * (1.0 - warm)));
        }
    }
}

fn window(cv: &mut Canvas, x0: i32, y0: i32, ww: i32, wh: i32, t: f32, sky: [Rgb; 2]) {
    rect(cv, x0 - 2, y0 - 2, x0 + ww + 2, y0 + wh + 2, (60, 38, 24));
    for y in y0..y0 + wh {
        for x in x0..x0 + ww {
            cv.put(x, y, mix(sky[0], sky[1], (y - y0) as f32 / wh as f32));
        }
    }
    // snow falling past the glass
    for k in 0..(ww * wh / 30).max(6) {
        let h = hash(k, 7);
        let fx = (h % ww as u32) as f32 + (t * 0.7 + k as f32).sin() * 1.5;
        let fy = ((h >> 8) % (wh as u32 * 4)) as f32 * 0.25 + t * (4.0 + (h >> 20) as f32 % 4.0);
        cv.put(x0 + fx as i32 % ww, y0 + (fy as i32 % wh), (240, 244, 255));
    }
    rect(cv, x0 + ww / 2 - 1, y0, x0 + ww / 2 + 1, y0 + wh, (60, 38, 24));
    rect(cv, x0, y0 + wh / 2 - 1, x0 + ww, y0 + wh / 2, (60, 38, 24));
}

// ------------------------------------------------------------- the comic

/// Seconds between panels slamming in, and how long a slam takes.
pub const BEAT: f32 = 0.9;
const SLAM: f32 = 0.2;
pub const PANELS: usize = 3;

/// When the whole page is showing.
pub fn comic_done(t: f32) -> bool {
    t >= BEAT * (PANELS - 1) as f32 + SLAM + 0.6
}

/// The two gutters, as x (0..1 of the width) at a height `v` (0..1 down
/// the page): sweeping diagonals from top right to bottom left, each with an
/// S-kink in the middle, so the middle panel is a slanted, winding band.
fn gutter(k: usize, v: f32) -> f32 {
    let (top, bottom) = if k == 0 { (0.48, 0.17) } else { (0.78, 0.53) };
    let line = top + (bottom - top) * v;
    let kink = -0.06 * (std::f32::consts::TAU * (v - 0.3)).sin() * (1.0 - (2.0 * v - 1.0).abs());
    line + kink
}

/// Which panel a pixel is in, or None in a gutter.
fn panel_at(x: f32, y: f32, w: f32, h: f32) -> Option<usize> {
    let v = y / h;
    let (g0, g1) = (gutter(0, v) * w, gutter(1, v) * w);
    let gap = 2.5;
    if x < g0 - gap {
        Some(0)
    } else if x > g0 + gap && x < g1 - gap {
        Some(1)
    } else if x > g1 + gap {
        Some(2)
    } else {
        None
    }
}

/// Where each caption goes (top-left, in pixels): under the sleeping Santa,
/// at the top of the road, above the plough.
pub fn caption_spots(w: usize, h: usize) -> [(i32, i32, i32); PANELS] {
    let (w, h) = (w as f32, h as f32);
    // (x, y, width in cells)
    [
        (2, (h * 0.78) as i32 & !1, (gutter(0, 0.85) * w - 6.0) as i32),
        ((gutter(0, 0.04) * w + 4.0) as i32, 2, ((gutter(1, 0.04) - gutter(0, 0.04)) * w - 8.0) as i32),
        ((gutter(1, 0.06) * w + 4.0) as i32, 2, ((1.0 - gutter(1, 0.06)) * w - 6.0) as i32),
    ]
}

/// The middle panel: the road winding up the page, pines either side, and
/// everyone with runners charging up it.
fn road_up(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    for y in 0..cv.h as i32 {
        for x in 0..cv.w as i32 {
            cv.put(x, y, mix((236, 242, 255), (210, 222, 244), y as f32 / h));
        }
    }
    // the road follows the middle of the band, narrowing up the page
    let mid = |v: f32| (gutter(0, v) + gutter(1, v)) / 2.0 * w;
    for y in 0..cv.h as i32 {
        let v = y as f32 / h;
        let half = (0.05 + 0.08 * v) * w;
        let cx = mid(v);
        for x in (cx - half - 2.0) as i32..(cx + half + 2.0) as i32 {
            let edge = (x as f32 - cx).abs() > half;
            let kerb = ((y / 3) % 2 == 0) as i32;
            let c = if edge { if kerb == 1 { (210, 40, 52) } else { (250, 250, 252) } } else { mix((170, 176, 200), (150, 156, 184), v) };
            cv.put(x, y, c);
        }
    }
    // pines up both sides
    for k in 0..10 {
        let v = 0.08 + k as f32 * 0.095;
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let x = mid(v) + side * (0.13 + 0.1 * v) * w;
        let sz = 4.0 + v * 10.0;
        for row in 0..(sz * 1.8) as i32 {
            let f = row as f32 / (sz * 1.8);
            let half = f * sz * 0.6;
            for dx in -half as i32..=half as i32 {
                cv.put(x as i32 + dx, (v * h) as i32 - (sz * 1.8) as i32 + row, if f < 0.3 { (232, 240, 252) } else { (30, 90, 64) });
            }
        }
    }
    let s = (h / 70.0).max(0.6);
    let c = cast();
    // charging up the road, smaller as they go
    // spread up the road, clear of the sound words at top and bottom
    let racers: [(&Model, f32, f32); 4] = [(&c.reindeer, 0.3, -0.03), (&c.snowmobile, 0.43, 0.03), (&c.toboggan, 0.55, -0.04), (&c.sleigh, 0.68, 0.02)];
    for (m, v, off) in racers {
        let x = mid(v) + off * w;
        let y = v * h;
        // speed streaks behind
        for k in 0..6 {
            let sx = x + (k as f32 - 2.5) * s * 3.0;
            for q in 0..(10.0 * s * v + 4.0) as i32 {
                cv.blend(sx as i32, y as i32 + q, (255, 255, 255), 0.7);
            }
        }
        let bob = ((t * 12.0 + v * 9.0).sin()) * s;
        m.draw(cv, &pose(x, y + bob, s * (0.35 + 0.7 * v), 0.15 - (mid(v + 0.05) - mid(v)) / w * 3.0));
    }
}

/// A comic starburst behind a sound word.
fn burst(cv: &mut Canvas, cx: f32, cy: f32, r: f32, word: &str, scale: i32, spin: f32) {
    let points = 12;
    let inside = |x: f32, y: f32| -> Option<bool> {
        let (dx, dy) = (x - cx, (y - cy) * 1.3);
        let a = dy.atan2(dx) + spin;
        let k = (a / std::f32::consts::TAU * points as f32 * 2.0).rem_euclid(points as f32 * 2.0);
        let f = k.fract();
        let edge = if (k as i32) % 2 == 0 { r * (0.55 + 0.45 * f) } else { r * (1.0 - 0.45 * f) };
        let d = dx.hypot(dy);
        (d < edge).then_some(d < edge - 2.0)
    };
    for y in (cy - r) as i32..(cy + r) as i32 {
        for x in (cx - r * 1.2) as i32..(cx + r * 1.2) as i32 {
            match inside(x as f32, y as f32) {
                Some(true) => cv.put(x, y, (255, 214, 40)),
                Some(false) => cv.put(x, y, (20, 16, 24)),
                None => {}
            }
        }
    }
    let width = crate::render::big_width(word, scale);
    big_text_at(cv, word, cx as i32 - width / 2, cy as i32 - 5 * scale / 2, scale, (226, 40, 46));
}

/// Santa asleep in his chair, for the left panel.
fn asleep(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    let floor = (h * 0.7) as i32;
    room(cv, floor, 0.45);
    window(cv, (w * 0.04) as i32, (h * 0.1) as i32, (w * 0.1) as i32, (h * 0.2) as i32, t, [(10, 14, 44), (40, 50, 100)]);
    glow(cv, w * 0.05, h * 0.65, w * 0.3, (255, 150, 60), 0.3 * (0.8 + 0.2 * (t * 11.0).sin()));
    let s = (h / 60.0).max(0.7);
    cast().santa_chair.draw(cv, &pose(w * 0.17, h * 0.74, s * 1.3, 1.1));
    // snores drifting up toward the top
    let big = ((h / 40.0) as i32).max(1);
    for k in 0..3 {
        let a = (t * 0.6 + k as f32 / 3.0).fract();
        let c = mix((40, 30, 50), (230, 236, 255), 1.0 - (a - 0.5).abs() * 2.0);
        big_text_at(cv, "Z", (w * (0.2 + a * 0.12)) as i32, (h * (0.32 - a * 0.25)) as i32, big + (a * 2.0) as i32, c);
    }
}

/// Santa charging in his plough, for the right panel.
fn plough_charge(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    for y in 0..cv.h as i32 {
        for x in 0..cv.w as i32 {
            let f = y as f32 / h;
            let c = if f < 0.45 { mix((120, 30, 40), (240, 120, 80), f / 0.45) } else { mix((214, 222, 244), (246, 248, 255), (f - 0.45) / 0.55) };
            cv.put(x, y, c);
        }
    }
    for k in 0..16 {
        let a = k as f32 / 16.0 * std::f32::consts::TAU + t * 0.5;
        for r in (w * 0.05) as i32..(w * 0.5) as i32 {
            if (r / 3) % 2 == 0 {
                cv.blend((w * 0.82 + a.cos() * r as f32) as i32, (h * 0.6 + a.sin() * r as f32) as i32, (255, 230, 120), 0.2);
            }
        }
    }
    let s = (h / 50.0).max(0.7);
    let shake = (t * 40.0).sin() * s * 0.6;
    cast().plough.draw(cv, &pose(w * 0.8 + shake, h * 0.97, s * 1.1, 2.6));
    glow(cv, w * 0.72, h * 0.86, h * 0.14, (255, 250, 200), 0.6);
    for k in 0..50 {
        let hk = hash(k, (t * 20.0) as i32);
        let (x, y) = ((w * 0.6) as i32 + (hk % (w * 0.4) as u32) as i32, (h * 0.8) as i32 + ((hk >> 10) % (h * 0.2) as u32) as i32);
        cv.blend(x, y, (255, 255, 255), 0.7);
    }
}

/// The intro page: three slanted panels slamming in one after another,
/// the page shaking as each lands, sound words bursting over the gutters.
pub fn draw_comic(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w, cv.h);
    let (wf, hf) = (w as f32, h as f32);
    // the page: a gutter-white sheet with ink outlines along the gutters
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            cv.put(x, y, (248, 244, 232));
        }
    }
    // which panels are in, and the newest one's slam
    let landed = |k: usize| t >= BEAT * k as f32;
    let newest = (0..PANELS).rev().find(|&k| landed(k)).unwrap_or(0);
    let since_new = t - BEAT * newest as f32;
    let slam = (since_new / SLAM).min(1.0);
    // the page jolts as a panel lands
    let jolt = if since_new < 0.18 { ((since_new * 90.0).sin() * 3.0 * (1.0 - since_new / 0.18)) as i32 } else { 0 };
    for k in 0..PANELS {
        if !landed(k) {
            continue;
        }
        let mut panel = Canvas::new(w as u16, (h / 2) as u16);
        let since = t - BEAT * k as f32;
        match k {
            0 => asleep(&mut panel, since),
            1 => road_up(&mut panel, since),
            _ => plough_charge(&mut panel, since),
        }
        let zoom = if k == newest { 1.0 + (1.0 - slam) * 0.25 } else { 1.0 };
        let flash = if k == newest { (1.0 - slam) * 0.9 } else { 0.0 };
        let (cx, cy) = (wf / 2.0, hf / 2.0);
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                if panel_at(x as f32, y as f32, wf, hf) != Some(k) {
                    continue;
                }
                let sx = (cx + (x as f32 - cx) / zoom) as usize;
                let sy = (cy + (y as f32 - cy) / zoom) as usize;
                if sx < panel.w && sy < panel.h {
                    cv.put(x + jolt, y, mix(panel.px[sy * panel.w + sx], (255, 255, 255), flash));
                }
            }
        }
    }
    // ink along each panel edge
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let here = panel_at(x as f32, y as f32, wf, hf);
            let right = panel_at(x as f32 + 1.0, y as f32, wf, hf);
            if here.is_some() != right.is_some() {
                let px = if here.is_some() { x } else { x + 1 };
                cv.put(px + jolt, y, (18, 14, 22));
                cv.put(px + jolt + if here.is_some() { -1 } else { 1 }, y, (18, 14, 22));
            }
        }
    }
    for x in 0..w as i32 {
        for y in [0, 1, h as i32 - 2, h as i32 - 1] {
            cv.put(x, y, (18, 14, 22));
        }
    }
    for y in 0..h as i32 {
        for x in [0, 1, w as i32 - 2, w as i32 - 1] {
            cv.put(x, y, (18, 14, 22));
        }
    }
    // the sound words, bursting over the gutters with an overshoot
    // each in clear space: over the top of the first gutter, the foot of
    // the road, and the sky above the plough
    let words = [("SNORE!", 0.44, 0.12), ("VROOOM!", 0.42, 0.88), ("HONK!!", 0.86, 0.3)];
    for (k, &(word, fx, fy)) in words.iter().enumerate() {
        let since = t - BEAT * k as f32 - SLAM * 0.6;
        if since < 0.0 {
            continue;
        }
        let pop = (since / 0.16).min(1.0);
        let overshoot = 1.0 + 0.35 * (1.0 - pop) * (pop * std::f32::consts::PI).sin() * 3.0;
        let mut scale = 2;
        while scale > 1 && crate::render::big_width(word, scale) as f32 > wf * 0.24 {
            scale -= 1;
        }
        let r = (crate::render::big_width(word, scale) as f32 * 0.62 + 3.0 * scale as f32) * pop.max(0.3) * overshoot.min(1.6);
        burst(cv, wf * fx, hf * fy, r, word, scale, 0.4 * k as f32 + since * 0.2);
    }
}
