//! A track: a closed centreline, resampled one unit apart, and a lookup map
//! that tells every spot on the ground how far it is from the centreline
//! and which centreline sample is nearest (how far along the lap it is).
//!
//! World units are about half a metre. The map is MAP x MAP units.

pub const MAP: usize = 1024;
/// Beyond this distance from the centreline the snowbanks start.
const BANK_FROM: f32 = 40.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Surface {
    Road,
    Kerb,
    Powder,
    Bank,
    /// A slick patch on the road: hardly any grip.
    Ice,
}

/// A course's colours.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub road: (u8, u8, u8),
    pub kerb: [(u8, u8, u8); 2],
    pub powder: [(u8, u8, u8); 2],
    pub bank: [(u8, u8, u8); 2],
    /// The lake inside the loop: two tones and the colour of its specks.
    pub lake: [(u8, u8, u8); 3],
    /// Lake specks are cracks (thin lines) or flecks (dots).
    pub flecks: bool,
    pub ice: (u8, u8, u8),
    pub sky: [(u8, u8, u8); 2],
    pub haze: (u8, u8, u8),
    /// Far and near mountains.
    pub hills: [(u8, u8, u8); 2],
    pub aurora: bool,
}

pub const FROZEN: Theme = Theme {
    road: (168, 174, 200),
    kerb: [(206, 36, 52), (244, 242, 246)],
    powder: [(206, 218, 244), (238, 244, 255)],
    bank: [(176, 188, 222), (250, 252, 255)],
    lake: [(128, 186, 214), (170, 214, 232), (226, 240, 250)],
    flecks: false,
    ice: (176, 226, 246),
    sky: [(12, 16, 46), (150, 160, 214)],
    haze: (196, 206, 234),
    hills: [(124, 132, 190), (70, 80, 132)],
    aurora: true,
};

/// Eggnogg: a creamy lake flecked with nutmeg under a sunset.
pub const EGGNOGG: Theme = Theme {
    road: (186, 176, 188),
    kerb: [(226, 170, 52), (252, 242, 214)],
    powder: [(236, 226, 212), (252, 246, 234)],
    bank: [(214, 196, 178), (255, 250, 240)],
    lake: [(236, 208, 140), (248, 228, 176), (150, 92, 48)],
    flecks: true,
    ice: (200, 230, 240),
    sky: [(52, 34, 88), (252, 168, 120)],
    haze: (246, 212, 188),
    hills: [(196, 128, 150), (120, 70, 110)],
    aurora: false,
};

#[derive(Clone, Copy)]
struct Cell {
    dist: f32,
    idx: u32,
}

pub struct Tree {
    pub x: f32,
    pub y: f32,
    /// Size factor, about 0.8 to 1.3.
    pub size: f32,
}

pub struct Track {
    pub name: &'static str,
    /// Centreline samples, one unit apart, closed (the last joins the first).
    pub pts: Vec<(f32, f32)>,
    /// Unit direction of travel at each sample.
    tangents: Vec<(f32, f32)>,
    /// Road half-width.
    pub half: f32,
    cells: Vec<Cell>,
    pub trees: Vec<Tree>,
    pub theme: Theme,
    /// Ice patches on the road: centre and radius.
    pub ice: Vec<(f32, f32, f32)>,
}

