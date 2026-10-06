//! A track: a closed centreline, resampled one unit apart, and a lookup map
//! that tells every spot on the ground how far it is from the centreline
//! and which centreline sample is nearest (how far along the lap it is).
//!
//! World units are about half a metre. The map is MAP x MAP units.

use serde::{Deserialize, Serialize};

pub const MAP: usize = 1024;
/// Beyond this distance from the centreline the snowbanks start.
const BANK_FROM: f32 = 40.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Surface {
    Road,
    Kerb,
    Powder,
    Bank,
    /// A slick patch on the road: hardly any grip.
    Ice,
    /// A creamy inlet across the road: jump it, or wade through slowly.
    Cream,
    /// Out on the frozen lake: clear, slippery ice, road and all.
    BlackIce,
    /// A conveyor belt in the workshop: it carries you along, faster.
    Conveyor,
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
    /// The flock against this sky.
    pub birds: (u8, u8, u8),
    /// Giant candy canes stand where pines would.
    pub canes: bool,
    /// Gingerbread houses stand among the pines.
    pub houses: bool,
    /// Indoors: a ceiling of rafters and lamps instead of sky, toys
    /// instead of pines, no birds.
    pub indoors: bool,
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
    birds: (226, 232, 248),
    canes: false,
    houses: false,
    indoors: false,
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
    birds: (52, 30, 52),
    canes: false,
    houses: false,
    indoors: false,
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
    /// Ground height, MAP x MAP, one per unit cell.
    ground: Vec<f32>,
    pub lake: f32,
    pub trees: Vec<Tree>,
    pub theme: Theme,
    /// Ice patches on the road: centre and radius.
    pub ice: Vec<(f32, f32, f32)>,
    /// Inlets across the road: (first sample, length).
    pub inlets: Vec<(usize, usize)>,
    /// Kicker ramps on the road: (first sample, run).
    pub kicks: Vec<(usize, usize)>,
    /// Marker poles with pennants, either side of an inlet and along the
    /// road out on the lake.
    pub poles: Vec<(f32, f32)>,
    /// Stretches out on the frozen lake: (first sample, last sample).
    pub lake_spans: Vec<(usize, usize)>,
    /// Holes in the ice: centre and radius.
    pub holes: Vec<(f32, f32, f32)>,
    /// Cottages in the road: centre, size, and which way they face.
    pub cottages: Vec<(f32, f32, f32, f32)>,
    /// Conveyor belts along the road: (first sample, length).
    pub belts: Vec<(usize, usize)>,
    /// Stands of spectators beside the road, as fractions of the lap.
    pub stands: Vec<f32>,
    /// Weather: whether blizzards blow here, and how many snow tornadoes.
    pub blizzards: bool,
    pub twisters: usize,
    /// Where on the lap penguins cross the road (a polar bear lives there).
    pub penguins: Option<f32>,
    /// Where the Yule Cat sits, and whether Yule Lads lurk by the boxes.
    pub yule_cat: Option<f32>,
    pub yule_lads: bool,
    /// The relief it was built from (read by the tests).
    #[cfg_attr(not(test), allow(dead_code))]
    pub relief: &'static Relief,
}

/// A cottage's height and half-width per unit of size (a gingerbread
/// house is drawn ten units tall and nine wide at size 1).
pub const COTTAGE_TALL: f32 = 10.0;
pub const COTTAGE_HALF: f32 = 4.5;

/// The lie of the land: heights along the lap as (fraction of the lap,
/// height) keys eased between, kickers that throw you into the air, and the
/// level of the lake inside the loop. Heights are in world units.
pub struct Relief {
    pub keys: &'static [(f32, f32)],
    /// (fraction of the lap at the lip, rise, run-up in samples): the road
    /// ramps up by `rise` over the run-up and drops back at the lip.
    pub kickers: &'static [(f32, f32, usize)],
    /// (fraction of the lap where it starts, length in samples): inlets of
    /// the lake that cross the road, a little below it.
    pub inlets: &'static [(f32, usize)],
    /// (from, to) fractions of the lap where the course runs out on the
    /// frozen lake: flat at the lake's level, no kerbs, open ice around.
    pub on_lake: &'static [(f32, f32)],
    /// Holes in the ice: (fraction of the lap, offset right, radius).
    pub holes: &'static [(f32, f32, f32)],
    /// Little houses standing in the road, to jump over (or go round):
    /// (fraction of the lap, size).
    pub cottages: &'static [(f32, f32)],
    /// Conveyor belts along the road: (fraction of the lap, length).
    pub belts: &'static [(f32, usize)],
    /// Tall walls instead of slopes down to a lake (indoors).
    pub walls: bool,
    pub lake: f32,
}

