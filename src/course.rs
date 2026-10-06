//! Courses as data: one TOML file per course in `courses/`, built into the
//! binary. A file gives the course's name and the two lines the course page
//! shows, its colour theme (by name; themes stay in `track.rs`), the road's
//! half width, the centreline's control points, the hills along the lap,
//! and whatever is on it: kickers, inlets, a run out on the lake, holes,
//! cottages, conveyor belts, crevasses, ice patches, weather, wildlife,
//! folklore and spectator stands. Anything left out is simply not there.
//!
//! Fractions are of the lap, from the start line. See any of the files for
//! the shape; `courses/README.md` lists every key.

use crate::track::{Relief, Theme, Track};
use serde::Deserialize;
use std::sync::OnceLock;

/// The courses, in the order the game numbers them.
const FILES: [&str; 10] = [
    include_str!("../courses/1-frozen-lake.toml"),
    include_str!("../courses/2-eggnogg.toml"),
    include_str!("../courses/3-thin-ice.toml"),
    include_str!("../courses/4-candy-cane-forest.toml"),
    include_str!("../courses/5-gingerbread-village.toml"),
    include_str!("../courses/6-the-workshop.toml"),
    include_str!("../courses/7-the-glacier.toml"),
    include_str!("../courses/8-the-birch-wood.toml"),
    include_str!("../courses/9-the-fjord.toml"),
    include_str!("../courses/10-the-night-market.toml"),
];

#[derive(Deserialize, Default, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct CourseFile {
    pub name: String,
    /// The course page's two lines: what it looks like, what's on it.
    pub look: String,
    pub about: String,
    pub theme: String,
    pub half: f64,
    /// Height of the lake (or the floor beyond the banks).
    pub lake: f64,
    /// Tall walls instead of slopes down to a lake (indoors).
    pub walls: bool,
    pub points: Vec<[f64; 2]>,
    pub hills: Vec<[f64; 2]>,
    pub kickers: Vec<(f64, f64, usize)>,
    pub inlets: Vec<(f64, usize)>,
    pub on_lake: Vec<[f64; 2]>,
    pub holes: Vec<[f64; 3]>,
    pub cottages: Vec<[f64; 2]>,
    pub belts: Vec<(f64, usize)>,
    pub crevasses: Vec<(f64, usize)>,
    pub ice: Vec<[f64; 3]>,
    pub blizzards: bool,
    pub twisters: usize,
    pub penguins: Option<f64>,
    pub yule_cat: Option<f64>,
    pub yule_lads: bool,
    pub ded_moroz: Option<f64>,
    pub snegurochka: Option<f64>,
    pub stands: Vec<f64>,
}

/// A course ready to build tracks from.
pub struct Course {
    pub name: &'static str,
    pub look: &'static str,
    pub about: &'static str,
    file: CourseFile,
    theme: Theme,
    points: Vec<(f32, f32)>,
    relief: &'static Relief,
}

fn leak<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

fn leak_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// Parse and check one course file.
pub fn parse(text: &str) -> Result<Course, String> {
    let f: CourseFile = toml::from_str(text).map_err(|e| e.to_string())?;
    let theme = crate::track::theme_named(&f.theme).ok_or_else(|| format!("{}: no theme called {:?}", f.name, f.theme))?;
    if f.name.is_empty() {
        return Err("a course needs a name".into());
    }
    if f.points.len() < 4 {
        return Err(format!("{}: a course needs at least four points", f.name));
    }
    if f.half <= 4.0 {
        return Err(format!("{}: half (the road's half width) must be more than 4", f.name));
    }
    let x = |v: f64| v as f32;
    let relief = Relief {
        keys: leak(f.hills.iter().map(|k| (x(k[0]), x(k[1]))).collect()),
        kickers: leak(f.kickers.iter().map(|k| (x(k.0), x(k.1), k.2)).collect()),
        inlets: leak(f.inlets.iter().map(|k| (x(k.0), k.1)).collect()),
        on_lake: leak(f.on_lake.iter().map(|k| (x(k[0]), x(k[1]))).collect()),
        holes: leak(f.holes.iter().map(|k| (x(k[0]), x(k[1]), x(k[2]))).collect()),
        cottages: leak(f.cottages.iter().map(|k| (x(k[0]), x(k[1]))).collect()),
        belts: leak(f.belts.iter().map(|k| (x(k.0), k.1)).collect()),
        walls: f.walls,
        crevasses: leak(f.crevasses.iter().map(|k| (x(k.0), k.1)).collect()),
        lake: x(f.lake),
    };
    Ok(Course {
        name: leak_str(&f.name),
        look: leak_str(&f.look),
        about: leak_str(&f.about),
        theme,
        points: f.points.iter().map(|p| (x(p[0]), x(p[1]))).collect(),
        relief: Box::leak(Box::new(relief)),
        file: f,
    })
}

impl Course {
    pub fn build(&self) -> Track {
        let f = &self.file;
        let x = |v: f64| v as f32;
        let mut t = Track::build(self.name, &self.points, x(f.half), self.theme, self.relief);
        let ice: Vec<(f32, f32, f32)> = f.ice.iter().map(|k| (x(k[0]), x(k[1]), x(k[2]))).collect();
        t.add_ice(&ice);
        t.blizzards = f.blizzards;
        t.twisters = f.twisters;
        t.penguins = f.penguins.map(x);
        t.yule_cat = f.yule_cat.map(x);
        t.yule_lads = f.yule_lads;
        t.ded_moroz = f.ded_moroz.map(x);
        t.snegurochka = f.snegurochka.map(x);
        if !f.stands.is_empty() {
            t.stands = f.stands.iter().map(|&s| x(s)).collect();
            // clear the props out of the stands
            let spots = crate::crowd::spots(&t);
            t.trees.retain(|p| spots.iter().all(|s| (s.0 - p.x).hypot(s.1 - p.y) > 6.0));
        }
        t
    }
}

/// Every course, parsed once.
pub fn all() -> &'static [Course] {
    static ALL: OnceLock<Vec<Course>> = OnceLock::new();
    ALL.get_or_init(|| FILES.iter().map(|text| parse(text).unwrap_or_else(|e| panic!("bad course file: {e}"))).collect())
}

/// The course called `name`.
#[cfg(test)]
pub fn named(name: &str) -> &'static Course {
    all().iter().find(|c| c.name == name).unwrap_or_else(|| panic!("no course called {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_course_file_parses() {
        assert_eq!(all().len(), FILES.len());
        for c in all() {
            assert!(!c.look.is_empty() && !c.about.is_empty(), "{}: the course page's lines", c.name);
        }
    }

    #[test]
    fn mistakes_in_a_course_file_are_caught() {
        let ok = "name = \"T\"\ntheme = \"frozen\"\nhalf = 14.0\npoints = [[100.0, 100.0], [900.0, 100.0], [900.0, 900.0], [100.0, 900.0]]\n";
        assert!(parse(ok).is_ok());
        assert!(parse(&ok.replace("frozen", "lava")).err().unwrap().contains("no theme"));
        assert!(parse(&format!("{ok}hils = []\n")).is_err(), "a misspelt key");
        assert!(parse(&ok.replace("half = 14.0", "half = 2.0")).is_err());
    }
}
