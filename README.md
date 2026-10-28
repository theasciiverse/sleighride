# Sleighride

**SECOND DAY OF CHRISTMAS. SANTA'S ASLEEP. THE ELVES HAVE FOUND THE KEYS.**

Santa is snoring by the workshop fire, and his helpers have taken everything
with runners for a joyride: the big sleigh, the reindeer, the snowmobile, a
toboggan, racing skis, the straw Yule goat and Befana's broomstick. Race them
round ten courses of ice, eggnog, candy canes and gingerbread, drift until
your sparks burn aurora, fly off the kickers, and mind the penguins. From lap
two Santa comes after the leader in the snowplough, and he is not happy.
Can you win without getting caught?

## Play

With [av](https://asciiverse.sh), the Asciiverse client:

```
av install sleighride
av play sleighride
```

Or download the archive for your system from this repository's Releases,
unpack it and run `./sleighride`.

To build it yourself: `cargo run --release`.

Up to eight players on the local network: one runs `--host`, the others
`--join`.

## Keys

| Key | |
|---|---|
| Left / Right, A / D | steer |
| Up, W | gas |
| Down, S | brake, reverse |
| Space or Shift, held while steering | drift |
| X, E or Ctrl | use your item; with brake held, drop a present behind you |
| P / R / Esc | pause / restart / back |
| F1 or H | help |
| F8 / M | sound / music on or off |

Gamepad: stick or d-pad steers, A gas, B brake, right shoulder drift, X, Y
or left shoulder item. Pads missing from SDL's mapping database work with
`SLEIGHRIDE_ANY_PAD=1`. On terminals without the kitty keyboard protocol, Up
latches the gas on, Down turns it off, and Space toggles a drift.

## Options

`sleighride --help` lists them: `--race`, `--course N`, `--as NAME`, `--mute`,
`--host`, `--join [ID]`, `--name NAME`.

Session log for bug reports: `RUST_LOG=sleighride=debug sleighride 2>sleighride.log`

## Credits

The folklore (Julbocken, La Befana, the Yule Cat, the Yule Lads, Ded Moroz
and Snegurochka) is public-domain Christmas tradition. Networking uses
[iroh](https://iroh.computer).

## Licence

GNU General Public License, version 3 or (at your option) any later
version; see [LICENSE](LICENSE).
