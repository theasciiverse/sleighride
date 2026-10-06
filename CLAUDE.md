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

Christmas morning: every present delivered, Santa asleep in his armchair
by the workshop fire. All of Santa's helpers have taken everything with
runners for a joyride:
the big sleigh, the reindeer, the snowmobiles, toboggans and skis. Bob (the
elf in the big sleigh, the player) races the others. Santa is not happy: from
lap 2 he comes after them in the one thing they left behind, the
snowplough, and hunts whoever is leading: he has a list, he is checking
it twice, and whoever is out in front is clearly the ringleader.

The story is four pages (`game::STORY`) shown once per run between the
title and the racer choice, each over its own still picture
(`story.rs`): Santa asleep by the workshop fire; Bob with the keys in the
garage; the empty garage at dawn with only the plough; the NAUGHTY list
being ticked, twice. The text fades in word by word, centred low.

When Santa catches the ringleader he gives them a good clip round the ear
(an oversized mitten swings in, dizzy stars), "YOU'RE ON SANTA'S NAUGHTY
LIST!", then rumbles on past for two seconds before leaving. His other
lines: "SANTA'S AWAKE!" on his first appearance, "SANTA'S BACK!" after,
and a verdict on the results card by place and catches (`santa_says`).

No copyrighted characters: generic winter-fantasy archetypes and public
domain folklore only (no Rudolph: our reindeer have black noses).

## Status

Playable (2026-10-05): two courses, six racers, items, Santa.

- Splash/title screen: SLEIGHRIDE, each racer turning on a lit stage, a
  course picker (Left/Right), difficulty (Up/Down), an autopilot race
  behind it all
- Difficulty sets the rivals' pace: Easy 93%, Normal 96% (default), Hard
  100%. The rivals drift perfectly, so full pace is a hard race for a
  person. Measured with the autopilot driving the player from the back:
  about 1st on Easy, about 2nd on Normal, 3rd to 4th on Hard
- Choose your racer (Enter on the title): any of the six, with speed,
  accel, handling and weight bars; the pick starts at the back and the rest,
  Bob included, race as rivals
- Animation, paced by distance travelled (`render::frame_of`, 4 frames per
  cycle built by `Model::frames`): reindeer gallop and bound, skiers skate
  with poles planting, snowmobile and plough tracks roll, hat bobbles swing
- Courses: **Frozen Lake** (aurora night, ice cracks, two ice patches; hills
  and two kickers, see below) and
  **Eggnogg** (sunset, a creamy eggnog lake flecked with nutmeg, gold kerbs,
  four ice patches, twistier)
- Racers (`roster.rs`): Bob (sleigh, player), Holly and Juniper (reindeer),
  Sprocket (snowmobile), Pip (toboggan), Nutmeg (skis), each with its own
  handling (top speed, accel, turn, grip, drift charge, weight)
- Rivals keep their own lines, drift, take rocket starts give or take their
  nerves, steer round dropped presents and pass slower racers, use items
  sensibly, and close up a little when behind the player (up to 4%) / ease
  off ahead (up to 3%); softened 2026-10-06 after a six-way photo finish,
  now the field finishes 2 to 6 s apart
- Places 1st to 6th, standings, minimap markers, results card with the
  whole field
- Bumps: racers that touch are pushed apart; only closing on each other
  costs speed, as a weighted impulse (the heavy shove the light). Rivals
  pull out to pass anyone just ahead on their line
- Balance (2026-10-05, after "why am I always the slowest?"): each ride's
  solo lap is within about 1% of the others, and over many autopilot races
  every racer averages 3rd to 4th; tests guard both. The cause of the
  complaint was a bump bug (every frame of contact cost 4% speed, so the
  player, starting at the back in traffic, got glued to the pack) plus a
  6% top-speed gap for the reindeer
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
- Hills and jumps (2026-10-06): a course has a `Relief` (height keys round
  the lap, kickers, lake level); `ground_heights` turns it into a height
  per cell (road flat across, shoulders rising to the banks, down to the
  lake, smoothed off the road). Frozen Lake climbs a ridge up the west
  shore (+26) to a kicker on the crest, rolls over the east side, and a
  second kicker after the hairpin drops you to the lake shore (-9); the
  lake lies at -26. Eggnogg is flat for now