#[cfg(test)]
const FLAT: Relief = Relief { keys: &[], kickers: &[], inlets: &[], on_lake: &[], holes: &[], cottages: &[], belts: &[], walls: false, lake: 0.0 };

/// Eggnogg: up from the start to a high ridge in the north, a kicker off
/// its edge down into the eggnog basin, rolling hills through the hairpins
/// in the east, and on the south shore a kicker over a creamy inlet that
/// crosses the road. Fall short and you wade.
const EGGNOGG_RELIEF: Relief = Relief {
    keys: &[
        (0.0, 0.0),
        (0.04, 0.0),
        (0.12, 18.0),
        (0.2, 18.0),
        (0.27, -6.0),
        (0.35, -2.0),
        (0.42, 6.0),
        (0.5, 2.0),
        (0.58, 8.0),
        (0.64, 4.0),
        // a hill, then down toward the inlet so you see it coming
        (0.705, 9.0),
        (0.775, -3.0),
        (0.81, -3.0),
        (0.9, 2.0),
    ],
    kickers: &[(0.2, 3.5, 26), (0.775, 4.5, 22)],
    inlets: &[(0.7765, 22)],
    on_lake: &[],
    holes: &[],
    cottages: &[],
    belts: &[],
    walls: false,
    lake: -20.0,
};

/// Thin Ice: round the hilly north and east shores, off the bank onto the
/// frozen lake with a kicker, and home across the ice past holes.
const THIN_ICE_RELIEF: Relief = Relief {
    keys: &[
        (0.0, 1.0),
        (0.1, 10.0),
        (0.18, 16.0),
        (0.27, 8.0),
        (0.36, 14.0),
        (0.44, 6.0),
        (0.52, 12.0),
        (0.61, 6.0),
        (0.69, 9.0),
        (0.755, 5.0),
        (0.775, 0.0),
        (0.935, 0.0),
        (0.97, 1.0),
    ],
    kickers: &[(0.765, 3.0, 22)],
    inlets: &[],
    on_lake: &[(0.775, 0.935)],
    holes: &[
        (0.795, -6.5, 3.5),
        (0.815, 6.0, 4.0),
        (0.835, 0.0, 3.0),
        (0.852, -7.0, 4.5),
        (0.87, 7.5, 4.0),
        (0.888, -2.5, 3.2),
        (0.905, 6.5, 3.8),
        (0.92, -6.0, 3.5),
    ],
    cottages: &[],
    belts: &[],
    walls: false,
    lake: 0.0,
};

const THIN_ICE_PTS: &[(f32, f32)] = &[
    (140.0, 720.0),
    (128.0, 470.0),
    (190.0, 270.0),
    (360.0, 155.0),
    (590.0, 135.0),
    (790.0, 190.0),
    (905.0, 360.0),
    (890.0, 590.0),
    (790.0, 760.0),
    (640.0, 800.0),
    (480.0, 720.0),
    (330.0, 650.0),
    (215.0, 770.0),
];

/// Candy Cane Forest: lilac dusk, mint kerbs, pink-tinged snow and a
/// frozen strawberry-milk lake with sprinkles.
pub const CANDY: Theme = Theme {
    road: (184, 176, 198),
    kerb: [(52, 186, 120), (246, 246, 250)],
    powder: [(232, 214, 234), (252, 244, 252)],
    bank: [(214, 190, 220), (255, 250, 255)],
    lake: [(232, 150, 184), (246, 192, 212), (255, 255, 255)],
    flecks: true,
    ice: (206, 236, 250),
    sky: [(74, 50, 120), (250, 190, 222)],
    haze: (246, 216, 234),
    hills: [(206, 156, 206), (150, 100, 162)],
    aurora: false,
    birds: (72, 40, 84),
    canes: true,
    houses: false,
    indoors: false,
};

