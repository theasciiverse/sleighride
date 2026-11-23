# Sleighride's carols and winter pieces: sources

Every tune here is a public-domain composition, arranged for Sleighride.
From each source below **only the melody is taken**: the highest note at
each moment of the upper part, read from the MIDI file beside this note
by `gen_carols.py`. The bass roots, tempi and everything else are our
own. No recording is used; the game synthesises the music itself.

Each melody was checked note for note against the tune as it is sung or
played (2026-11-23). Licences were read on each source's page that day.

The carols play from 1 December to 6 January (`--christmas always` or
`never` overrides that); the winter piece plays all year (David,
2026-11-23).

| Tune | Composer, date | Source | File here | Licence of the source file |
|---|---|---|---|---|
| Jingle Bells | James Lord Pierpont, 1857 | CPDL, https://www.cpdl.org/wiki/index.php/Jingle_Bells_(James_Pierpont), file https://www.cpdl.org/wiki/images/c/cf/Jingle_Bells_v2_a4.mid | `c-jingle.mid` (fetched) | CPDL licence (the edition); the melody itself is public domain |
| Deck the Hall | traditional Welsh | CPDL, https://www.cpdl.org/wiki/index.php/Deck_the_hall_(Traditional), file https://www.cpdl.org/wiki/images/9/99/Ws-deckhall.mid | `c-deck.mid` (fetched) | CPDL licence (the edition); the melody itself is public domain |
| God Rest You Merry, Gentlemen | traditional English | CPDL, https://www.cpdl.org/wiki/index.php/God_rest_you_merry,_gentlemen_(Traditional), file https://www.cpdl.org/wiki/images/1/13/Xmas-002.mid | `c-godrest.mid` (fetched) | CPDL licence (the edition); the melody itself is public domain |
| Good King Wenceslas | tune *Tempus adest floridum* (1582), words J. M. Neale (1853) | Mutopia 905, https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=905 | `m905.mid` | Public Domain |
| Stille Nacht | Franz Xaver Gruber, 1818 | Mutopia 1295, https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=1295 | `m1295.mid` | Public Domain |
| The Holly and the Ivy | traditional English | Mutopia 644, https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=644 | `m644.mid` | Public Domain |
| Vom Himmel hoch | Martin Luther, 1539 | Mutopia 1302, https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=1302 | `m1302.mid` | Public Domain |
| March of the Wooden Soldiers (all year) | P. I. Tchaikovsky, Op. 39 No. 5, 1878 | Mutopia 1806, https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=1806 | `m1806.mid` | Public Domain |

The CPDL licence (https://www.cpdl.org/wiki/index.php/ChoralWiki:CPDL)
is a copyleft licence for an editor's edition: copy, perform and modify,
with works based on the edition under the same terms. What an editor adds
(harmony, voicing, editing) is theirs; the centuries-old melody is not.
Only the melody is taken from these three, so nothing of the editions'
own expression is in Sleighride, and their files are not committed:
`gen_carols.py` fetches them from CPDL when regenerating, pinned by
SHA-256 (`.gitignore` keeps them out). The five Mutopia files are public
domain and kept here unchanged. Decision recorded in
`katalystnord/policy-on-training-and-extraction.md`, 2026-11-23.

Looked at and not used: Mutopia 521 (Stille Nacht, CC BY-SA 2.0; a
public-domain edition was there), Mutopia 1240 (Es ist ein Ros'
entsprungen: the extracted line had stray notes, so it was left out
rather than guessed at), Mutopia 856 and 198 (a Polish carol of 1586 and
a French noel, little known). Not found in any free source checked:
"Carol of the Bells" (only Leontovych's *Shchedryk* melody would be free;
the 1936 English arrangement is not), The Skaters' Waltz, Trepak. "Sleigh
Ride" (Leroy Anderson, 1948) is under copyright and must not be used.

## Regenerating

    python3 asciiverse/carols/gen_carols.py > src/carols.rs

`midi.py` reads the MIDI files (no libraries needed).
