//! The story's pictures: one still scene per page, drawn into the pixel
//! canvas with a little life in each (the fire, the steam, the snow, the
//! glint of the keys, the ticks going down the list).

use crate::model::{Model, Pose, Ride};
use crate::render::{big_text_at, ink_text_at, mix, Canvas, Rgb};
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
    bob_keys: Model,
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
        bob_keys: Model::elf_with_keys(&ROSTER[0].elf),
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

/// Page 2: the garage at night, Bob with the keys.
fn garage(cv: &mut Canvas, t: f32, dawn: bool) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    let floor = (h * 0.58) as i32;
    room(cv, floor, if dawn { 0.7 } else { 0.25 });
    // the big doors, open on the snow outside
    let (dx0, dx1, dy0) = ((w * 0.36) as i32, (w * 0.64) as i32, (h * 0.14) as i32);
    let sky = if dawn { [(80, 60, 120), (255, 170, 120)] } else { [(8, 12, 40), (40, 60, 110)] };
    for y in dy0..floor {
        for x in dx0..dx1 {
            let c = if y > floor - (h * 0.1) as i32 { if dawn { (250, 226, 214) } else { (180, 196, 230) } } else { mix(sky[0], sky[1], (y - dy0) as f32 / (floor - dy0) as f32) };
            cv.put(x, y, c);
        }
    }
    if !dawn {
        // a ribbon of aurora through the door
        for x in dx0..dx1 {
            let y = dy0 as f32 + (floor - dy0) as f32 * (0.3 + 0.1 * ((x as f32) * 0.15 + t * 0.6).sin());
            glow(cv, x as f32, y, 4.0, (60, 220, 150), 0.25);
        }
    }
    rect(cv, dx0 - 3, dy0 - 3, dx1 + 3, dy0, (60, 38, 24));
    rect(cv, dx0 - 3, dy0, dx0, floor, (60, 38, 24));
    rect(cv, dx1, dy0, dx1 + 3, floor, (60, 38, 24));
    let s = (h / 70.0).max(0.8);
    let c = cast();
    if dawn {
        // runner tracks out through the door
        for k in 0..5 {
            let lane = (k as f32 - 2.0) * w * 0.05;
            for y in floor..cv.h as i32 {
                let f = (y - floor) as f32 / (h - floor as f32);
                let x = w * 0.5 + lane * (0.3 + f * 1.4);
                cv.blend(x as i32, y, (40, 26, 16), 0.5);
                cv.blend(x as i32 + 3, y, (40, 26, 16), 0.5);
            }
        }
        // the plough, all that's left, with Santa already in it
        c.plough.draw(cv, &pose(w * 0.27, h * 0.95, s * 1.25, 0.6));
        let big = ((h / 22.0) as i32).max(2);
        let shake = ((t * 30.0).sin() * 1.5) as i32;
        big_text_at(cv, "!", (w * 0.2) as i32 + shake, (h * 0.12) as i32, big, (240, 60, 60));
    } else {
        // everything with runners, lined up
        c.reindeer.draw(cv, &pose(w * 0.12, h * 0.72, s * 0.8, 1.5));
        c.snowmobile.draw(cv, &pose(w * 0.82, h * 0.72, s * 0.8, -1.5));
        c.toboggan.draw(cv, &pose(w * 0.9, h * 0.92, s * 0.8, -1.4));
        c.sleigh.draw(cv, &pose(w * 0.27, h * 0.96, s * 1.0, 1.3));
        // Bob, keys held high, catching the light
        c.bob_keys.draw(cv, &pose(w * 0.5, h * 0.92, s * 1.5, 0.35));
        let glint = (t * 3.0).sin().max(0.0);
        glow(cv, w * 0.5 + s * 1.5 * 4.0, h * 0.92 - s * 1.5 * 19.0 * 0.97, h * 0.07, (255, 240, 160), 0.6 * glint);
    }
}

/// Page 4: the list.
fn the_list(cv: &mut Canvas, t: f32) {
    let (w, h) = (cv.w as f32, cv.h as f32);
    // the desk, by candlelight
    for y in 0..cv.h as i32 {
        for x in 0..cv.w as i32 {
            let grain = (hash(x / 6, y) % 7) as f32 / 7.0;
            cv.put(x, y, mix((58, 34, 22), (84, 52, 32), grain * 0.5));
        }
    }
    glow(cv, w * 0.82, h * 0.25, h * 0.9, (255, 190, 100), 0.35);
    rect(cv, (w * 0.8) as i32, (h * 0.22) as i32, (w * 0.84) as i32, (h * 0.5) as i32, (240, 232, 210));
    let flame = 0.5 + 0.5 * (t * 12.0).sin();
    glow(cv, w * 0.82, h * 0.19, 4.0 + flame * 2.0, (255, 220, 120), 1.0);
    // the scroll, unrolled
    let (sx0, sx1) = ((w * 0.28) as i32, (w * 0.66) as i32);
    let (sy0, sy1) = ((h * 0.05) as i32, (h * 0.66) as i32);
    for y in sy0..sy1 {
        for x in sx0..sx1 {
            let n = (hash(x / 3, y / 3) % 6) as f32 / 6.0;
            cv.put(x, y, mix((236, 222, 186), (214, 196, 156), n * 0.6));
        }
    }
    rect(cv, sx0 - 3, sy0 - 2, sx1 + 3, sy0 + 2, (196, 170, 120));
    rect(cv, sx0 - 3, sy1 - 2, sx1 + 3, sy1 + 2, (196, 170, 120));
    let scale = ((h / 60.0) as i32).max(1);
    let line = 7 * scale;
    ink_text_at(cv, "NAUGHTY", sx0 + 4 * scale, sy0 + 4 * scale, scale, (180, 30, 40));
    ink_text_at(cv, "NAUGHTY", sx0 + 4 * scale + 1, sy0 + 4 * scale, scale, (180, 30, 40));
    // names, and a tick going down the list, then a second: checked twice
    for (i, who) in ROSTER.iter().enumerate() {
        let y = sy0 + 4 * scale + line * (i as i32 + 1) + 2 * scale;
        let name = who.name.to_uppercase();
        ink_text_at(cv, &name, sx0 + 4 * scale, y, scale, (60, 40, 30));
        for pass in 0..2 {
            let due = 1.0 + i as f32 * 0.35 + pass as f32 * 2.4;
            if t > due {
                let tx = sx1 - (8 - pass * 4) * scale;
                let ty = y + 4 * scale;
                for k in 0..3 * scale {
                    cv.put(tx - k / 2, ty - k / 2 + 2 * scale / 2, (190, 30, 40));
                }
                for k in 0..5 * scale {
                    cv.put(tx + k / 2, ty - k, (190, 30, 40));
                }
            }
        }
    }
}

/// Draw the picture for a story page, `t` seconds after it came up.
pub fn draw(cv: &mut Canvas, page: usize, t: f32) {
    match page {
        0 => workshop(cv, t),
        1 => garage(cv, t, false),
        2 => garage(cv, t, true),
        _ => the_list(cv, t),
    }
}