/// Candy Cane Forest: winding through the canes, over a big hill with a
/// kicker on its top, round a frozen strawberry-milk lake.
const CANDY_RELIEF: Relief = Relief {
    keys: &[
        (0.0, 0.0),
        (0.07, 6.0),
        (0.13, 2.0),
        (0.19, 10.0),
        (0.25, 4.0),
        (0.31, 15.0),
        (0.37, 4.0),
        (0.43, 10.0),
        (0.49, 6.0),
        (0.55, 12.0),
        (0.61, 3.0),
        (0.67, 8.0),
        (0.74, 2.0),
        (0.81, 6.0),
        (0.87, -2.0),
        (0.94, 2.0),
    ],
    kickers: &[(0.31, 3.5, 24)],
    inlets: &[],
    on_lake: &[],
    holes: &[],
    cottages: &[],
    belts: &[],
    walls: false,
    lake: -14.0,
};

const CANDY_PTS: &[(f32, f32)] = &[
    (160.0, 820.0),
    (130.0, 600.0),
    (240.0, 470.0),
    (140.0, 330.0),
    (230.0, 170.0),
    (420.0, 140.0),
    (520.0, 290.0),
    (650.0, 160.0),
    (830.0, 170.0),
    (895.0, 330.0),
    (780.0, 460.0),
    (885.0, 610.0),
    (845.0, 810.0),
    (650.0, 870.0),
    (500.0, 760.0),
    (340.0, 885.0),
];

/// Gingerbread Village: an amber evening, icing kerbs, a frozen syrup pond.
pub const GINGER: Theme = Theme {
    road: (182, 168, 166),
    kerb: [(250, 250, 252), (232, 120, 152)],
    powder: [(232, 226, 236), (250, 246, 250)],
    bank: [(214, 204, 216), (255, 252, 255)],
    lake: [(196, 120, 50), (226, 162, 82), (255, 232, 182)],
    flecks: false,
    ice: (214, 234, 246),
    sky: [(30, 30, 82), (255, 170, 92)],
    haze: (250, 212, 176),
    hills: [(156, 112, 140), (94, 62, 92)],
    aurora: false,
    birds: (52, 34, 46),
    canes: false,
    houses: true,
    indoors: false,
};

/// Gingerbread Village: through the village and up over a hill with a
/// kicker on top, round the syrup pond.
const GINGER_RELIEF: Relief = Relief {
    keys: &[
        (0.0, 0.0),
        (0.08, 4.0),
        (0.16, 1.0),
        (0.26, 9.0),
        (0.33, 14.0),
        // a steep drop off the hilltop kicker, a cottage in the dip below:
        // fly over its roof
        (0.345, 7.0),
        (0.4, 4.0),
        (0.5, 7.0),
        (0.6, 2.0),
        (0.7, 8.0),
        (0.8, 3.0),
        (0.9, 5.0),
    ],
    kickers: &[(0.33, 3.0, 22)],
    inlets: &[],
    on_lake: &[],
    holes: &[],
    cottages: &[(0.3385, 0.42)],
    belts: &[],
    walls: false,
    lake: -10.0,
};

const GINGER_PTS: &[(f32, f32)] = &[
    (200.0, 860.0),
    (140.0, 660.0),
    (210.0, 500.0),
    (360.0, 470.0),
    (420.0, 330.0),
    (300.0, 190.0),
    (470.0, 120.0),
    (660.0, 150.0),
    (720.0, 300.0),
    (860.0, 260.0),
    (900.0, 450.0),
    (760.0, 560.0),
    (860.0, 720.0),
    (720.0, 880.0),
    (540.0, 800.0),
    (400.0, 890.0),
];

/// Santa's workshop, indoors: polished floorboards, painted kerbs, wood
/// shavings off the track, wooden walls, rafters and warm lamps above.
pub const WORKSHOP: Theme = Theme {
    road: (196, 152, 104),
    kerb: [(196, 36, 48), (40, 140, 70)],
    powder: [(206, 176, 128), (226, 200, 150)],
    bank: [(104, 66, 40), (140, 92, 56)],
    lake: [(70, 44, 28), (84, 54, 34), (60, 38, 24)],
    flecks: false,
    ice: (214, 234, 246),
    sky: [(40, 26, 18), (120, 80, 48)],
    haze: (150, 108, 70),
    hills: [(80, 52, 32), (60, 38, 24)],
    aurora: false,
    birds: (0, 0, 0),
    canes: false,
    houses: false,
    indoors: true,
};

