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

/// Page 1: Santa asleep by the fire.
fn workshop(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    let floor = (h * 0.62) as i32;
    room(cv, floor, 0.4);
    window(cv, (w * 0.08) as i32, (h * 0.12) as i32, (w * 0.16) as i32, (h * 0.26) as i32, t, [(10, 14, 44), (40, 50, 100)]);
    // the fireplace
    let (fx0, fx1) = ((w * 0.62) as i32, (w * 0.86) as i32);
    let (fy0, fy1) = ((h * 0.28) as i32, floor + 2);
    for y in fy0..fy1 {
        for x in fx0..fx1 {
            let stone = mix((120, 116, 120), (86, 82, 90), (hash(x / 3, y / 2) % 5) as f32 / 5.0);
            cv.put(x, y, stone);
        }
    }
    rect(cv, fx0 - 2, fy0 - 3, fx1 + 2, fy0, (70, 44, 28));
    let (ox0, ox1, oy0) = (fx0 + (fx1 - fx0) / 5, fx1 - (fx1 - fx0) / 5, fy0 + (fy1 - fy0) / 3);
    rect(cv, ox0, oy0, ox1, fy1, (24, 12, 10));
    // flames: a flickering heap
    let fw = (ox1 - ox0) as f32;
    for x in ox0..ox1 {
        let u = (x - ox0) as f32 / fw;
        let lick = (t * 9.0 + u * 13.0).sin() * 0.5 + (t * 13.0 - u * 7.0).sin() * 0.3;
        let height = ((1.0 - (u - 0.5).abs() * 2.0) * (fy1 - oy0) as f32 * (0.55 + 0.15 * lick)).max(0.0);
        for k in 0..height as i32 {
            let f = k as f32 / height.max(1.0);
            let c = mix((255, 236, 140), (220, 60, 20), f);
            cv.put(x, fy1 - 2 - k, c);
        }
    }
    let flicker = 0.75 + 0.25 * (t * 11.0).sin() * (t * 7.3).cos();
    glow(cv, (ox0 + ox1) as f32 / 2.0, fy1 as f32 - 4.0, w * 0.45, (255, 150, 60), 0.35 * flicker);
    // the cocoa on a little table, steaming
    let (tx, ty) = ((w * 0.5) as i32, (h * 0.66) as i32);
    rect(cv, tx - 6, ty, tx + 6, ty + 2, (110, 72, 44));
    rect(cv, tx - 1, ty + 2, tx + 1, ty + 12, (90, 58, 36));
    rect(cv, tx - 2, ty - 5, tx + 2, ty, (236, 236, 240));
    cv.put(tx + 2, ty - 3, (236, 236, 240));
    for k in 0..6 {
        let rise = ((t * 0.6 + k as f32 / 6.0).fract()) * 10.0;
        let sway = (t * 2.0 + k as f32).sin() * 1.5;
        cv.blend((tx as f32 + sway) as i32, ty - 6 - rise as i32, (230, 230, 240), 0.6 * (1.0 - rise / 10.0));
    }
    // Santa in his chair, three-quarters on
    let s = (h / 70.0).max(0.8);
    cast().santa_chair.draw(cv, &pose(w * 0.3, h * 0.86, s * 1.3, 1.25));
    // snores
    let big = ((h / 40.0) as i32).max(1);
    for k in 0..3 {
        let a = ((t * 0.5 + k as f32 / 3.0).fract()) as f32;
        let x = w * 0.30 + a * w * 0.12;
        let y = h * 0.30 - a * h * 0.2;
        let c = mix((40, 30, 50), (220, 230, 255), 1.0 - (a - 0.5).abs() * 2.0);
        big_text_at(cv, "Z", x as i32, y as i32, big + (a * 2.0) as i32, c);
    }
}

// ------------------------------------------------------------- the comic

/// Seconds between panels slamming in, and how long a slam takes.
pub const BEAT: f32 = 1.5;
const SLAM: f32 = 0.28;
pub const PANELS: usize = 3;

/// When the whole page is showing.
pub fn comic_done(t: f32) -> bool {
    t >= BEAT * (PANELS - 1) as f32 + SLAM + 0.5
}