/// The courses, in menu order.
pub fn courses() -> [(&'static str, fn() -> Track); 2] {
    [("Frozen Lake", Track::frozen_lake), ("Eggnogg", Track::eggnogg)]
}

/// Control points of the first course, Frozen Lake: start straight running
/// north up the west shore, a sweep along the north bank, an S down the east
/// side, a hairpin, an inner wiggle and back along the south shore.
const FROZEN_LAKE: &[(f32, f32)] = &[
    (170.0, 760.0),
    (168.0, 560.0),
    (175.0, 380.0),
    (210.0, 235.0),
    (330.0, 145.0),
    (520.0, 130.0),
    (700.0, 150.0),
    (855.0, 205.0),
    (905.0, 330.0),
    (845.0, 450.0),
    (900.0, 575.0),
    (860.0, 705.0),
    (765.0, 800.0),
    (650.0, 765.0),
    (600.0, 630.0),
    (480.0, 565.0),
    (385.0, 640.0),
    (335.0, 815.0),
    (255.0, 885.0),
    (185.0, 850.0),
];

/// Eggnogg: twistier, round a lake of eggnog, with a run of hairpins in the
/// east and ice in the bends.
const EGGNOGG_PTS: &[(f32, f32)] = &[
    (150.0, 520.0),
    (152.0, 320.0),
    (230.0, 175.0),
    (380.0, 150.0),
    (475.0, 250.0),
    (430.0, 380.0),
    (520.0, 465.0),
    (650.0, 425.0),
    (700.0, 290.0),
    (820.0, 180.0),
    (915.0, 265.0),
    (890.0, 420.0),
    (800.0, 525.0),
    (865.0, 655.0),
    (830.0, 825.0),
    (680.0, 880.0),
    (540.0, 800.0),
    (420.0, 865.0),
    (270.0, 875.0),
    (165.0, 765.0),
];

fn catmull(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32) -> (f32, f32) {
    let t2 = t * t;
    let t3 = t2 * t;
    let f = |a: f32, b: f32, c: f32, d: f32| 0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (-a + 3.0 * b - 3.0 * c + d) * t3);
    (f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
}

/// Closed Catmull-Rom spline through `ctrl`, resampled every `step` units.
fn resample(ctrl: &[(f32, f32)], step: f32) -> Vec<(f32, f32)> {
    let n = ctrl.len();
    let mut dense = Vec::new();
    for i in 0..n {
        let (p0, p1, p2, p3) = (ctrl[(i + n - 1) % n], ctrl[i], ctrl[(i + 1) % n], ctrl[(i + 2) % n]);
        for k in 0..200 {
            dense.push(catmull(p0, p1, p2, p3, k as f32 / 200.0));
        }
    }
    let mut out = vec![dense[0]];
    // distance still to go before the next sample
    let mut need = step;
    for i in 0..dense.len() {
        let a = dense[i];
        let b = dense[(i + 1) % dense.len()];
        let seg = (b.0 - a.0).hypot(b.1 - a.1);
        let mut pos = 0.0;
        while seg - pos >= need {
            pos += need;
            let t = pos / seg;
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
            need = step;
        }
        need -= seg - pos;
    }
    // the walk ends where it began; drop a last sample that sits on the first
    if let (Some(l), Some(f)) = (out.last(), out.first()) {
        if (l.0 - f.0).hypot(l.1 - f.1) < step * 0.5 {
            out.pop();
        }
    }
    out
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^ (h >> 13)
}

impl Track {
    pub fn frozen_lake() -> Track {
        let mut t = Track::build("Frozen Lake", FROZEN_LAKE, 14.0, FROZEN);
        t.add_ice(&[(0.36, -5.0, 6.0), (0.64, 4.0, 7.0)]);
        t
    }

    pub fn eggnogg() -> Track {
        let mut t = Track::build("Eggnogg", EGGNOGG_PTS, 13.0, EGGNOGG);
        t.add_ice(&[(0.17, 3.0, 6.0), (0.45, -4.0, 7.0), (0.58, 5.0, 6.0), (0.86, 0.0, 8.0)]);
        t
    }

    /// Ice patches at (fraction of the lap, offset right, radius).
    fn add_ice(&mut self, at: &[(f32, f32, f32)]) {
        for &(f, side, r) in at {
            let i = (self.len() as f32 * f) as usize;
            let (px, py) = self.pts[i];
            let (tx, ty) = self.tangents[i];
            self.ice.push((px - ty * side, py + tx * side, r));
        }
    }

    pub fn on_ice(&self, x: f32, y: f32) -> bool {
        self.ice.iter().any(|&(ix, iy, r)| (x - ix).abs() < r && (y - iy).abs() < r && (x - ix).hypot(y - iy) < r)
    }

    /// A plain ring of road for tests.
    #[cfg(test)]
    pub fn ring(half: f32) -> Track {
        let ctrl: Vec<(f32, f32)> = (0..24).map(|i| {
            let a = i as f32 / 24.0 * std::f32::consts::TAU;
            (512.0 + 380.0 * a.cos(), 512.0 + 380.0 * a.sin())
        }).collect();
        Track::build("Ring", &ctrl, half, FROZEN)
    }

    fn build(name: &'static str, ctrl: &[(f32, f32)], half: f32, theme: Theme) -> Track {
        let pts = resample(ctrl, 1.0);
        // how far around each sample the map is filled in
        let reach = BANK_FROM.max(half + 4.0) as i32 + 16;
        let mut cells = vec![Cell { dist: f32::INFINITY, idx: 0 }; MAP * MAP];
        for (i, &(px, py)) in pts.iter().enumerate() {
            let (cx, cy) = (px as i32, py as i32);
            for y in (cy - reach).max(0)..(cy + reach).min(MAP as i32) {
                for x in (cx - reach).max(0)..(cx + reach).min(MAP as i32) {
                    let d = (x as f32 + 0.5 - px).hypot(y as f32 + 0.5 - py);
                    let c = &mut cells[y as usize * MAP + x as usize];
                    if d < c.dist {
                        c.dist = d;
                        c.idx = i as u32;
                    }
                }
            }
        }
        let n = pts.len();
        let tangents = (0..n)
            .map(|i| {
                let (a, b) = (pts[(i + n - 2) % n], pts[(i + 2) % n]);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let l = dx.hypot(dy).max(1e-6);
                (dx / l, dy / l)
            })
            .collect();
        let mut t = Track { name, pts, tangents, half, cells, trees: Vec::new(), theme, ice: Vec::new() };
        // pines on a jittered grid, in the powder between kerb and bank
        let grid = 9;
        for gy in 0..(MAP as i32 / grid) {
            for gx in 0..(MAP as i32 / grid) {
                let h = hash(gx, gy);
                if h % 100 >= 55 {
                    continue;
                }
                let x = (gx * grid) as f32 + (h >> 8 & 0xff) as f32 / 255.0 * grid as f32;
                let y = (gy * grid) as f32 + (h >> 16 & 0xff) as f32 / 255.0 * grid as f32;
                let d = t.dist(x, y);
                if d > half + 9.0 && d < BANK_FROM - 3.0 {
                    t.trees.push(Tree { x, y, size: 0.8 + (h >> 24) as f32 / 255.0 * 0.5 });
                }
            }
        }
        t
    }

    pub fn len(&self) -> usize {
        self.pts.len()
    }

    fn cell(&self, x: f32, y: f32) -> Option<Cell> {
        if x < 0.0 || y < 0.0 || x >= MAP as f32 || y >= MAP as f32 {
            return None;
        }
        Some(self.cells[y as usize * MAP + x as usize])
    }

    pub fn dist(&self, x: f32, y: f32) -> f32 {
        self.cell(x, y).map_or(f32::INFINITY, |c| c.dist)
    }

    /// Nearest centreline sample, if the spot is anywhere near the course.
    pub fn idx(&self, x: f32, y: f32) -> Option<usize> {
        self.cell(x, y).filter(|c| c.dist.is_finite()).map(|c| c.idx as usize)
    }

    pub fn surface(&self, x: f32, y: f32) -> Surface {
        let d = self.dist(x, y);
        if d <= self.half {
            if self.on_ice(x, y) {
                Surface::Ice
            } else {
                Surface::Road
            }
        } else if d <= self.half + 2.5 {
            Surface::Kerb
        } else if d <= BANK_FROM.max(self.half + 4.0) {
            Surface::Powder
        } else {
            Surface::Bank
        }
    }

    /// Unit direction of travel at sample `i`.
    pub fn tangent(&self, i: usize) -> (f32, f32) {
        self.tangents[i]
    }

    /// For drawing: nearest sample, distance from the centreline and the
    /// signed sideways offset (positive to the right of travel).
    pub fn probe(&self, x: f32, y: f32) -> Option<(usize, f32, f32)> {
        let c = self.cell(x, y).filter(|c| c.dist.is_finite())?;
        let i = c.idx as usize;
        let (px, py) = self.pts[i];
        let (tx, ty) = self.tangents[i];
        Some((i, c.dist, (x - px) * -ty + (y - py) * tx))
    }

    pub fn bank_from(&self) -> f32 {
        BANK_FROM.max(self.half + 4.0)
    }

    pub fn heading_at(&self, i: usize) -> f32 {
        let (dx, dy) = self.tangent(i);
        dy.atan2(dx)
    }

    /// Sample `i` moved `k` samples along the lap (wrapping).
    pub fn ahead(&self, i: usize, k: isize) -> usize {
        let n = self.len() as isize;
        (((i as isize + k) % n + n) % n) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_are_one_unit_apart_and_closed() {
        for (_, make) in courses() {
        let t = make();
        assert!(t.len() > 2000, "lap is {} units", t.len());
        for i in 0..t.len() {
            let a = t.pts[i];
            let b = t.pts[(i + 1) % t.len()];
            let d = (b.0 - a.0).hypot(b.1 - a.1);
            assert!((0.5..1.5).contains(&d), "gap {d} at {i}");
        }
        }
    }

    #[test]
    fn centreline_is_road_and_stays_on_the_map() {
        for (name, make) in courses() {
        let t = make();
        for (i, &(x, y)) in t.pts.iter().enumerate() {
            assert!(matches!(t.surface(x, y), Surface::Road | Surface::Ice), "{name} sample {i}");
            // the course never runs close to itself (nearest sample is local)
            let near = t.idx(x, y).unwrap();
            let gap = (near as isize - i as isize).rem_euclid(t.len() as isize).min((i as isize - near as isize).rem_euclid(t.len() as isize));
            assert!(gap < 3, "{name}: sample {i} is nearer {near}");
            assert!(x > t.half + 3.0 && y > t.half + 3.0 && x < MAP as f32 - t.half - 3.0 && y < MAP as f32 - t.half - 3.0);
        }
        }
    }

    #[test]
    fn trees_stay_off_the_road() {
        let t = Track::frozen_lake();
        assert!(t.trees.len() > 200, "{} trees", t.trees.len());
        for tr in &t.trees {
            assert!(t.dist(tr.x, tr.y) > t.half + 8.0);
        }
    }
}
