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

The second day of Christmas (David, 2026-10-27: Santa's big day is done):
every present delivered, Santa asleep in his armchair by the workshop fire. All of Santa's helpers have taken everything with
runners for a joyride:
the big sleigh, the reindeer, the snowmobiles, toboggans and skis. Bob (the
elf in the big sleigh, the player) races the others. Santa is not happy: from
lap 2 he comes after them in the one thing they left behind, the
snowplough, and hunts whoever is leading: he has a list, he is checking
it twice, and whoever is out in front is clearly the ringleader.

The intro is one action-comic page (`story.rs`, captions in
`game::CAPTIONS`), shown once per run between the title and the racer
choice, laid out after David's sketch (2026-10-06): three panels side by
side, split by sweeping S-curved diagonal gutters. Left, Santa out cold in
his armchair with Zs drifting up (SNORE!); middle, a slanted band where the
road winds up the page between pines and everyone with runners charges up
it with speed lines (VROOOM!); right, Santa in the plough with headlights
and rays (HONK!!). Panels slam in 0.9 s apart ("bam-bam-bam"): zoom, white
flash, the page jolts; sound words pop in starbursts with an overshoot, in
clear space (over a gutter top, at the foot of the road, in the sky above
the plough) so they never cover a character, and sized to the panel.
Gutters are thin (about two columns of white, one of ink; David found the
first ones far too wide). SNORE! sits in Santa's room; a sound word that
would land on a caption drops below it, and the middle caption sits flush
against the far gutter (`game::caption_layout`, tested at six terminal
sizes).
Yellow captions: "SECOND DAY OF CHRISTMAS. SANTA'S OUT COLD." / "THE ELVES
FOUND THE KEYS!" / "SANTA'S AWAKE. THE LEADER GETS A CLIP ROUND THE EAR!".
About three seconds; Enter shows the page, then races.

When Santa catches the ringleader he gives them a good clip round the ear
(an oversized mitten swings in, dizzy stars), "YOU'RE ON SANTA'S NAUGHTY
LIST!", then rumbles on past for two seconds before leaving. His other
lines: "SANTA'S AWAKE!" on his first appearance, "SANTA'S BACK!" after,
and a verdict on the results card by place and catches (`santa_says`).

No copyrighted characters: generic winter-fantasy archetypes and public
domain folklore only (no Rudolph: our reindeer have black noses).

## Status

Playable (2026-10-05): two courses, six racers, items, Santa.

- Splash/title screen: SLEIGHRIDE and the tagline ("SECOND DAY OF CHRISTMAS.
  SANTA'S ASLEEP. THE ELVES HAVE FOUND THE KEYS." / "HE IS NOT GOING TO
  BE HAPPY.") all in the pixel lettering, stacked a sixth of the way down
  and wrapped onto more lines in narrow windows; difficulty (Up/Down); an
  autopilot race dimmed behind it all. The turning racer showcase came
  out (2026-10-06, David: "a little confusing"); the racer page still has
  its stage
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
- Courses (ten; `track::courses()`, one file each in `courses/`):
  **Thin Ice**, **Candy Cane Forest**, **Gingerbread Village**, **The
  Workshop**, **The Glacier**, **The Birch Wood**, **The Fjord** and **The
  Night Market** (see Next steps),
  **Frozen Lake** (aurora night, ice cracks, two ice patches; hills and two
  kickers, see below; the learner course, so no weather or wildlife since
  2026-10-06, David: "we have overloaded the first track") and
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
  gamepad; network races for up to six (`--host`, `--join [ID]`); session logging per the Asciiverse convention
- 47 tests, including a full six-racer race on each course (winner to last
  about 15 s on Frozen Lake, 11 s on Eggnogg; Santa visits twice), snowball,
  mistletoe and item-odds checks, Santa catching the leader, the results
  and title screens

## Decisions

- **Santa's catch is a scene (David, 2026-10-27):** he pulls up beside the
  one he caught, clips them, and rolls slowly past in the next lane with a
  speech bubble in the small pixel letters ("YOU'RE ON THE NAUGHTY LIST!",
  a rival's name first when it isn't you), then speeds off out of sight
  (`race::GLOAT`, `GLOAT_CRAWL`, `render::speech`). His rest is shortened
  by the longer show, so his visits come as often as before
- **Gingerbread Village balance (2026-10-27):** the reindeer averaged 2.5th
  and the skis 6.2nd there. Causes found with the autopilot, slice by slice:
  rivals swung out round the cottage and crossed the kicker's lip still
  moving sideways, so the low-grip rides slid off the road on landing; and
  a hairpin at 30% led straight into the climb. Rivals now fly over the
  cottage's roof when fast enough (`auto::COTTAGE_CLEAR`) and go round only
  when slow, planned from further back; the hairpin's control point moved
  from (300, 190) to (335, 200). Found on the way: a sleigh coming down on
  the roof just short of clearing it was trapped inside the cottage for up
  to 20 s; it now slides off to the nearest side (`track::out_of_cottage`)

- **The lens follows the window's height** (`View::new`: f = 0.68 h,
  capped at 0.62 w for tall, narrow windows), so a wider window shows more
  to the sides at the same size. It used to follow the width, and on a very
  wide screen everything zoomed in (David, 2026-10-06)
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
- **Licence:** GPL-3.0-or-later (David, 2026-10-27). No outside GPL code is
  in it; the choice was free.

## Layout

| File | What |
|---|---|
| `src/track.rs` | Track: closed Catmull-Rom centreline resampled every unit, a 1024x1024 lookup map (distance to centreline, nearest sample), surfaces, pines |
| `src/course.rs`, `courses/*.toml` | Courses as data: one TOML file per course (name, page lines, theme, points, hills, kickers, hazards, extras), parsed and checked once |
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
cargo run --release -- --race --course 2 --shot out.ppm 160 45 10i   # ... just short of an inlet
cargo run --release -- --race --course 4 --shot out.ppm 160 45 5t    # ... with a snow tornado ahead
cargo run --release -- --race --shot out.ppm 160 45 5p               # ... with penguins crossing ahead
cargo run --release -- --race --course 2 --shot out.ppm 160 45 5K    # ... the Yule Cat mid-pounce ahead
```

Use `RUST_LOG=sleighride=debug` rather than plain `debug`: gilrs is chatty.
The log has every key event, control changes, kart events (hop, tiers,
boosts, bumps, start result) and a snapshot every 500 ms.

## Controls

- Steer: Left/Right or A/D, easing in (a press starts at 20% and builds
  to full lock over about half a second, `input::ease_steer`; David found
  instant full lock too abrupt, then still too strong). Without the kitty
  protocol (GNOME Terminal, David's, has none) a lone press steers only
  110 ms (`STEER_TAP`) instead of the half second key repeat takes to
  start, which had him counter-steering all the time; gas: Up or W; brake/reverse: Down or S
- Drift: hold Space or Shift while steering
- Item: X, E or Ctrl; brake + item drops a present behind you
- Title screen: Up/Down sets the rivals. Then the racer page, then the
  course page: Left/Right picks the course (shown live behind the page,
  with its map and a line on what's on it), Up/Down the rivals, Enter
  races, Esc goes back
- P pause, R restart, Esc back, F1 or H (or ?) help, F8 sound on/off, M
  music on/off. The help page covers keys, drifting, items, Santa and the
  hazards. GNOME Terminal may take F1 for its own help; H always works
- Without the kitty keyboard protocol a terminal only repeats the last key
  held, so gas can't be held while steering: there Up latches the gas on,
  Down turns it off, and Space toggles a drift
- Pad: stick or d-pad steer, A gas, B brake, right shoulder drift, X/Y or
  left shoulder item. The stick has a response curve (`input::stick_curve`,
  `STICK_CURVE` 1.6 past the dead zone): gentle over the first half of its
  travel, full lock at the edge
- Drift turning (2026-10-27, David found it too quick on a gamepad):
  `kart::drift_share`, 0.55 of the ride's turn with the stick centred
  (was 0.72, as hard as full lock), up to 1.2 steered fully in and down to
  0.24 fully out, as before. The rivals steer for the arc they always took
  (`kart::drift_inward_for`), so the balance is unchanged; a straight cut
  to the whole drift handed every twisty course to the reindeer
  `SLEIGHRIDE_ANY_PAD=1` accepts pads missing from SDL's mapping database

## Roster

| Racer | Ride | Handling |
|---|---|---|
| Bob (player) | The big sleigh | balanced |
| Holly, Juniper | Reindeer | quick off the mark, nimble, light |
| Sprocket | Snowmobile | fast, heavy, wide line, slow drift charge |
| Pip | Toboggan | fast, light, quick drift charge |
| Nutmeg | Skis | highest top speed, twitchy, little grip |
| Tomte | Julbocken, the straw Yule goat | sturdy, steady |
| Befana | Her broomstick | flies low: powder and cream hardly slow her |
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
  (Italian witch on a broomstick: a flying racer), Jolakotturinn (the Icelandic Yule Cat: hazard), the Yule Lads
  (Iceland's thirteen pranksters: item thieves), Tomte/Nisse (racer), Ded
  Moroz and Snegurochka, Mari Lwyd. Possibly a world
  tour of courses
- More courses beyond Frozen Lake and Eggnogg; a track format instead of
  control points in code
- Hills for Eggnogg; more jumps; a "Slayride" name for Hard or a mode where
  Santa really means business
- **A road across the sky through the aurora** (David, 2026-11-03,
  "emphatically yes"; backlogged, not started): course 11, a road of
  shimmering aurora bands (green, violet) floating in the night sky on
  Christmas night, stars below and the earth far down under them. The
  reindeer fly, so it fits. Our own name, never "Rainbow Road" (Nintendo's).
  How it fits the existing code, checked 2026-11-03:
  - Nothing under the road: the land beyond the banks drops to a void floor
    far below (the course's `lake` height), and those pixels show sky. The
    one real renderer change: `terrain()` in `src/render.rs` first fills
    everything below the horizon with haze; for this course it fills with
    a starfield instead, and void cells are left unpainted
  - Driving off the edge: the crevasse code already does it (`kart.rs`,
    `in_crevasse`, hauled back along the road); the road's edge works the
    same way
  - The rainbow: bands in `ground_colour()`, across the road or along the
    lap, animated over time like `tier_colour()`
  - Hills, jumps and gaps between floating sections: `hills` and `kickers`
    in the course file as they are
  - Limit of the height map (one height per point): no road over itself,
    no loops or overhangs; banked turns are fine

## Next steps

David wants all of these (2026-10-06), roughly in this order:

- [x] Sound (2026-10-06, `audio.rs`, rodio, all synthesised): a band
      plays the rides live by speed (sleigh bells and runner hiss,
      galloping hoofbeats synced to the drawn stride, the snowmobile's
      two-stroke, skis swishing, Santa's diesel getting louder as he
      closes), the nearest three rivals by distance, and the music: two
      original tunes, "First Light" (gentle, pentatonic) on the menus and
      "Runners" (driving, E minor) in the race, faster on the final lap,
      a low warning bass while Santa is after you, soft hi-hats rather
      than sleigh bells. **The soundtrack stays wintry, not Christmassy**
      (David, 2026-10-06: the game is played all year and carols would
      get to be a bit much); the first version's carols were replaced.
      Santa's plough sounds like a tank: a low beating growl and
      clanking tracks that speed up with him. 28 effects:
      countdown, hop, spark tiers, boost, rocket start, stall, launch,
      landing crunch, trick, bumps, tree, hits, throws, burst, tinsel,
      peppermint, smooch, box ding, roulette tick, Santa's horn, the whack
      and boing of the clip, lap bell, finish. `--mute`, F8 sound, M music.
      **Needs David's ears:** levels and character were checked only by
      tests (every effect and voice audible, peaks in range), not listened
      to
- [x] Birds (2026-10-06, `birds.rs`, scenery only): crows on pines by
      the road flap off, climbing and veering away, when a racer comes
      within 24 units, and are back on their perch 22 s later; every 30 s
      a flock of eleven drifts across the sky ahead in a loose, well spread V for 16 s (spaced out 2026-10-06, David found it too tight).
      The flock flies in the camera's frame (far, high birds seem to keep
      pace), pale against Frozen Lake's night, dark against Eggnogg's
      sunset (`Theme::birds`)
- [x] Hills for Eggnogg (2026-10-06): up to a ridge in the north, a
      kicker off its edge down into the eggnog basin (lake at -20),
      rolling hairpins, then down off a hill to a kicker over a creamy
      inlet crossing the road (`Relief::inlets`, `Surface::Cream`: slow as
      powder, no sparks); fall short and you wade. Kickers are now ramps
      built on the road only (the snow beside stays at the land's height),
      painted as pale wooden planks with yellow-and-black chevrons at the
      lip; striped poles with pennants flag the inlet. Lessons: a hollow
      seen from a chase camera hides until you're on it, so the inlet is
      nearly flush and approached downhill. Slopes now change everyone's
      top speed by the same share (they used to favour quick
      accelerators); the snowmobile and toboggan got a little more turn,
      the snowmobile more accel and grip, so every ride still averages
      3rd to 4th on both courses
- [ ] Many more courses. The track format is done (2026-10-06): each
      course is a TOML file in `courses/` (`courses/README.md` lists every
      key), built into the binary and loaded by `course.rs`; themes stay in
      code, named from the file; the course page's two lines live in the
      file too. Unknown keys or themes are errors, and the usual tests
      (road clear of itself, every rival finishes, balance) run over every
      file. The seven files were generated from the old code and checked
      identical (points, heights, trees, ice, holes, poles, everything)
      before the code copies went. Courses so far:
      - [x] **Thin Ice** (2026-10-06, David's idea): a bright overcast
        morning (`MORNING` theme); round the hilly north and east shores
        with blue-and-white kerbs, off the bank onto the frozen lake with a
        kicker, and home across the ice (`Relief::on_lake`, flat at the
        lake's level, no kerbs, poles every 45 units, `Surface::BlackIce`
        at 35% grip). Eight holes in the ice (`Relief::holes`), always
        leaving a lane: drive in and it's SPLASH!, hauled out 10+ units
        back, all speed gone, 1.3 s shivering; rivals steer round them
      - [x] **Candy Cane Forest** (2026-10-06): lilac dusk (`CANDY`
        theme), a forest of giant striped candy canes in place of pines
        (`Theme::canes`, hooks turned either way), mint kerbs, pink-tinged
        snow, a frozen strawberry-milk lake with sprinkles; winding and
        hilly with a kicker off a big hilltop and two ice patches
      - [x] **Gingerbread Village** (2026-10-06): an amber evening
        (`GINGER` theme), gingerbread houses among the pines
        (`Theme::houses`: icing-trimmed roofs, candy buttons, flickering
        lit windows), pink-and-white icing kerbs, a frozen syrup pond.
        Off the hilltop kicker the road drops steeply into a dip with a
        **cottage standing in the road** (`Relief::cottages`, David's
        idea: "can we jump over some of the houses?"): fly over its roof
        at speed, or crash into its wall, or steer round it at the edges
        (the rivals go round). Yule Lads lurk by the boxes
      - [x] **The Workshop** (2026-10-06): indoors (`WORKSHOP` theme,
        `Theme::indoors`): dark rafters and swaying warm lamps instead of
        sky, wooden walls rising sheer 40 units out (`Relief::walls`),
        polished floorboards, red-and-green kerbs, wood shavings off the
        road, present stacks and nutcracker soldiers instead of pines. Three
        **conveyor belts** (`Surface::Conveyor`, `Relief::belts`): top speed
        x1.22, double acceleration, yellow chevrons running along. A ramp up
        to a 16-high mezzanine and a kicker off its edge. No birds or
        weather; Yule Lads by the boxes. **Spectators** (`crowd.rs`, David's
        idea: "spectators in the workshop that also throw packages"): five
        stands of five elves beside the road, cheering racers past; every
        ~6 s per stand one lobs a present at an oncoming racer, aimed at
        where it will be give or take 6 units, landing in 1.1 s with its
        shadow showing where; bursts like a thrown present
      - [x] **The Glacier** (2026-10-06): night under the northern lights
        (`GLACIER` theme), packed snow on blue ice, seracs (jagged ice
        spires, `Theme::seracs`) instead of pines, four ice patches,
        blizzards. A long climb to 32 high, then a fast run down over two
        **crevasses** (`Relief::crevasses`, `Track::in_crevasse`): a chasm
        14 deep right across the road and 8 past its edges, a kicker just
        before each and marker poles at both lips. At speed you sail over;
        drop below the lip (come at it slow) and it's "CREVASSE!", hauled
        out 100+ units back, before the kicker, with room to get up to
        speed again. From up the road the chasm hides behind the kicker
        (the road falls away), so the poles and the ramp's chevrons are the
        warning. Every rival finishes; balance holds across all seven
      - [x] **The Birch Wood** (2026-10-06): a pink and gold winter dusk
        (`BIRCH` theme) among clumps of bare birches (`Theme::birches`:
        white trunks, black marks, fine twigs), red-and-gold troika-road
        kerbs, hills and two kickers. **Ded Moroz** (Grandfather Frost)
        stands by the road in his long blue coat; every 22 s he raises his
        staff (the crystal flashes, 1.2 s warning) and strikes: frost
        spreads over the road just past him into an ice patch (radius 10)
        that holds 7 s and melts in 2 (`Track::frost`, synced from the
        race's folklore so network guests skate on it too). **Snegurochka**
        (the Snow Maiden) further on hands a present to anyone in the back
        half of the field who passes empty-handed ("A GIFT FROM
        SNEGUROCHKA!"), once per racer every 25 s: the Yule Lads' opposite
      - [x] **The Fjord** (2026-10-06): a cold, clear morning (`FJORD`
        theme: dark mountains, deep green-blue fjord ice, red-and-white
        kerbs). Up onto the clifftops, along them 30 high, then a ski jump
        (a 5-high kicker with a 34-sample run-up) off the cliff down to the
        frozen fjord, across the ice past four holes, and home. Data only
        apart from the theme: the first course made just by writing a file
      - [x] **The Night Market** (2026-10-06): a Christmas market by
        lamplight (`MARKET` theme: deep blue night, green-and-gold kerbs,
        `Theme::stalls`: wooden stalls with striped awnings, lit counters
        and twinkling strings of lights among the pines). A stall stands
        in the road just past a hilltop kicker to jump (as in Gingerbread
        Village), another on the flat to steer round, and three stands of
        market-goers throw presents, as in the Workshop but outdoors
      More course ideas welcome
- [x] Weather (2026-10-06, `weather.rs`): blizzards on Thin Ice (Frozen
      Lake had them too until it became the learner course): a 22 s calm at the start, then building 6 s, blowing
      16-26 s, easing 6 s, calm 40-70 s, repeat. While it blows: sight
      shrinks to 30%, everything fades to white, snow streaks across the
      view along the wind, gusts push everyone sideways (up to 5 u/s),
      the wind howls in the band, "BLIZZARD!" as one blows in. Snow
      tornadoes (one each on Thin Ice and Candy Cane Forest)
      drift back against the race at 7 u/s weaving across the road; touch
      one and you're thrown up spinning ("WHOOSH!"); rivals steer round
      them. Eggnogg stays calm
- [x] Wildlife (2026-10-06, `wildlife.rs`): a penguin crossing on Thin Ice
      (83%, out on the lake; Frozen Lake had one too until it became the
      learner course). Every 24 s
      (first at 14 s) seven penguins waddle across in single file from
      alternate sides; plough into one and it's bowled into a belly-slide
      away (nobody gets hurt) with a honk, and you lose 28% speed. A
      polar bear lives by each crossing: ambles about, trails the
      penguins when they cross but always keeps 4+ units off (never
      catches one), and rears up and swipes any racer within 8 units into
      a spin ("SWIPED!", a roar), then cools off 2.5 s. Voxel models: a
      waddling penguin, a walking (four frames) and a reared bear. Rivals
      steer round them. Scenery closer than 5 units to the camera is no
      longer drawn (it only looked huge and odd)
- [x] Network play (2026-10-06; `net.rs` and `lan.rs` from Panzer, which
      took them from Tetrix; `proto.rs`, `netplay.rs`): up to six players.
      `--host` hosts (the menus show who's in and the iroh id for internet
      guests); `--join` looks for races on the local network (UDP beacon on
      port 31459, Enter joins the first found); `--join ID` dials a host
      across the internet; `--name NAME`. The host runs the only race:
      guests send their controls every step and pick their own racer,
      the host picks course and difficulty and starts; the whole race
      (`race::Snap`, under 6 kB) goes back to every guest every step,
      events included, so sounds and effects match. A guest who leaves
      mid-race hands their racer back to the AI; joining mid-race is
      refused until it ends. No pausing a network race. Verified with two
      copies on one machine over the LAN path (found, joined, lobby on
      both, race started, guest's racer driven by the guest).
      **Needs David:** a race between two real machines, and one across
      the internet (relay path), haven't been tried
- [x] Folklore racers (2026-10-06): **Tomte on Julbocken** (the straw
      Yule goat, ribbon-bound, curling horns; a bearded Tomte riding;
      sturdy) and **Befana on her broomstick** (flying low: deep powder
      and cream barely slow her, `Stats::hover`, but low enough to dip
      into holes in the ice). Races now have eight racers; network races
      up to eight players. Balance measured again (each averages within
      1.6 places of the middle on all four courses).
      Found and fixed on the way: rivals fell into Thin Ice's holes ~20
      times a race, as their line only shifted far ahead; they now swerve
      from anything on their path just ahead (holes, penguins, the bear,
      tornadoes, dropped presents), looking further on slippery ice.
- [x] Folklore hazards (2026-10-06, `folklore.rs`): **the Yule Cat**
      (Jolakotturinn) on Eggnogg: a huge black cat with glowing eyes by
      the road; every ~12 s it crouches and wiggles for 1.6 s (the
      warning), then pounces low across the road, spinning out anyone
      under it (a yowl, "YULE CAT!"). **The Yule Lads** on Eggnogg and
      Candy Cane Forest: one lurks in the snow (hood and eyes showing) by
      each row of present boxes; pass within 6.5 units holding an item
      and he snatches it and runs off giggling with his sack ("YULE LAD
      STOLE YOUR ITEM!"), hiding 14 s. Rivals keep clear of a pouncing
      cat, and of the lads while holding an item.
      **Ruled out by David (2026-10-06): Krampus, and Tio de Nadal (the
      Catalan log).** Ded Moroz and Snegurochka are on the Birch
      Wood (2026-10-06). **The Mari Lwyd: ruled out by David (2026-10-06,
      "lovely but no, not for this game")**. No folklore ideas left open
- [x] The intro as an action-comic page (David, 2026-10-06): see the
      premise section
- [x] A course selection page (2026-10-06, David: "once we have a few
      tracks lined up ... a track selection page at the beginning";
      `Phase::Course`): after the racer page, "CHOOSE YOUR COURSE" over
      the course itself racing live behind, its name, two lines on its
      look and what's on it (`game::COURSE_NOTES`), its map, a dot per
      course, the rivals setting. In a network game the host chooses
      (guests wait on the racer page). `--courses` opens on it for shots

## History

Came up in conversation with Claude on 2026-06-30 as Bob-sleigh, parked
while at scout camp. Built to a first playable cut on 2026-10-05, then the
same day grew rivals, items, Santa, a second course and the Sleighride
premise.
