# Sleighride

Asciiverse kart racer. Rust + Ratatui, half-block pixel rendering with a
Braille minimap.

## Name

**Sleighride** (`game::TITLE` on the splash screen; folder, crate and
binary `sleighride`). Renamed from Bob-sleigh 2026-10-05; chosen by David
over "Joyride", which is crowded with racing titles (a 2022
Steam racer, Joyride: Lowpoly World, Kinect Joy Ride, a Rebellion board game,
Jetpack Joyride). No game uses "Sleighride" as one word, though there are
several "Santa's Sleigh Ride" titles and the song.

Repo: private, github.com/theasciiverse/sleighride (branch `main`).

## Premise

All of Santa's helpers have taken everything with runners for a joyride:
the big sleigh, the reindeer, the snowmobiles, toboggans and skis. Bob (the
elf in the big sleigh, the player) races the others. Santa is not happy: from
lap 2 he comes after them in the one thing they left behind, the
snowplough, and hunts whoever is leading.

No copyrighted characters: generic winter-fantasy archetypes and public
domain folklore only (no Rudolph: our reindeer have black noses).

## Status

Playable (2026-10-05): two courses, six racers, items, Santa.

- Splash/title screen: SLEIGHRIDE, each racer turning on a lit stage, a
  course picker (Left/Right), an autopilot race behind it all
- Courses: **Frozen Lake** (aurora night, ice cracks, two ice patches) and
  **Eggnogg** (sunset, a creamy eggnog lake flecked with nutmeg, gold kerbs,
  four ice patches, twistier)
- Racers (`roster.rs`): Bob (sleigh, player), Holly and Juniper (reindeer),
  Sprocket (snowmobile), Pip (toboggan), Nutmeg (skis), each with its own
  handling (top speed, accel, turn, grip, drift charge, weight)
- Rivals keep their own lines, drift, take rocket starts give or take their
  nerves, steer round dropped presents, use items sensibly, and close up a
  little when behind the player / ease off ahead
- Places 1st to 6th, standings, bumping by weight, minimap markers, results
  card with the whole field
- Present boxes (three rows of four) with odds that lean on place:
  snowballs (three, straight), present (lobbed, bursts; with brake: dropped
  behind as a trap), tinsel (homes in on the racer one place ahead),
  peppermint (boost), mistletoe (held out on a stick for 8 s: rivals who come
  close get kissed, stop dead with hearts, and you get a little boost)
- Santa: from the leader's lap 2, roars in 90 m behind the last racer at
  1.3x top speed, ignores pines, shoves others aside, catches the leader
  (two full spins), rests 22 s, comes again. Warnings: banner, distance bar,
  "SANTA'S AFTER YOU!" when it's you
- Handling: speed closing in on top speed, hop into a drift, three spark
  tiers (frost blue, gold, aurora) released as boosts, deep powder slows you
  and stops sparks, ice has a tenth of the grip, snowbanks and pines you
  glance off (a hard hit on a pine spins you out)
- Everyone is a voxel model drawn at its true angle every frame
- 3-2-1-GO countdown with a rocket start, three laps, lap times, wrong-way,
  final-lap banners, pause, help, restart
- Keyboard (exact holds under the kitty protocol, latched gas otherwise) and
  gamepad; session logging per the Asciiverse convention
- 17 tests, including a full six-racer race on each course (winner to last
  about 15 s on Frozen Lake, 11 s on Eggnogg; Santa visits twice), snowball,
  mistletoe and item-odds checks, Santa catching the leader, the results
  and title screens

## Decisions

- **Perspective: Mode 7 chase camera**, as in Mario Kart: Super Circuit
  (GBA) and Super Mario Kart (SNES). The track is a flat picture seen from a
  camera behind and above the sleigh; each screen row below the horizon is
  one distance, so it is one texture lookup per pixel. Pines and snow are
  scaled billboards. No 3D engine, and nothing from the Doom engine.
- **Rendering:** the scene is a half-block pixel canvas (two square-ish
  pixels per cell, 24-bit colour), not Braille: a textured ground in
  perspective needs a colour per pixel, which Braille's one colour per cell
  can't give. Braille is used where one colour is enough: the minimap.
- **Handling is our own**, tuned by feel. SRB2Kart (Kart Krew, GPL-2.0-only;
  source zip in `../../references/kart/`) is a reference for how a kart
  racer should *feel*, not a source: no code or tuning numbers are taken from
  it. It is itself a 3D game on the Doom Legacy engine, so its renderer is
  irrelevant here anyway.
- **Racers are voxel models, not sprite sheets.** Mario Kart draws each kart
  at a set of fixed angles; here the model is drawn at any angle, so turning,
  drifting and spinning show for free and each roster vehicle is one model in
  code. The model is drawn from a flatter angle than the real camera (55% of
  its pitch), the way kart sprites are, so it reads from behind rather than
  from above.