/// The Workshop: along the floor between the toy stacks, onto conveyor
/// belts, up a ramp to the mezzanine and off its edge with a kicker.
const WORKSHOP_RELIEF: Relief = Relief {
    keys: &[(0.0, 0.0), (0.26, 0.0), (0.33, 16.0), (0.42, 16.0), (0.47, 0.0), (0.7, 0.0), (0.76, 3.0), (0.82, 0.0)],
    kickers: &[(0.42, 3.0, 22)],
    inlets: &[],
    on_lake: &[],
    holes: &[],
    cottages: &[],
    belts: &[(0.08, 150), (0.52, 180), (0.86, 160)],
    walls: true,
    lake: 0.0,
};

const WORKSHOP_PTS: &[(f32, f32)] = &[
    (180.0, 820.0),
    (160.0, 560.0),
    (170.0, 300.0),
    (300.0, 170.0),
    (520.0, 170.0),
    (560.0, 330.0),
    (440.0, 450.0),
    (560.0, 580.0),
    (760.0, 520.0),
    (800.0, 300.0),
    (900.0, 200.0),
    (910.0, 560.0),
    (860.0, 830.0),
    (620.0, 860.0),
    (400.0, 780.0),
];

/// A bright, overcast winter morning.
pub const MORNING: Theme = Theme {
    road: (172, 178, 198),
    kerb: [(44, 96, 206), (240, 244, 250)],
    powder: [(214, 224, 240), (244, 248, 255)],
    bank: [(186, 198, 224), (252, 252, 255)],
    lake: [(118, 168, 204), (156, 198, 224), (226, 240, 250)],
    flecks: false,
    ice: (176, 226, 246),
    sky: [(112, 150, 200), (226, 236, 246)],
    haze: (220, 230, 242),
    hills: [(150, 166, 200), (96, 112, 150)],
    aurora: false,
    birds: (44, 44, 64),
    canes: false,
    houses: false,
    indoors: false,
};

/// Frozen Lake: the start straight climbs a ridge along the west shore to a
/// kicker on the crest, rolls down the north bank, over the hills of the
/// east side, round the hairpin, then a second kicker throws you down to
/// the lake shore for the run home along the south.
const FROZEN_RELIEF: Relief = Relief {
    keys: &[
        (0.0, 0.0),
        (0.035, 0.0),
        (0.125, 26.0),
        (0.14, 21.0),
        (0.2, 10.0),
        (0.26, 10.0),
        (0.33, 22.0),
        (0.42, 4.0),
        (0.5, 15.0),
        (0.56, 8.0),
        (0.62, 12.0),
        (0.66, -6.0),
        (0.75, -9.0),
        (0.9, -6.0),
    ],
    kickers: &[(0.125, 3.5, 26), (0.62, 3.5, 26)],
    inlets: &[],
    on_lake: &[],
    holes: &[],
    cottages: &[],
    belts: &[],
    walls: false,
    lake: -26.0,
};