/// Panel rectangles in pixels: one wide panel on top, two below.
/// Edges fall on even rows so captions line up with terminal cells.
pub fn comic_rects(w: usize, h: usize) -> [(i32, i32, i32, i32); PANELS] {
    let (w, h) = (w as i32, h as i32);
    let even = |v: i32| v & !1;
    let m = 2;
    let mid = even(h * 50 / 100);
    let split = w * 47 / 100;
    [(m, m, w - m, mid - 2), (m, mid + 2, split - 2, h - m), (split + 2, mid + 2, w - m, h - m)]
}

/// An action scene for panel 2: everything with runners bursting out of
/// the workshop doors into the snow.
fn breakout(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    // night snow outside, the open doors glowing behind
    for y in 0..cv.h as i32 {
        for x in 0..cv.w as i32 {
            let f = y as f32 / h;
            let c = if f < 0.45 { mix((14, 18, 52), (46, 60, 120), f / 0.45) } else { mix((200, 214, 240), (240, 244, 255), (f - 0.45) / 0.55) };
            cv.put(x, y, c);
        }
    }
    rect(cv, (w * 0.3) as i32, (h * 0.12) as i32, (w * 0.7) as i32, (h * 0.5) as i32, (60, 38, 24));
    rect(cv, (w * 0.34) as i32, (h * 0.17) as i32, (w * 0.66) as i32, (h * 0.5) as i32, (255, 214, 140));
    glow(cv, w * 0.5, h * 0.45, w * 0.5, (255, 200, 120), 0.35);
    // speed lines, bursting outward
    for k in 0..40 {
        let a = k as f32 / 40.0 * std::f32::consts::TAU + (hash(k, 3) % 100) as f32 * 0.01;
        let r0 = w * 0.18 + ((t * 3.0 + k as f32 * 0.37).fract()) * w * 0.2;
        for s in 0..(w * 0.12) as i32 {
            let r = r0 + s as f32;
            cv.blend((w * 0.5 + a.cos() * r) as i32, (h * 0.45 + a.sin() * r * 0.6) as i32, (255, 255, 255), 0.5);
        }
    }
    let s = (h / 50.0).max(0.8);
    let c = cast();
    // charging at us, fanned out
    c.reindeer.draw(cv, &pose(w * 0.18, h * 0.86, s * 1.2, 3.6));
    c.snowmobile.draw(cv, &pose(w * 0.82, h * 0.86, s * 1.2, 2.6));
    c.toboggan.draw(cv, &pose(w * 0.68, h * 0.66, s * 0.8, 3.0));
    c.sleigh.draw(cv, &pose(w * 0.46, h * 0.98, s * 1.7, 3.3));
}

/// Panel 3: Santa in his plough, charging, headlights blazing.
fn charge(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    for y in 0..cv.h as i32 {
        for x in 0..cv.w as i32 {
            let f = y as f32 / h;
            let c = if f < 0.4 { mix((120, 30, 40), (240, 120, 80), f / 0.4) } else { mix((214, 222, 244), (246, 248, 255), (f - 0.4) / 0.6) };
            cv.put(x, y, c);
        }
    }
    // furious rays behind him
    for k in 0..18 {
        let a = k as f32 / 18.0 * std::f32::consts::TAU + t * 0.4;
        for r in (w * 0.1) as i32..(w * 0.9) as i32 {
            if (r / 3) % 2 == 0 {
                cv.blend((w * 0.5 + a.cos() * r as f32) as i32, (h * 0.42 + a.sin() * r as f32) as i32, (255, 230, 120), 0.18);
            }
        }
    }
    let s = (h / 45.0).max(0.8);
    let shake = (t * 40.0).sin() * s * 0.6;
    let p = pose(w * 0.5 + shake, h * 0.95, s * 1.1, 2.75);
    cast().plough.draw(cv, &p);
    // headlights and spray
    glow(cv, w * 0.36, h * 0.8, h * 0.2, (255, 250, 200), 0.6);
    glow(cv, w * 0.64, h * 0.8, h * 0.2, (255, 250, 200), 0.6);
    for k in 0..60 {
        let hk = hash(k, (t * 20.0) as i32);
        let (x, y) = ((hk % cv.w as u32) as i32, (h * 0.75) as i32 + ((hk >> 10) % (h * 0.25) as u32) as i32);
        cv.blend(x, y, (255, 255, 255), 0.7);
    }
}

