# Sleighride courses

One TOML file per course, built into the game (`src/course.rs` lists them in
order). Copy one to start a new course, add it to `FILES` in
`src/course.rs`, and `cargo test` checks it: unknown keys and unknown themes
are errors, the road may not come near itself, and every rival has to
finish three laps.

The map is 1024 by 1024 units (a unit is about half a metre). "Fraction"
means a fraction of the lap, from the start line. "Offset right" is across
the road, positive to the right of the direction of travel. Anything left
out is simply not on the course.

| Key | Meaning |
|---|---|
| `name` | The course's name |
| `look`, `about` | The course page's two lines: what it looks like, what's on it |
| `theme` | Colours: `frozen`, `eggnogg`, `morning`, `candy`, `ginger`, `workshop`, `glacier`, `birch`, `fjord` or `market` (themes live in `src/track.rs`) |
| `half` | Half the road's width (14 is usual) |
| `lake` | Height of the lake, or the floor beyond the banks |
| `walls` | `true` for tall walls instead of slopes down to a lake (indoors) |
| `points` | The centreline: `[x, y]` control points of a closed loop |
| `hills` | Road height along the lap: `[fraction, height]` |
| `kickers` | Jumps: `[fraction at the lip, rise, run-up in samples]` |
| `inlets` | Arms of the lake across the road: `[fraction, length]` |
| `on_lake` | Out on the frozen lake: `[from, to]` fractions |
| `holes` | Holes in the ice: `[fraction, offset right, radius]` |
| `cottages` | Cottages in the road to jump: `[fraction, size]` |
| `belts` | Conveyor belts: `[fraction, length]` |
| `crevasses` | Crevasses right across the road: `[fraction, length]`; put a kicker just before |
| `ice` | Ice patches: `[fraction, offset right, radius]` |
| `blizzards` | `true` for blizzards that come and go |
| `twisters` | How many snow tornadoes wander the road |
| `penguins` | Fraction where penguins cross (a polar bear lives there too) |
| `yule_cat` | Fraction where the Yule Cat sits |
| `yule_lads` | `true` for Yule Lads by the present boxes |
| `ded_moroz` | Fraction where Ded Moroz stands; every so often he lays frost (ice) across the road just past him |
| `snegurochka` | Fraction where Snegurochka stands; she gives a present to back markers passing empty-handed |
| `stands` | Spectator stands, as fractions |