- Physics: slopes pull on speed; where the ground falls away faster than
  gravity (50 u/s^2) can follow, you take off; no grip, gas or drift
  charge in the air; tap drift in the air for a trick (a barrel roll),
  land it for a boost, land mid-roll and stumble
- Rendering: the ground is drawn Comanche-style (voxel space): per column,
  front to back, each sample projected by its height, slopes lit by a low
  sun, a depth per pixel so pines, racers and items hide behind hills. The
  view tips with the road ahead (a horizon shear) so climbs and crests read;
  racers tilt with the slope and in flight
- Everyone is a voxel model drawn at its true angle every frame
- 3-2-1-GO countdown with a rocket start, three laps, lap times, wrong-way,
  final-lap banners, pause, help, restart
- Keyboard (exact holds under the kitty protocol, latched gas otherwise) and
  gamepad; session logging per the Asciiverse convention
- 27 tests, including a full six-racer race on each course (winner to last
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
cargo run --release -- --race --as Nutmeg          # ... as another racer
RUST_LOG=sleighride=debug cargo run --release 2>sleighride.log
cargo run --release -- --shot out.ppm 160 45 12    # screenshot of the title screen
cargo run --release -- --race --shot out.ppm 160 45 30   # a race, everyone on autopilot
cargo run --release -- --shot out.ppm 160 45 5d    # ... at the first gold-spark drift after 5 s
cargo run --release -- --race --shot out.ppm 160 45 5a   # ... at the first jump after 5 s
```

Use `RUST_LOG=sleighride=debug` rather than plain `debug`: gilrs is chatty.
The log has every key event, control changes, kart events (hop, tiers,
boosts, bumps, start result) and a snapshot every 500 ms.

## Controls

- Steer: Left/Right or A/D; gas: Up or W; brake/reverse: Down or S
- Drift: hold Space or Shift while steering
- Item: X, E or Ctrl; brake + item drops a present behind you
- Title screen: Left/Right picks the course
- P pause, R restart, Esc back, H help, F8 sound on/off, M music on/off
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
- Hills for Eggnogg; more jumps; a "Slayride" name for Hard or a mode where
  Santa really means business

## Next steps

David wants all of these (2026-10-06), roughly in this order:

- [x] Sound (2026-10-06, `audio.rs`, rodio, all synthesised): a band
      plays the rides live by speed (sleigh bells and runner hiss,
      galloping hoofbeats synced to the drawn stride, the snowmobile's
      two-stroke, skis swishing, Santa's diesel getting louder as he
      closes), the nearest three rivals by distance, and the music:
      Jingle Bells (Pierpont, 1857) on the menus, Deck the Halls
      (traditional) in the race, faster on the final lap, a low warning
      bass while Santa is after you. Leroy Anderson's "Sleigh Ride" (1948)
      is still under copyright and deliberately not used. 28 effects:
      countdown, hop, spark tiers, boost, rocket start, stall, launch,
      landing crunch, trick, bumps, tree, hits, throws, burst, tinsel,
      peppermint, smooch, box ding, roulette tick, Santa's horn, the whack
      and boing of the clip, lap bell, finish. `--mute`, F8 sound, M music.
      **Needs David's ears:** levels and character were checked only by
      tests (every effect and voice audible, peaks in range), not listened
      to
- [ ] Birds: crows lifting off the pines as you pass, a flock wheeling
      across the sky
- [ ] Hills for Eggnogg: a big drop into the eggnog basin, a jump over a
      creamy inlet
- [ ] Many more courses (and a track format instead of control points in
      code)
- [ ] Weather: blizzards (shorter sight, driving snow, gusts), snow
      tornadoes that wander and spin you out
- [ ] Wildlife: a penguin line crossing the road, a polar bear that
      swipes (and eyes the penguins)
- [ ] Network play: LAN and internet over iroh, as Panzer and Tetrix do
- [ ] Folklore racers and hazards (see the backlog)

## History

Came up in conversation with Claude on 2026-06-30 as Bob-sleigh, parked
while at scout camp. Built to a first playable cut on 2026-10-05, then the
same day grew rivals, items, Santa, a second course and the Sleighride
premise.