- **License:** not chosen yet, as for every other Asciiverse game; keeping
  GPL code out keeps the choice open.

## Layout

| File | What |
|---|---|
| `src/track.rs` | Track: closed Catmull-Rom centreline resampled every unit, a 1024x1024 lookup map (distance to centreline, nearest sample), surfaces, pines |
| `src/kart.rs` | Handling, drift tiers, start judging, collisions, being hit |
| `src/race.rs` | Six racers, countdown, laps by quarters in order, places, bumps, boxes, items, shots, traps, kisses, Santa |
| `src/auto.rs` | Rival AI (lines, drifts, items), Santa's hunting, attract mode |
| `src/roster.rs` | Who races: names, rides, colours, stats |
| `src/model.rs` | Voxel models built in code (sleigh, reindeer, snowmobile, toboggan, skis, plough with Santa, present), drawn with a depth buffer, lit from the camera side, outlined |
| `src/render.rs` | Mode 7 ground and sky per course theme, everything in the world sorted far to near, sparks, mistletoe, hearts, tinsel, 3x5 pixel font, Braille minimap |
| `src/game.rs` | Splash/title, HUD (item slot, place, standings, Santa bar), particles, results |
| `src/input.rs` | Keyboard and gamepad |

World units are about half a metre; the sim steps at 60 Hz. Frozen Lake is
2914 units a lap; a lap takes about 45 s. Courses are control points in
`track.rs` plus a `Theme`; `courses()` lists them for the menu.

## Running

```bash
cargo run --release                 # title screen
cargo run --release -- --race       # straight into a race
cargo run --release -- --race --course 2           # ... on Eggnogg
RUST_LOG=sleighride=debug cargo run --release 2>sleighride.log
cargo run --release -- --shot out.ppm 160 45 12    # screenshot of the title screen
cargo run --release -- --race --shot out.ppm 160 45 30   # a race, everyone on autopilot
cargo run --release -- --shot out.ppm 160 45 5d    # ... at the first gold-spark drift after 5 s
```

Use `RUST_LOG=sleighride=debug` rather than plain `debug`: gilrs is chatty.
The log has every key event, control changes, kart events (hop, tiers,
boosts, bumps, start result) and a snapshot every 500 ms.

## Controls

- Steer: Left/Right or A/D; gas: Up or W; brake/reverse: Down or S
- Drift: hold Space or Shift while steering
- Item: X, E or Ctrl; brake + item drops a present behind you
- Title screen: Left/Right picks the course
- P pause, R restart, Esc back, H help
- Without the kitty keyboard protocol a terminal only repeats the last key
  held, so gas can't be held while steering: there Up latches the gas on,
  Down turns it off, and Space toggles a drift
- Pad: stick or d-pad steer, A gas, B brake, right shoulder drift, X/Y or
  left shoulder item.
  `SLEIGHRIDE_ANY_PAD=1` accepts pads missing from SDL's mapping database

## Roster

| Racer | Ride | Handling |
|---|---|---|
| Bob (player) | The big sleigh | balanced |
| Holly, Juniper | Reindeer | quick off the mark, nimble, light |
| Sprocket | Snowmobile | fast, heavy, wide line, slow drift charge |
| Pip | Toboggan | fast, light, quick drift charge |
| Nutmeg | Skis | highest top speed, twitchy, little grip |
| Santa (chaser) | Snowplough | 1.3x anyone, unstoppable |

A new racer is a row in `roster.rs` and, for a new kind of ride, a model in
`model.rs`.

## Ideas backlog (from David, 2026-10-05)

- **Weather:** blizzards (shorter sight, driving snow, gusts that push you),
  snow tornadoes wandering the track and spinning out whoever they catch
- **Wildlife:** penguins waddling across the road in a line; a polar bear
  lumbering about and swiping, perhaps chasing the penguins (they'd never
  meet in the wild, Arctic vs Antarctic, which is the joke)
- **Christmas folklore from around the world** (public domain): Julbocken
  (the Swedish straw Yule goat: racer or giant course landmark), La Befana
  (Italian witch on a broomstick: a flying racer), Krampus (Alpine: another
  chaser), Jolakotturinn (the Icelandic Yule Cat: hazard), the Yule Lads
  (Iceland's thirteen pranksters: item thieves), Tomte/Nisse (racer), Ded
  Moroz and Snegurochka, Tio de Nadal (item), Mari Lwyd. Possibly a world
  tour of courses
- More courses beyond Frozen Lake and Eggnogg; a track format instead of
  control points in code

## Next steps

- [ ] Weather and wildlife hazards (above)
- [ ] Sound (rodio, synthesised, as in the sibling games)
- [ ] Battle mode: in or out for v1?

## History

Came up in conversation with Claude on 2026-06-30 as Bob-sleigh, parked
while at scout camp. Built to a first playable cut on 2026-10-05, then the
same day grew rivals, items, Santa, a second course and the Sleighride
premise.