/// The courses, in menu order.
pub fn courses() -> [(&'static str, fn() -> Track); 6] {
    [
        ("Frozen Lake", Track::frozen_lake),
        ("Eggnogg", Track::eggnogg),
        ("Thin Ice", Track::thin_ice),
        ("Candy Cane Forest", Track::candy_cane_forest),
        ("Gingerbread Village", Track::gingerbread_village),
        ("The Workshop", Track::workshop),
    ]
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

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Height of the road at each of `n` samples round the lap, kickers
/// included.
#[cfg(test)]
fn road_profile(r: &Relief, n: usize) -> Vec<f32> {
    let (base, kick) = lie_of_land(r, n);
    base.iter().zip(&kick).map(|(a, b)| a + b).collect()
}

/// The land's height along the lap, and the kicker ramps built on the road
/// on top of it (the snow beside a ramp stays at the land's height).
fn lie_of_land(r: &Relief, n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut p = vec![0.0; n];
    if !r.keys.is_empty() {
        for (i, h) in p.iter_mut().enumerate() {
            let f = i as f32 / n as f32;
            // the keys before and after, wrapping round the lap
            let k = r.keys.iter().rposition(|k| k.0 <= f).unwrap_or(r.keys.len() - 1);
            let (a, b) = (r.keys[k], r.keys.get(k + 1).copied().unwrap_or((1.0 + r.keys[0].0, r.keys[0].1)));
            let span = (b.0 - a.0).max(1e-6);
            *h = a.1 + (b.1 - a.1) * smooth((f - a.0) / span);
        }
    }
    for &(f, len) in r.inlets {
        let s0 = (f * n as f32) as usize;
        for j in 0..len {
            // cream to the brim: just a touch below the land, so it shows
            // from far off as a flat strip rather than hiding in a hollow
            let k = (j as f32 + 0.5) / len as f32;
            p[(s0 + j) % n] -= 0.5 * (k * std::f32::consts::PI).sin();
        }
    }
    let mut kick = vec![0.0; n];
    for &(lip, rise, run) in r.kickers {
        let l = (lip * n as f32) as usize;
        for j in 0..run {
            let i = (l + n - run + j) % n;
            kick[i] += rise * (j + 1) as f32 / run as f32;
        }
    }
    (p, kick)
}

fn value_noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (smooth(x - xi as f32), smooth(y - yi as f32));
    let v = |a: i32, b: i32| (hash(a, b) & 0xffff) as f32 / 65535.0;
    let top = v(xi, yi) + (v(xi + 1, yi) - v(xi, yi)) * fx;
    let bot = v(xi, yi + 1) + (v(xi + 1, yi + 1) - v(xi, yi + 1)) * fx;
    top + (bot - top) * fy
}

/// Ground height for every cell: the road flat across at its profile
/// height, shoulders of powder rising gently to the snowbanks with soft
/// drifts, then down to the lake. Off the road it is smoothed so that where
/// two stretches of course meet at different heights the land between rolls
/// rather than steps.
fn ground_heights(cells: &[Cell], profile: &[f32], kick: &[f32], lake: f32, half: f32, on_lake: &[bool]) -> Vec<f32> {
    let bank = BANK_FROM.max(half + 4.0);
    let shoulder = |d: f32| (d - half - 2.5).max(0.0) * 0.1;
    let mut g = vec![lake; MAP * MAP];
    for y in 0..MAP {
        for x in 0..MAP {
            let c = cells[y * MAP + x];
            if !c.dist.is_finite() {
                continue;
            }
            let (d, base) = (c.dist, profile[c.idx as usize]);
            let edge = base + shoulder(bank);
            g[y * MAP + x] = if on_lake[c.idx as usize] {
                // out on the ice: flat all round
                base
            } else if d <= half + 2.5 {
                base + kick[c.idx as usize]
            } else if d <= bank {
                let drift = value_noise(x as f32 / 11.0, y as f32 / 11.0) - 0.5;
                base + shoulder(d) + drift * 2.4 * ((d - half - 2.5) / 10.0).min(1.0) * ((bank - d) / 6.0).min(1.0)
            } else if d <= bank + 6.0 {
                edge + 1.5 * (1.0 - ((d - bank - 3.0) / 3.0).abs())
            } else {
                edge + (lake - edge) * smooth((d - bank - 6.0) / 45.0)
            };
        }
    }
    // smooth off the road, twice
    for _ in 0..2 {
        let src = g.clone();
        for y in 2..MAP - 2 {
            for x in 2..MAP - 2 {
                if cells[y * MAP + x].dist <= half + 5.0 {
                    continue;
                }
                let mut sum = 0.0;
                for dy in 0..5 {
                    for dx in 0..5 {
                        sum += src[(y + dy - 2) * MAP + x + dx - 2];
                    }
                }
                g[y * MAP + x] = sum / 25.0;
            }
        }
    }
    g
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0xC2B2_AE3D);
    h ^ (h >> 13)
}

impl Track {
    pub fn frozen_lake() -> Track {
        // the first course, for learning: hills, jumps, the inlet and a
        // little ice, but no weather or wildlife (Thin Ice has those)
        let mut t = Track::build("Frozen Lake", FROZEN_LAKE, 14.0, FROZEN, &FROZEN_RELIEF);
        t.add_ice(&[(0.36, -5.0, 6.0), (0.64, 4.0, 7.0)]);
        t
    }

    pub fn eggnogg() -> Track {
        let mut t = Track::build("Eggnogg", EGGNOGG_PTS, 13.0, EGGNOGG, &EGGNOGG_RELIEF);
        t.yule_cat = Some(0.38);
        t.yule_lads = true;
        t.add_ice(&[(0.17, 3.0, 6.0), (0.45, -4.0, 7.0), (0.58, 5.0, 6.0), (0.86, 0.0, 8.0)]);
        t
    }

    pub fn candy_cane_forest() -> Track {
        let mut t = Track::build("Candy Cane Forest", CANDY_PTS, 14.0, CANDY, &CANDY_RELIEF);
        t.twisters = 1;
        t.yule_lads = true;
        t.add_ice(&[(0.22, 4.0, 6.0), (0.58, -4.0, 7.0)]);
        t
    }

