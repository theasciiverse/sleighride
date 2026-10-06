//! Racers as little voxel models, drawn every frame at their real angle to
//! the camera: you see a side as it turns, it rolls out of a drift, and a
//! spin-out turns it right round.
//!
//! Model axes: x forward (0 at the back), y right, z up. Each voxel is VOX
//! world units (times the model's own size factor). Only voxels with an open
//! face are kept, each with a normal from its open sides, so the model can be
//! lit from the camera's side.

use crate::render::{mix, Canvas, Rgb};

/// World units per voxel.
pub const VOX: f32 = 0.18;
/// Animation frames per cycle (a stride, a turn of the tracks).
pub const FRAMES: usize = 4;
const LX: i32 = 32;
const LY: i32 = 21;
const LZ: i32 = 30;
const OUTLINE: Rgb = (26, 18, 40);

const RED: Rgb = (214, 38, 46);
const RED_DARK: Rgb = (128, 20, 30);
const GOLD: Rgb = (242, 194, 62);
const GOLD_DARK: Rgb = (170, 122, 32);
const WHITE: Rgb = (246, 246, 252);
const SKIN: Rgb = (238, 188, 150);
const BLUE: Rgb = (60, 110, 220);
const WOOD: Rgb = (150, 98, 56);
const WOOD_DARK: Rgb = (104, 66, 38);
const BLACK: Rgb = (34, 30, 40);
const GREY: Rgb = (120, 124, 140);
const GREY_DARK: Rgb = (70, 72, 86);
const GLASS: Rgb = (150, 210, 240);
const FUR: Rgb = (132, 86, 50);
const FUR_DARK: Rgb = (92, 58, 34);
const FUR_PALE: Rgb = (214, 186, 150);
const ANTLER: Rgb = (226, 206, 168);
const YELLOW: Rgb = (250, 206, 40);

/// What a racer rides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ride {
    Sleigh,
    Reindeer,
    Snowmobile,
    Toboggan,
    Skis,
    /// Julbocken, the straw Yule goat, with a Tomte riding.
    Goat,
    /// La Befana's broomstick, flying low.
    Broom,
    /// Santa's snowplough: the one thing the helpers left behind.
    Plough,
}

/// An elf's colours.
#[derive(Clone, Copy, Debug)]
pub struct Elf {
    pub hat: Rgb,
    pub coat: Rgb,
    pub hair: Rgb,
    /// The ride's own paint (sleigh tub, snowmobile body, saddle, sled rails).
    pub paint: Rgb,
}

fn darker(c: Rgb) -> Rgb {
    ((c.0 as f32 * 0.62) as u8, (c.1 as f32 * 0.62) as u8, (c.2 as f32 * 0.62) as u8)
}

/// A visible voxel: model position, colour and outward normal.
#[derive(Clone, Copy)]
struct Vox {
    p: [f32; 3],
    n: [f32; 3],
    c: Rgb,
}

pub struct Model {
    voxels: Vec<Vox>,
    /// Model x that sits on the racer's position.
    pivot: f32,
    /// Farthest voxel from the pivot, for sizing the draw buffer.
    reach: f32,
    /// Size factor on VOX.
    pub size: f32,
    /// Back ends of the runners/skis/track, where drift sparks come off.
    pub sparks: [[f32; 3]; 2],
}

struct Grid(Vec<Option<Rgb>>);

impl Grid {
    fn new() -> Grid {
        Grid(vec![None; (LX * LY * LZ) as usize])
    }

    fn idx(x: i32, y: i32, z: i32) -> Option<usize> {
        let yy = y + LY / 2;
        if x < 0 || yy < 0 || z < 0 || x >= LX || yy >= LY || z >= LZ {
            return None;
        }
        Some(((z * LY + yy) * LX + x) as usize)
    }

    fn set(&mut self, x: i32, y: i32, z: i32, c: Rgb) {
        if let Some(i) = Grid::idx(x, y, z) {
            self.0[i] = Some(c);
        }
    }

    fn get(&self, x: i32, y: i32, z: i32) -> Option<Rgb> {
        Grid::idx(x, y, z).and_then(|i| self.0[i])
    }

    fn block(&mut self, x: (i32, i32), y: (i32, i32), z: (i32, i32), c: Rgb) {
        for zz in z.0..=z.1 {
            for yy in y.0..=y.1 {
                for xx in x.0..=x.1 {
                    self.set(xx, yy, zz, c);
                }
            }
        }
    }

    /// Ellipsoid with radii (rx, ry, rz).
    fn blob(&mut self, c: (f32, f32, f32), r: (f32, f32, f32), col: Rgb) {
        for z in (c.2 - r.2).floor() as i32..=(c.2 + r.2).ceil() as i32 {
            for y in (c.1 - r.1).floor() as i32..=(c.1 + r.1).ceil() as i32 {
                for x in (c.0 - r.0).floor() as i32..=(c.0 + r.0).ceil() as i32 {
                    let d = ((x as f32 - c.0) / r.0).powi(2) + ((y as f32 - c.1) / r.1).powi(2) + ((z as f32 - c.2) / r.2).powi(2);
                    if d <= 1.0 {
                        self.set(x, y, z, col);
                    }
                }
            }
        }
    }

    fn ball(&mut self, c: (f32, f32, f32), r: f32, col: Rgb) {
        self.blob(c, (r, r, r), col);
    }