/// A comic starburst behind a sound word.
fn burst(cv: &mut Canvas, cx: f32, cy: f32, r: f32, word: &str, scale: i32, spin: f32) {
    let points = 14;
    let inside = |x: f32, y: f32| -> Option<bool> {
        let (dx, dy) = (x - cx, (y - cy) * 1.4);
        let a = dy.atan2(dx) + spin;
        let k = (a / std::f32::consts::TAU * points as f32 * 2.0).rem_euclid(points as f32 * 2.0);
        let f = k.fract();
        let edge = if (k as i32) % 2 == 0 { r * (0.62 + 0.38 * f) } else { r * (1.0 - 0.38 * f) };
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

/// The intro page: three panels slamming in one after another.
pub fn draw_comic(cv: &mut Canvas, t: f32) {
    // the page: off-white paper with a faint halftone
    for y in 0..cv.h as i32 {
        for x in 0..cv.w as i32 {
            let dot = (x % 4 == 0) && (y % 4 == 0);
            cv.put(x, y, if dot { (226, 220, 206) } else { (246, 242, 230) });
        }
    }
    let rects = comic_rects(cv.w, cv.h);
    let sfx = ["SNORE!", "VROOOM!", "HONK!!"];
    for (k, &(x0, y0, x1, y1)) in rects.iter().enumerate() {
        let since = t - BEAT * k as f32;
        if since < 0.0 {
            continue;
        }
        let (pw, ph) = ((x1 - x0) as usize, (y1 - y0) as usize);
        let mut panel = Canvas::new(pw as u16, (ph / 2).max(1) as u16);
        match k {
            0 => workshop(&mut panel, since),
            1 => breakout(&mut panel, since),
            _ => charge(&mut panel, since),
        }
        // slam: in from a little larger, with a white flash
        let slam = (since / SLAM).min(1.0);
        let zoom = 1.0 + (1.0 - slam) * 0.35;
        let flash = (1.0 - slam) * 0.8;
        let (cx, cy) = (pw as f32 / 2.0, ph as f32 / 2.0);
        for y in 0..ph as i32 {
            for x in 0..pw as i32 {
                let sx = (cx + (x as f32 - cx) / zoom) as usize;
                let sy = (cy + (y as f32 - cy) / zoom) as usize;
                if sx < panel.w && sy < panel.h {
                    let c = panel.px[sy * panel.w + sx];
                    cv.put(x0 + x, y0 + y, mix(c, (255, 255, 255), flash));
                }
            }
        }
        // a heavy black border
        for b in 0..2 {
            for x in x0 - b..x1 + b {
                cv.put(x, y0 - b - 1, (16, 14, 20));
                cv.put(x, y1 + b, (16, 14, 20));
            }
            for y in y0 - b - 1..y1 + b + 1 {
                cv.put(x0 - b - 1, y, (16, 14, 20));
                cv.put(x1 + b, y, (16, 14, 20));
            }
        }
        // the sound word pops a beat after the panel lands
        if since > SLAM * 0.8 {
            let pop = ((since - SLAM * 0.8) / 0.18).min(1.0);
            // as big as fits comfortably in the panel
            let pw = (x1 - x0) as f32;
            let mut scale = 2;
            while scale > 1 && crate::render::big_width(sfx[k], scale) as f32 > pw * 0.42 {
                scale -= 1;
            }
            let word = crate::render::big_width(sfx[k], scale) as f32;
            let r = (word * 0.62 + 3.0 * scale as f32) * (0.6 + 0.4 * pop);
            // tucked into a corner, wholly inside the panel
            let (bx, by) = match k {
                0 => (x1 as f32 - r * 1.25, y0 as f32 + r * 0.85),
                1 => (x0 as f32 + r * 1.25, y0 as f32 + r * 0.95 + 6.0),
                _ => (x1 as f32 - r * 1.4, y0 as f32 + r * 0.95 + 6.0),
            };
            if pop >= 0.6 {
                burst(cv, bx, by, r, sfx[k], scale, 0.3 * k as f32 + since * 0.15);
            }
        }
    }
}