    pub fn gingerbread_village() -> Track {
        let mut t = Track::build("Gingerbread Village", GINGER_PTS, 14.0, GINGER, &GINGER_RELIEF);
        t.add_ice(&[(0.12, -4.0, 6.0), (0.64, 3.0, 7.0)]);
        t.yule_lads = true;
        t
    }

    pub fn workshop() -> Track {
        let mut t = Track::build("The Workshop", WORKSHOP_PTS, 14.0, WORKSHOP, &WORKSHOP_RELIEF);
        t.yule_lads = true;
        t.stands = vec![0.18, 0.31, 0.63, 0.77, 0.95];
        // clear the toys out of the stands
        let spots = crate::crowd::spots(&t);
        t.trees.retain(|p| spots.iter().all(|s| (s.0 - p.x).hypot(s.1 - p.y) > 6.0));
        t
    }

    pub fn thin_ice() -> Track {
        let mut t = Track::build("Thin Ice", THIN_ICE_PTS, 14.0, MORNING, &THIN_ICE_RELIEF);
        t.blizzards = true;
        t.twisters = 1;
        t.penguins = Some(0.83);
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
        Track::build("Ring", &ctrl, half, FROZEN, &FLAT)
    }

    fn build(name: &'static str, ctrl: &[(f32, f32)], half: f32, theme: Theme, relief: &'static Relief) -> Track {
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
        let (land, kick) = lie_of_land(relief, n);
        let lake_spans: Vec<(usize, usize)> = relief.on_lake.iter().map(|&(a, b)| ((a * n as f32) as usize, (b * n as f32) as usize)).collect();
        let out_on_lake: Vec<bool> = (0..n).map(|i| lake_spans.iter().any(|&(a, b)| (i + n - a) % n <= (b + n - a) % n)).collect();
        let mut ground = ground_heights(&cells, &land, &kick, relief.lake, half, &out_on_lake);
        if relief.walls {
            // indoors: the floor stays level out to the walls, which rise
            // sheer beyond the bank
            let bank = BANK_FROM.max(half + 4.0);
            for (k, c) in cells.iter().enumerate() {
                let base = if c.dist.is_finite() { land[c.idx as usize] } else { 0.0 };
                ground[k] = if c.dist <= half + 2.5 {
                    ground[k]
                } else if c.dist <= bank {
                    base
                } else {
                    base + 16.0 * ((c.dist - bank) / 3.0).min(1.0)
                };
            }
        }
        let inlets = relief.inlets.iter().map(|&(f, len)| ((f * n as f32) as usize, len)).collect();
        let kicks = relief.kickers.iter().map(|&(lip, _, run)| (((lip * n as f32) as usize + n - run) % n, run)).collect();
        let mut t = Track { name, pts, tangents, half, cells, ground, lake: relief.lake, trees: Vec::new(), theme, ice: Vec::new(), inlets, kicks, poles: Vec::new(), lake_spans, holes: Vec::new(), cottages: Vec::new(), belts: relief.belts.iter().map(|&(f, len)| ((f * n as f32) as usize, len)).collect(), stands: Vec::new(), blizzards: false, twisters: 0, penguins: None, yule_cat: None, yule_lads: false, relief };
        for &(f, size) in relief.cottages {
            let i = (f * n as f32) as usize;
            let (px, py) = t.pts[i];
            t.cottages.push((px, py, size, t.heading_at(i)));
        }
        for &(f, side, r) in relief.holes {
            let i = (f * n as f32) as usize;
            let (px, py) = t.pts[i];
            let (tx, ty) = t.tangents[i];
            t.holes.push((px - ty * side, py + tx * side, r));
        }
        // marker poles along the road out on the lake
        let spans = t.lake_spans.clone();
        for (a, b) in spans {
            let mut i = a;
            while (i + n - a) % n <= (b + n - a) % n {
                let (px, py) = t.pts[i % n];
                let (tx, ty) = t.tangents[i % n];
                for side in [-(half + 3.0), half + 3.0] {
                    t.poles.push((px - ty * side, py + tx * side));
                }
                i = (i + 45) % n;
                if i < a && b > a {
                    break;
                }
            }
        }
        let inlets: Vec<(usize, usize)> = t.inlets.clone();
        for (s0, len) in inlets {
            for i in [(s0 + n - 3) % n, (s0 + len + 2) % n] {
                let (px, py) = t.pts[i];
                let (tx, ty) = t.tangents[i];
                for side in [-(half + 4.0), half + 4.0] {
                    t.poles.push((px - ty * side, py + tx * side));
                }
            }
        }
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
                let on_ice = t.idx(x, y).is_some_and(|i| t.out_on_lake(i));
                if d > half + 9.0 && d < BANK_FROM - 3.0 && !on_ice {
                    t.trees.push(Tree { x, y, size: 0.8 + (h >> 24) as f32 / 255.0 * 0.5 });
                }
            }
        }
        t
    }

    pub fn len(&self) -> usize {
        self.pts.len()
    }

    /// Ground height at a point, smoothly between cells.
    pub fn height(&self, x: f32, y: f32) -> f32 {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let at = |cx: f32, cy: f32| -> f32 {
            if cx < 0.0 || cy < 0.0 || cx >= MAP as f32 || cy >= MAP as f32 {
                self.lake
            } else {
                self.ground[cy as usize * MAP + cx as usize]
            }
        };
        let top = at(x0, y0) + (at(x0 + 1.0, y0) - at(x0, y0)) * tx;
        let bot = at(x0, y0 + 1.0) + (at(x0 + 1.0, y0 + 1.0) - at(x0, y0 + 1.0)) * tx;
        top + (bot - top) * ty
    }

    /// The slope of the ground along a heading: rise per unit run.
    pub fn slope(&self, x: f32, y: f32, heading: f32) -> f32 {
        let (c, s) = (heading.cos(), heading.sin());
        (self.height(x + c, y + s) - self.height(x - c, y - s)) / 2.0
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

    /// Whether sample `i` is on a kicker ramp, and how far up it (0..1).
    pub fn on_kick(&self, i: usize) -> Option<f32> {
        let n = self.len();
        self.kicks.iter().find_map(|&(s, run)| {
            let k = (i + n - s) % n;
            (k < run).then(|| k as f32 / run as f32)
        })
    }

    /// Whether sample `i` is out on the frozen lake.
    pub fn out_on_lake(&self, i: usize) -> bool {
        let n = self.len();
        self.lake_spans.iter().any(|&(a, b)| (i + n - a) % n <= (b + n - a) % n)
    }

    /// A cottage in the road whose walls take in (x, y) at height `z`:
    /// its centre, if so.
    pub fn cottage_at(&self, x: f32, y: f32, z: f32, reach: f32) -> Option<(f32, f32)> {
        self.cottages.iter().find_map(|&(cx, cy, size, _)| {
            let tall = COTTAGE_TALL * size;
            let r = COTTAGE_HALF * size + reach;
            ((x - cx).hypot(y - cy) < r && z < self.height(cx, cy) + tall).then_some((cx, cy))
        })
    }

    /// Whether a point is in a hole in the ice.
    pub fn in_hole(&self, x: f32, y: f32) -> bool {
        self.holes.iter().any(|&(hx, hy, r)| (x - hx).abs() < r && (y - hy).abs() < r && (x - hx).hypot(y - hy) < r)
    }

    /// Whether sample `i` is on a conveyor belt.
    pub fn on_belt(&self, i: usize) -> bool {
        let n = self.len();
        self.belts.iter().any(|&(s, len)| (i + n - s) % n < len)
    }

    /// Whether sample `i` is where an inlet crosses the road.
    pub fn in_inlet(&self, i: usize) -> bool {
        let n = self.len();
        self.inlets.iter().any(|&(s, len)| (i + n - s) % n < len)
    }

    pub fn surface(&self, x: f32, y: f32) -> Surface {
        let d = self.dist(x, y);
        if d <= BANK_FROM && !self.inlets.is_empty() && self.idx(x, y).is_some_and(|i| self.in_inlet(i)) {
            Surface::Cream
        } else if d <= BANK_FROM && !self.lake_spans.is_empty() && self.idx(x, y).is_some_and(|i| self.out_on_lake(i)) {
            Surface::BlackIce
        } else if d <= self.half && !self.belts.is_empty() && self.idx(x, y).is_some_and(|i| self.on_belt(i)) {
            Surface::Conveyor
        } else if d <= self.half {
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
            assert!(matches!(t.surface(x, y), Surface::Road | Surface::Ice | Surface::Cream | Surface::BlackIce | Surface::Conveyor), "{name} sample {i}");
            // the course never runs close to itself (nearest sample is local)
            let near = t.idx(x, y).unwrap();
            let gap = (near as isize - i as isize).rem_euclid(t.len() as isize).min((i as isize - near as isize).rem_euclid(t.len() as isize));
            assert!(gap < 3, "{name}: sample {i} is nearer {near}");
            assert!(x > t.half + 3.0 && y > t.half + 3.0 && x < MAP as f32 - t.half - 3.0 && y < MAP as f32 - t.half - 3.0);
        }
        }
    }

    #[test]
    fn the_road_climbs_smoothly_except_at_the_kicker_lips() {
        for (name, make) in courses() {
        let t = make();
        let n = t.len();
        let profile = road_profile(t.relief, n);
        let lips: Vec<usize> = t.relief.kickers.iter().map(|k| (k.0 * n as f32) as usize).collect();
        let (lo, hi) = profile.iter().fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
        assert!(hi - lo > 15.0, "{name} hills: {lo}..{hi}");
        for i in 0..n {
            let step = profile[(i + 1) % n] - profile[i];
            let at_lip = lips.iter().any(|&l| (i + 1) % n == l || i == l);
            // the inlet is a channel cut into the road on purpose
            let at_inlet = t.inlets.iter().any(|&(s0, len)| ((i + n + 2 - s0) % n) < len + 4);
            if !at_lip && !at_inlet {
                assert!(step.abs() < 0.3, "{name}: step {step} at {i}");
            }
            // the ground under the centreline is the road height (beside a
            // lip the lookup blends the top of the ramp with the drop)
            let (x, y) = t.pts[i];
            let near_lip = lips.iter().any(|&l| (i as isize - l as isize).abs() <= 2);
            assert!(near_lip || (t.height(x, y) - profile[i]).abs() < 0.6, "{name} at {i}: {} vs {}", t.height(x, y), profile[i]);
        }
        for &l in &lips {
            assert!(profile[l - 1] - profile[l] > 2.5, "{name}: a drop at the lip {l}");
        }
        }
    }

    #[test]
    fn thin_ice_runs_out_on_the_lake_with_holes_that_leave_a_way_through() {
        let t = Track::thin_ice();
        let n = t.len();
        let (a, b) = t.lake_spans[0];
        let mid = (a + b) / 2;
        let (x, y) = t.pts[mid];
        assert_eq!(t.surface(x, y), Surface::BlackIce);
        assert!((t.height(x, y) - t.lake).abs() < 0.5, "flat at the lake's level");
        assert_eq!(t.holes.len(), 8);
        // at every sample on the lake some lane across the road is clear
        for i in a..=b {
            let (px, py) = t.pts[i % n];
            let (tx, ty) = t.tangents[i % n];
            let clear = (-10..=10).any(|k| {
                let side = k as f32;
                let (qx, qy) = (px - ty * side, py + tx * side);
                (-2..=2).all(|w| !t.in_hole(qx - ty * w as f32, qy + tx * w as f32))
            });
            assert!(clear, "a way through at {i}");
        }
        // no trees out on the ice
        assert!(t.trees.iter().all(|tr| !t.idx(tr.x, tr.y).is_some_and(|i| t.out_on_lake(i))));
    }

    #[test]
    fn eggnoggs_inlet_crosses_the_road_just_past_a_kicker() {
        let t = Track::eggnogg();
        let n = t.len();
        let (s0, len) = t.inlets[0];
        let lip = (0.775 * n as f32) as usize;
        assert!(s0 > lip && s0 - lip < 6, "the cream starts just past the lip");
        let (x, y) = t.pts[(s0 + len / 2) % n];
        assert_eq!(t.surface(x, y), Surface::Cream);
        let (x, y) = t.pts[(s0 + len + 5) % n];
        assert_eq!(t.surface(x, y), Surface::Road);
    }

    #[test]
    fn trees_stay_off_the_road() {
        let t = Track::frozen_lake();
        assert!(t.trees.len() > 200, "{} trees", t.trees.len());
        for tr in &t.trees {
            assert!(t.dist(tr.x, tr.y) > t.half + 8.0);
        }
        let e = Track::eggnogg();
        assert_eq!(e.poles.len(), 4, "a pole each side, before and after the inlet");
        assert_eq!(e.kicks.len(), 2);
    }
}