    fn line(&mut self, a: (f32, f32, f32), b: (f32, f32, f32), col: Rgb) {
        let n = ((b.0 - a.0).abs().max((b.1 - a.1).abs()).max((b.2 - a.2).abs()) * 2.0).ceil() as i32 + 1;
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.set((a.0 + (b.0 - a.0) * t).round() as i32, (a.1 + (b.1 - a.1) * t).round() as i32, (a.2 + (b.2 - a.2) * t).round() as i32, col);
        }
    }

    /// A runner along y = `side` that curls up and back over at the front.
    fn runner(&mut self, side: i32, from: i32, to: i32, curl: f32, col: Rgb) {
        for x in from..=to {
            self.set(x, side, 0, col);
        }
        for k in 0..=40 {
            let a = -std::f32::consts::FRAC_PI_2 + k as f32 / 40.0 * 3.6;
            let (x, z) = (to as f32 + curl * a.cos(), curl + curl * a.sin());
            self.set(x.round() as i32, side, z.round() as i32, col);
        }
    }

    /// A seated elf whose seat is at (x, z): coat, arms reaching `reach`
    /// forward, head with hair at the back, pointed ears, a drooping hat.
    fn elf(&mut self, x: i32, z: i32, reach: i32, e: &Elf, frame: usize) {
        let (dx, dz) = (x - 7, z - 6);
        self.block((5 + dx, 8 + dx), (-2, 2), (6 + dz, 9 + dz), e.coat);
        self.block((5 + dx, 5 + dx), (-2, 2), (6 + dz, 9 + dz), darker(e.coat));
        self.block((5 + dx, 8 + dx), (-2, 2), (9 + dz, 9 + dz), WHITE);
        for side in [-3, 3] {
            self.block((8 + dx, 8 + dx + reach), (side, side), (7 + dz, 8 + dz), e.coat);
            self.set(9 + dx + reach, side, 8 + dz, WHITE);
        }
        self.ball((7.0 + dx as f32, 0.0, 11.5 + dz as f32), 2.6, SKIN);
        for zz in (9 + dz)..=(14 + dz) {
            for y in -3..=3 {
                for xx in (3 + dx)..=(7 + dx) {
                    if self.get(xx, y, zz) == Some(SKIN) {
                        self.set(xx, y, zz, e.hair);
                    }
                }
            }
        }
        for side in [-1, 1] {
            self.set(7 + dx, side * 3, 11 + dz, SKIN);
            self.set(7 + dx, side * 4, 12 + dz, SKIN);
            self.set(6 + dx, side * 5, 13 + dz, SKIN);
        }
        for y in -3..=3 {
            for xx in 4..=10 {
                if (xx - 7) * (xx - 7) + y * y <= 8 {
                    self.set(xx + dx, y, 14 + dz, WHITE);
                }
            }
        }
        for (i, zz) in (15..=21).enumerate() {
            let r = 2.6 - i as f32 * 0.36;
            let cx = 7.0 - i as f32 * 0.5 + dx as f32;
            for y in -3..=3 {
                for xx in (1 + dx)..=(10 + dx) {
                    let d = ((xx as f32 - cx).powi(2) + (y as f32).powi(2)).sqrt();
                    if d <= r.max(0.7) {
                        self.set(xx, y, zz + dz, if y > 0 { darker(e.hat) } else { e.hat });
                    }
                }
            }
        }
        // the bobble swings with the ride
        let sway = [0.0f32, 0.6, 0.0, -0.6][frame % 4];
        self.ball((3.5 + dx as f32 - sway.abs() * 0.5, sway, 22.5 + dz as f32), 1.3, WHITE);
    }

    /// Santa, seated at (x, z): big red coat with white trim, belt, beard.
    fn santa(&mut self, x: i32, z: i32) {
        let (cx, cz) = (x as f32, z as f32);
        self.blob((cx, 0.0, cz + 4.5), (4.0, 4.5, 4.8), RED);
        self.block((x - 4, x + 4), (-5, 5), (z + 2, z + 2), BLACK);
        self.set(x + 4, 0, z + 2, GOLD);
        self.set(x + 4, 1, z + 2, GOLD);
        for side in [-5.0f32, 5.0] {
            self.line((cx, side, cz + 7.0), (cx + 6.0, side * 0.9, cz + 5.0), RED);
            self.ball((cx + 6.5, side * 0.9, cz + 5.0), 1.0, WHITE);
        }
        self.blob((cx, 0.0, cz + 8.6), (3.6, 4.6, 1.0), WHITE);
        self.ball((cx + 0.5, 0.0, cz + 11.5), 3.0, SKIN);
        // the beard, from the front of the face down onto the chest
        self.blob((cx + 2.2, 0.0, cz + 9.6), (2.0, 3.4, 3.0), WHITE);
        self.set(x + 3, -1, z + 12, BLACK);
        self.set(x + 3, 1, z + 12, BLACK);
        self.set(x + 4, 0, z + 11, (230, 120, 110));
        self.blob((cx + 0.5, 0.0, cz + 13.6), (3.3, 3.3, 0.9), WHITE);
        for (i, zz) in (15..=19).enumerate() {
            let r = 3.0 - i as f32 * 0.55;
            self.blob((cx + 0.5 - i as f32 * 0.7, 0.0, cz + zz as f32 - 0.5), (r.max(0.8), r.max(0.8), 0.6), RED);
        }
        self.ball((cx - 3.3, 0.0, cz + 19.0), 1.4, WHITE);
    }
}

impl Model {
    /// All animation frames of a ride.
    pub fn frames(ride: Ride, e: &Elf) -> [Model; FRAMES] {
        std::array::from_fn(|f| Model::build(ride, e, f))
    }

    /// One animation frame (0..FRAMES) of a ride.
    pub fn build(ride: Ride, e: &Elf, frame: usize) -> Model {
        let mut g = Grid::new();
        let (pivot, size, sparks) = match ride {
            Ride::Sleigh => {
                for side in [-6, 6] {
                    g.runner(side, 1, 22, 3.2, GOLD);
                    g.set(0, side, 1, GOLD);
                    for x in [4, 11, 18] {
                        g.block((x, x), (side, side), (1, 2), GOLD_DARK);
                    }
                }
                g.block((2, 20), (-5, 5), (3, 3), darker(e.paint));
                for x in 2..=20 {
                    let top = if x <= 14 { 7 } else { 7 + (x - 14).min(4) };
                    for side in [-5, 5] {
                        g.block((x, x), (side, side), (3, top - 1), e.paint);
                        g.set(x, side, top, GOLD);
                    }
                }
                g.block((20, 21), (-5, 5), (3, 10), e.paint);
                g.block((21, 22), (-5, 5), (11, 11), GOLD);
                g.block((2, 2), (-5, 5), (3, 9), e.paint);
                g.block((2, 2), (-5, 5), (10, 10), GOLD);
                g.block((1, 1), (-1, 1), (5, 7), GOLD);
                g.block((3, 8), (-4, 4), (4, 5), darker(e.paint));
                g.block((13, 15), (-4, -2), (4, 6), (40, 170, 72));
                g.block((14, 14), (-4, -2), (4, 7), GOLD);
                g.block((16, 18), (1, 4), (4, 7), BLUE);
                g.block((16, 18), (2, 2), (4, 8), WHITE);
                g.block((13, 14), (1, 3), (4, 5), WOOD);
                g.elf(7, 6, 3, e, frame);
                (12.0, 1.0, [[0.0, -6.0, 0.0], [0.0, 6.0, 0.0]])
            }
            Ride::Reindeer => {
                // a gallop in four beats: stretched out, passing, gathered
                // under, passing; (reach forward, hoof lift) per leg, the two
                // legs of a pair a little out of step
                let beat: [[(f32, f32); 2]; 4] = [[(4.0, 0.0), (3.0, 1.0)], [(0.5, 2.0), (1.5, 0.0)], [(-3.0, 0.5), (-2.0, 0.0)], [(-0.5, 0.0), (0.5, 2.0)]];
                let back: [[(f32, f32); 2]; 4] = [[(-4.0, 0.5), (-3.0, 0.0)], [(-0.5, 0.0), (0.5, 2.0)], [(3.0, 0.0), (2.0, 1.0)], [(0.5, 2.0), (-0.5, 0.0)]];
                for (lx, set) in [(19.0f32, &beat[frame % 4]), (8.0, &back[frame % 4])] {
                    for (k, ly) in [-2, 2].into_iter().enumerate() {
                        let (fwd, lift) = set[k];
                        // a knee halfway down, bent against the swing
                        let knee = (lx + fwd * 0.4 - fwd.signum() * 0.8, ly as f32, 4.5 + lift * 0.4);
                        let hoof = (lx + fwd, ly as f32, 1.0 + lift);
                        g.line((lx, ly as f32, 8.0), knee, FUR_DARK);
                        g.line(knee, hoof, FUR_DARK);
                        g.set(hoof.0.round() as i32, ly, (hoof.2 - 1.0).round() as i32, BLACK);
                    }
                }
                g.blob((13.5, 0.0, 9.5), (8.0, 3.2, 3.4), FUR);
                g.blob((13.5, 0.0, 8.0), (6.0, 2.6, 1.8), FUR_PALE);
                g.ball((5.0, 0.0, 11.0), 1.2, WHITE);
                for k in 0..=6 {
                    let t = k as f32 / 6.0;
                    g.ball((20.0 + t * 3.5, 0.0, 11.0 + t * 4.5), 1.9 - t * 0.4, FUR);
                }
                g.blob((25.0, 0.0, 16.0), (2.2, 1.7, 1.6), FUR);
                g.blob((27.0, 0.0, 15.4), (1.2, 1.1, 1.0), FUR_PALE);
                g.set(28, 0, 16, BLACK);
                g.set(25, -2, 17, BLACK);
                g.set(25, 2, 17, BLACK);
                for side in [-1.0f32, 1.0] {
                    g.set(23, (side * 2.0) as i32, 17, FUR_DARK);
                    // antlers: a beam up and back, with tines
                    g.line((24.0, side * 1.5, 17.5), (23.0, side * 4.0, 23.0), ANTLER);
                    g.line((23.0, side * 4.0, 23.0), (21.0, side * 5.0, 26.0), ANTLER);
                    g.line((23.6, side * 3.0, 21.0), (26.0, side * 3.5, 23.0), ANTLER);
                    g.line((22.5, side * 4.5, 24.5), (24.5, side * 6.0, 26.5), ANTLER);
                }
                g.blob((12.5, 0.0, 12.2), (3.5, 3.6, 0.8), e.paint);
                g.line((26.0, -1.5, 15.0), (16.0, -3.0, 14.0), BLACK);
                g.line((26.0, 1.5, 15.0), (16.0, 3.0, 14.0), BLACK);
                g.elf(12, 12, 2, e, frame);
                (14.0, 1.0, [[5.0, -2.0, 0.0], [5.0, 2.0, 0.0]])
            }
            Ride::Snowmobile => {
                for side in [-5, 5] {
                    g.runner(side, 17, 25, 1.6, GREY_DARK);
                    g.line((20.0, side as f32, 1.0), (20.0, side as f32 * 0.6, 4.0), GREY_DARK);
                }
                // the track, with treads
                for x in 2..=13 {
                    for z in 0..=3 {
                        for y in -3..=3 {
                            let tread = (x + z + frame as i32) % 2 == 0 && (z == 0 || x == 2 || x == 13);
                            g.set(x, y, z, if tread { BLACK } else { GREY_DARK });
                        }
                    }
                }
                // body and hood, tapering to the nose
                for x in 3..=24 {
                    let w = if x < 16 { 4 } else { 4 - (x - 16) / 3 };
                    let top = if x < 14 { 5 } else { 7 - (x - 14) / 3 };
                    g.block((x, x), (-w, w), (3, top.max(3)), e.paint);
                }
                g.block((3, 24), (-4, 4), (3, 3), darker(e.paint));
                g.block((22, 23), (-2, 2), (4, 5), YELLOW);
                g.block((16, 17), (-3, 3), (8, 10), GLASS);
                g.line((15.0, -4.0, 9.0), (15.0, 4.0, 9.0), BLACK);
                g.block((5, 12), (-3, 3), (6, 6), BLACK);
                g.elf(9, 7, 4, e, frame);
                (13.0, 1.0, [[2.0, -3.0, 0.0], [2.0, 3.0, 0.0]])
            }
            Ride::Toboggan => {
                g.block((1, 23), (-4, 4), (1, 1), WOOD);
                for x in (1..=23).step_by(3) {
                    g.block((x, x), (-4, 4), (1, 1), WOOD_DARK);
                }
                // the high curled front
                for y in -4..=4 {
                    for k in 0..=40 {
                        let a = -std::f32::consts::FRAC_PI_2 + k as f32 / 40.0 * 3.8;
                        let (x, z) = (23.0 + 3.5 * a.cos(), 4.5 + 3.5 * a.sin());
                        g.set(x.round() as i32, y, z.round() as i32, if y.abs() == 4 { WOOD_DARK } else { WOOD });
                    }
                }
                for side in [-5, 5] {
                    g.block((2, 21), (side, side), (2, 2), e.paint);
                }
                g.line((24.0, 0.0, 7.0), (12.0, -3.0, 5.0), (200, 40, 40));
                g.line((24.0, 0.0, 7.0), (12.0, 3.0, 5.0), (200, 40, 40));
                g.block((3, 7), (-3, 3), (2, 3), e.paint);
                g.elf(7, 2, 3, e, frame);
                (12.0, 1.0, [[1.0, -4.0, 0.0], [1.0, 4.0, 0.0]])
            }
            Ride::Skis => {
                // skating: each ski in turn slides forward while the other
                // pushes back and out, the poles planting with the stride
                let stride = [2.0f32, 0.0, -2.0, 0.0][frame % 4];
                for (side, dir) in [(-2i32, 1.0f32), (2, -1.0)] {
                    let fwd = stride * dir;
                    // the pushing ski (sliding back) splays out at the tail
                    let splay = if fwd < 0.0 { side.signum() as f32 * 0.12 } else { 0.0 };
                    let ski = |x: i32| -> (i32, i32) {
                        let xf = x as f32 + fwd;
                        (xf.round() as i32, (side as f32 + (12.0 - xf).max(0.0) * splay).round() as i32)
                    };
                    for x in 0..=22 {
                        let (xx, yy) = ski(x);
                        g.set(xx, yy, 0, e.paint);
                    }
                    let (tx, ty) = ski(23);
                    g.set(tx, ty, 1, e.paint);
                    g.set(tx + 1, ty, 2, e.paint);
                    let (bx, by) = ski(1);
                    g.set(bx, by, 1, darker(e.paint));
                    // boot on the ski, leg up to the hip in a crouch
                    let (fx, fy) = ski(11);
                    g.block((fx - 1, fx + 1), (fy, fy), (1, 2), BLACK);
                    let knee = (12.5 + fwd * 0.5, (fy as f32 + side as f32) / 2.0, 6.0);
                    g.line((fx as f32, fy as f32, 3.0), knee, darker(e.coat));
                    g.line(knee, (10.0, side as f32, 9.0), darker(e.coat));
                }
                g.elf(10, 9, 3, e, frame);
                // poles: planted beside the boots, then swung back
                let plant = [true, false, true, false][frame % 4];
                for side in [-4.0f32, 4.0] {
                    let tip = if plant { (9.0, side * 1.3, 0.0) } else { (2.0, side * 1.2, 3.0) };
                    g.line((13.0, side, 10.0), tip, GREY);
                    g.set(tip.0 as i32, tip.1 as i32, tip.2 as i32 + 1, BLACK);
                }
                (11.0, 1.0, [[0.0, -2.0, 0.0], [0.0, 2.0, 0.0]])
            }
            Ride::Goat => {
                const STRAW: Rgb = (226, 186, 92);
                const STRAW_DARK: Rgb = (184, 142, 62);
                const RIBBON: Rgb = (206, 34, 50);
                // legs in a trot, bound with ribbon at the knee
                let swing = [[1.5f32, -1.5, -1.5, 1.5], [0.5, -0.5, 0.5, -0.5], [-1.5, 1.5, 1.5, -1.5], [-0.5, 0.5, -0.5, 0.5]][frame % 4];
                for (k, (lx, ly)) in [(6.0f32, -2.0f32), (6.0, 2.0), (17.0, -2.0), (17.0, 2.0)].into_iter().enumerate() {
                    g.line((lx, ly, 7.0), (lx + swing[k], ly, 0.0), STRAW_DARK);
                    g.set((lx + swing[k] * 0.5).round() as i32, ly as i32, 3, RIBBON);
                }
                // the body: a bundle of straw, ribbons wound round it
                g.blob((11.5, 0.0, 9.0), (8.0, 3.0, 3.0), STRAW);
                for x in [6, 11, 16] {
                    for y in -3..=3 {
                        for z in 6..=12 {
                            if g.get(x, y, z).is_some() {
                                g.set(x, y, z, RIBBON);
                            }
                        }
                    }
                }
                for x in (5..=18).step_by(2) {
                    g.set(x, 0, 12, STRAW_DARK);
                }
                // neck, head, beard and the great curling horns
                g.line((18.0, 0.0, 10.0), (21.0, 0.0, 15.0), STRAW);
                g.blob((22.5, 0.0, 15.5), (2.4, 1.6, 1.6), STRAW);
                g.set(25, 0, 15, STRAW_DARK);
                g.line((24.0, 0.0, 14.0), (24.0, 0.0, 12.0), STRAW_DARK);
                g.block((20, 20), (-2, 2), (14, 14), RIBBON);
                for side in [-1.0f32, 1.0] {
                    for k in 0..=20 {
                        let a = k as f32 / 20.0 * 4.2;
                        let r = 1.0 + a * 0.55;
                        g.set((21.5 - a.sin() * r).round() as i32, (side * (1.6 + a * 0.4)).round() as i32, (17.0 + a.cos() * r * 0.8 + a * 0.4).round() as i32, STRAW_DARK);
                    }
                }
                g.ball((3.5, 0.0, 10.5), 1.0, STRAW);
                // the Tomte: grey coat, red cap, a long white beard
                g.elf(11, 12, 3, e, frame);
                g.blob((10.5, 0.0, 15.5), (1.2, 2.0, 2.2), WHITE);
                (12.0, 1.0, [[5.0, -2.0, 0.0], [5.0, 2.0, 0.0]])
            }
            Ride::Broom => {
                // the handle, the bristles, and her sack of gifts
                g.line((4.0, 0.0, 6.0), (29.0, 0.0, 8.5), WOOD);
                g.line((4.0, 0.0, 5.0), (29.0, 0.0, 7.5), WOOD_DARK);
                for k in 0..=12 {
                    let t = k as f32 / 12.0;
                    g.blob((4.0 - t * 4.0, 0.0, 5.5), (1.6, 0.9 + t * 1.8, 0.9 + t * 1.6), if k % 3 == 0 { (176, 136, 70) } else { (206, 168, 96) });
                }
                g.block((6, 6), (-1, 1), (4, 7), RED_DARK);
                g.elf(13, 8, 5, e, frame);
                // sooty headscarf knot and a sack over the shoulder
                g.blob((8.0, 0.0, 12.0), (2.6, 2.6, 3.0), (130, 92, 60));
                g.set(8, 0, 15, GOLD);
                (14.0, 1.0, [[1.0, -2.0, 5.0], [1.0, 2.0, 5.0]])
            }
            Ride::Plough => {
                // chassis on caterpillar tracks
                for side in [-6, 6] {
                    for x in 1..=22 {
                        for z in 0..=3 {
                            let tread = (x + z + frame as i32) % 2 == 0 && (z == 0 || z == 3 || x == 1 || x == 22);
                            g.block((x, x), (side - 1, side + 1), (z, z), if tread { BLACK } else { GREY_DARK });
                        }
                    }
                }
                g.block((1, 23), (-5, 5), (2, 4), GREY_DARK);
                // engine hood and an open cab
                g.block((14, 23), (-5, 5), (5, 10), RED);
                g.block((14, 23), (-5, 5), (10, 10), RED_DARK);
                g.block((23, 23), (-4, 4), (6, 8), GREY);
                g.block((2, 13), (-6, 6), (5, 9), RED);
                g.block((3, 12), (-5, 5), (5, 8), RED_DARK);
                g.block((2, 13), (-6, -6), (9, 9), GOLD);
                g.block((2, 13), (6, 6), (9, 9), GOLD);
                g.block((2, 2), (-6, 6), (9, 9), GOLD);
                g.line((19.0, -4.0, 11.0), (19.0, -4.0, 17.0), GREY);
                g.ball((19.0, -4.0, 17.5), 0.8, BLACK);
                // the blade, striped
                for y in -9i32..=9 {
                    for z in 0i32..=8 {
                        let stripe = (y + z).rem_euclid(6) < 3;
                        let x = 26 + (y.abs() / 4) - (z / 5);
                        g.set(x, y, z, if stripe { YELLOW } else { BLACK });
                    }
                }
                g.line((23.0, -3.0, 4.0), (26.0, -3.0, 4.0), GREY);
                g.line((23.0, 3.0, 4.0), (26.0, 3.0, 4.0), GREY);
                // a beacon on a roll bar
                g.line((4.0, -6.0, 9.0), (4.0, -6.0, 15.0), GREY);
                g.line((4.0, 6.0, 9.0), (4.0, 6.0, 15.0), GREY);
                g.line((4.0, -6.0, 15.0), (4.0, 6.0, 15.0), GREY);
                g.ball((4.0, 0.0, 16.0), 1.0, (255, 150, 30));
                g.santa(7, 5);
                (13.0, 1.6, [[1.0, -6.0, 0.0], [1.0, 6.0, 0.0]])
            }
        };
        Model::finish(g, pivot, size, sparks)
    }

    fn finish(g: Grid, pivot: f32, size: f32, sparks: [[f32; 3]; 2]) -> Model {
        let mut voxels = Vec::new();
        let mut reach = 0.0f32;
        const DIRS: [(i32, i32, i32); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
        for z in 0..LZ {
            for y in -(LY / 2)..=(LY / 2) {
                for x in 0..LX {
                    let Some(c) = g.get(x, y, z) else { continue };
                    let mut n = [0.0f32; 3];
                    let mut open = false;
                    for (dx, dy, dz) in DIRS {
                        if g.get(x + dx, y + dy, z + dz).is_none() {
                            n[0] += dx as f32;
                            n[1] += dy as f32;
                            n[2] += dz as f32;
                            open = true;
                        }
                    }
                    if !open {
                        continue;
                    }
                    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    let n = if l < 1e-3 { [0.0, 0.0, 1.0] } else { [n[0] / l, n[1] / l, n[2] / l] };
                    let p = [x as f32 - pivot, y as f32, z as f32];
                    reach = reach.max((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt());
                    voxels.push(Vox { p, n, c });
                }
            }
        }
        Model { voxels, pivot, reach, size, sparks }
    }

    /// Santa asleep in his armchair, boots kicked off, in his socks.
    pub fn santa_armchair() -> Model {
        let mut g = Grid::new();
        const VELVET: Rgb = (150, 30, 44);
        // legs, seat, back and arms of the chair
        for (x, y) in [(3, -8), (3, 8), (17, -8), (17, 8)] {
            g.block((x, x), (y, y), (0, 2), WOOD_DARK);
        }
        g.block((2, 18), (-8, 8), (3, 6), VELVET);
        g.block((1, 4), (-8, 8), (3, 21), VELVET);
        g.block((1, 4), (-7, 7), (21, 22), darker(VELVET));
        for side in [-9, 9] {
            g.block((2, 17), (side, side), (3, 11), VELVET);
            g.block((2, 18), (side, side), (11, 12), darker(VELVET));
        }
        // Santa, sunk in, head lolling
        g.santa(9, 6);
        // legs out over the seat edge, socks on, toes up
        for side in [-3, 3] {
            g.block((14, 21), (side - 1, side + 1), (6, 8), RED);
            g.block((21, 23), (side - 1, side + 1), (1, 7), RED);
            g.block((23, 25), (side - 1, side + 1), (0, 2), WHITE);
        }
        // the boots, kicked off
        g.block((26, 29), (-7, -5), (0, 2), BLACK);
        g.block((27, 28), (-7, -5), (3, 5), BLACK);
        g.block((25, 28), (6, 9), (0, 2), BLACK);
        Model::finish(g, 12.0, 1.0, [[0.0, 0.0, 0.0]; 2])
    }

    /// An elf standing, one arm up holding a ring of keys.
    pub fn elf_with_keys(e: &Elf) -> Model {
        let mut g = Grid::new();
        for side in [-2, 2] {
            g.block((6, 7), (side - 1, side), (1, 5), darker(e.coat));
            g.block((6, 8), (side - 1, side), (0, 0), BLACK);
        }
        g.elf(7, 6, 1, e, 0);
        // the right arm raised, the ring of keys up high
        g.line((7.0, 3.0, 9.0), (8.0, 4.0, 16.0), e.coat);
        g.set(8, 4, 17, WHITE);
        for k in 0..12 {
            let a = k as f32 / 12.0 * std::f32::consts::TAU;
            g.set((8.0 + 1.6 * a.cos()).round() as i32, 5, (19.0 + 1.6 * a.sin()).round() as i32, GOLD);
        }
        for (dx, len) in [(-1, 3), (0, 4), (1, 3)] {
            g.line((8.0 + dx as f32, 5.0, 17.5), (8.0 + dx as f32 * 1.5, 5.0, 17.5 - len as f32), GOLD_DARK);
        }
        Model::finish(g, 7.0, 1.0, [[0.0, 0.0, 0.0]; 2])
    }

    /// A penguin: black back, white front, orange beak and feet.
    pub fn penguin() -> Model {
        let mut g = Grid::new();
        g.blob((4.0, 0.0, 4.5), (2.6, 2.6, 4.0), BLACK);
        g.blob((5.2, 0.0, 4.2), (1.6, 2.0, 3.3), WHITE);
        g.ball((4.3, 0.0, 9.0), 1.9, BLACK);
        g.blob((5.4, 0.0, 8.6), (0.8, 1.2, 1.0), WHITE);
        g.set(6, 0, 9, (240, 150, 40));
        g.set(7, 0, 9, (240, 150, 40));
        g.set(5, -1, 9, (250, 250, 250));
        g.set(5, 1, 9, (250, 250, 250));
        for side in [-1, 1] {
            g.line((4.0, side as f32 * 2.8, 6.5), (3.6, side as f32 * 3.4, 3.0), (30, 26, 40));
            g.block((5, 6), (side, side), (0, 0), (240, 150, 40));
        }
        Model::finish(g, 4.0, 1.0, [[0.0, 0.0, 0.0]; 2])
    }

    /// A polar bear on all fours (four walk frames), or reared up.
    pub fn bear(frame: usize, reared: bool) -> Model {
        let mut g = Grid::new();
        const FUR: Rgb = (240, 236, 222);
        const SHADE: Rgb = (214, 206, 190);
        if reared {
            // up on the hind legs, paws out to swipe
            for side in [-2.5f32, 2.5] {
                g.line((10.0, side, 0.0), (10.0, side, 6.0), SHADE);
            }
            g.blob((10.0, 0.0, 12.0), (3.6, 4.0, 7.0), FUR);
            g.ball((11.0, 0.0, 20.5), 2.8, FUR);
            g.blob((13.5, 0.0, 20.0), (1.4, 1.2, 1.1), FUR);
            g.set(15, 0, 20, BLACK);
            g.set(13, -1, 22, BLACK);
            g.set(13, 1, 22, BLACK);
            g.ball((10.0, -2.2, 23.0), 0.9, FUR);
            g.ball((10.0, 2.2, 23.0), 0.9, FUR);
            for side in [-1.0f32, 1.0] {
                g.line((10.0, side * 3.5, 16.0), (15.0, side * 6.0, 19.0), FUR);
                g.ball((15.5, side * 6.2, 19.0), 1.2, SHADE);
            }
        } else {
            // walking: legs in a slow four-beat
            let swing = [[1.5f32, -1.5, -1.5, 1.5], [0.5, -0.5, 0.5, -0.5], [-1.5, 1.5, 1.5, -1.5], [-0.5, 0.5, -0.5, 0.5]][frame % 4];
            for (k, (lx, ly)) in [(6.0f32, -2.5f32), (6.0, 2.5), (17.0, -2.5), (17.0, 2.5)].into_iter().enumerate() {
                g.line((lx, ly, 6.0), (lx + swing[k], ly, 0.0), SHADE);
                g.ball((lx + swing[k], ly, 0.6), 0.9, SHADE);
            }
            g.blob((11.5, 0.0, 7.5), (8.0, 4.0, 3.8), FUR);
            g.blob((19.5, 0.0, 9.0), (2.4, 2.2, 2.2), FUR);
            g.ball((22.0, 0.0, 9.5), 2.4, FUR);
            g.blob((24.3, 0.0, 9.0), (1.3, 1.1, 1.0), FUR);
            g.set(26, 0, 9, BLACK);
            g.set(23, -1, 11, BLACK);
            g.set(23, 1, 11, BLACK);
            g.ball((21.5, -2.0, 11.8), 0.8, FUR);
            g.ball((21.5, 2.0, 11.8), 0.8, FUR);
            g.ball((3.5, 0.0, 8.5), 1.0, FUR);
        }
        Model::finish(g, 12.0, 1.0, [[0.0, 0.0, 0.0]; 2])
    }

    /// A wrapped present: a box with a ribbon cross and a bow on top.
    pub fn gift(paper: Rgb, ribbon: Rgb) -> Model {
        let mut g = Grid::new();
        g.block((0, 8), (-4, 4), (0, 8), paper);
        g.block((3, 5), (-4, 4), (0, 8), ribbon);
        g.block((0, 8), (-1, 1), (0, 8), ribbon);
        g.blob((2.5, 0.0, 9.5), (1.6, 1.0, 1.2), ribbon);
        g.blob((5.5, 0.0, 9.5), (1.6, 1.0, 1.2), ribbon);
        Model::finish(g, 4.0, 1.0, [[0.0, 0.0, 0.0]; 2])
    }

    /// Model point to screen: (right, up, depth) in voxels.
    fn to_cam(p: [f32; 3], pose: &Pose) -> [f32; 3] {
        // roll about the forward axis
        let (cr, sr) = (pose.roll.cos(), pose.roll.sin());
        let (y, z) = (p[1] * cr - p[2] * sr, p[1] * sr + p[2] * cr);
        // nose-up pitch (hops)
        let (cn, sn) = (pose.nose.cos(), pose.nose.sin());
        let (x, z) = (p[0] * cn - z * sn, p[0] * sn + z * cn);
        // yaw: forward is (sin yaw) right and (cos yaw) away
        let (cy, sy) = (pose.yaw.cos(), pose.yaw.sin());
        let right = x * sy + y * cy;
        let away = x * cy - y * sy;
        // the camera looks down at `pitch`
        let (cp, sp) = (pose.pitch.cos(), pose.pitch.sin());
        [right, z * cp + away * sp, away * cp - z * sp]
    }

    /// Screen position of a model point, for effects that attach to it.
    pub fn point(&self, p: [f32; 3], pose: &Pose) -> (f32, f32) {
        let c = Model::to_cam([p[0] - self.pivot, p[1], p[2]], pose);
        (pose.sx + c[0] * pose.px, pose.sy - c[1] * pose.px)
    }

    pub fn draw(&self, cv: &mut Canvas, pose: &Pose) {
        let vp = pose.px;
        let half = (self.reach * vp).ceil() as i32 + 3;
        if pose.sx as i32 + half < 0 || pose.sx as i32 - half > cv.w as i32 || pose.sy as i32 - 2 * half > cv.h as i32 {
            return;
        }
        let (ox, oy) = (pose.sx as i32 - half, pose.sy as i32 - 2 * half);
        let (bw, bh) = ((2 * half) as usize, (2 * half + half / 2) as usize);
        let mut col: Vec<Option<Rgb>> = vec![None; bw * bh];
        let mut depth = vec![f32::INFINITY; bw * bh];
        // light from above, a little left of and behind the camera
        let light = {
            let l = [-0.45f32, 0.8, -0.4];
            let m = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
            [l[0] / m, l[1] / m, l[2] / m]
        };
        let splat = (vp + 0.35).ceil().max(1.0) as i32;
        for v in &self.voxels {
            let c = Model::to_cam(v.p, pose);
            let n = Model::to_cam(v.n, pose);
            if n[2] > 0.6 {
                // facing away; something nearer covers it
                continue;
            }
            let lit = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).max(0.0);
            let k = 0.72 + 0.42 * lit;
            let shade = |x: u8| ((x as f32 * k).min(255.0)) as u8;
            let mut rgb = (shade(v.c.0), shade(v.c.1), shade(v.c.2));
            if pose.fog > 0.0 {
                rgb = mix(rgb, pose.haze, pose.fog);
            }
            let sx = pose.sx + c[0] * vp - ox as f32;
            let sy = pose.sy - c[1] * vp - oy as f32;
            let (x0, y0) = ((sx - splat as f32 / 2.0).round() as i32, (sy - splat as f32 / 2.0).round() as i32);
            for yy in y0..y0 + splat {
                for xx in x0..x0 + splat {
                    if xx < 0 || yy < 0 || xx as usize >= bw || yy as usize >= bh {
                        continue;
                    }
                    let i = yy as usize * bw + xx as usize;
                    if c[2] < depth[i] {
                        depth[i] = c[2];
                        col[i] = Some(rgb);
                    }
                }
            }
        }
        // composite with a dark outline so it reads against the snow
        let outline = 0.7 * (1.0 - pose.fog);
        for y in 0..bh as i32 {
            for x in 0..bw as i32 {
                let i = y as usize * bw + x as usize;
                let at = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < bw && (y as usize) < bh && col[y as usize * bw + x as usize].is_some();
                match col[i] {
                    Some(c) => cv.put(ox + x, oy + y, c),
                    None if at(x - 1, y) || at(x + 1, y) || at(x, y - 1) || at(x, y + 1) => cv.blend(ox + x, oy + y, OUTLINE, outline),
                    None => {}
                }
            }
        }
    }
}

/// How the model sits on screen this frame.
#[derive(Clone, Copy)]
pub struct Pose {
    /// Screen point of the model's pivot on the ground.
    pub sx: f32,
    pub sy: f32,
    /// Pixels per voxel.
    pub px: f32,
    /// Nose angle relative to the line of sight, positive to the right.
    pub yaw: f32,
    /// Camera look-down angle.
    pub pitch: f32,
    /// Lean, positive rolls the top to the left.
    pub roll: f32,
    /// Nose up.
    pub nose: f32,
    /// Distance haze, 0..1, and its colour.
    pub fog: f32,
    pub haze: Rgb,
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOB: Elf = Elf { hat: (40, 168, 70), coat: (40, 168, 70), hair: (150, 74, 34), paint: RED };

    #[test]
    fn every_ride_has_a_shell_of_voxels() {
        for r in [Ride::Sleigh, Ride::Reindeer, Ride::Snowmobile, Ride::Toboggan, Ride::Skis, Ride::Goat, Ride::Broom, Ride::Plough] {
            let m = Model::build(r, &BOB, 0);
            assert!(m.voxels.len() > 250, "{r:?}: {}", m.voxels.len());
        }
    }

    #[test]
    fn a_reindeer_gallops_through_different_frames() {
        let f = Model::frames(Ride::Reindeer, &BOB);
        let legs = |m: &Model| m.voxels.iter().filter(|v| v.p[2] < 5.0).map(|v| (v.p[0] * 10.0) as i32).sum::<i32>();
        assert_ne!(legs(&f[0]), legs(&f[2]), "stretched and gathered differ");
    }

    #[test]
    fn turned_nose_shows_more_of_the_side() {
        let m = Model::build(Ride::Sleigh, &BOB, 0);
        let width = |yaw: f32| {
            let pose = Pose { sx: 0.0, sy: 0.0, px: 1.0, yaw, pitch: 0.4, roll: 0.0, nose: 0.0, fog: 0.0, haze: (0, 0, 0) };
            let xs: Vec<f32> = m.voxels.iter().map(|v| Model::to_cam(v.p, &pose)[0]).collect();
            xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min)
        };
        assert!(width(0.8) > width(0.0) * 1.4, "{} vs {}", width(0.8), width(0.0));
    }
}
