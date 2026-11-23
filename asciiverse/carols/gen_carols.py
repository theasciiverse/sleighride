#!/usr/bin/env python3
"""Writes src/carols.rs: Sleighride's carols and winter pieces as melody data.

Each melody is read from a MIDI file of a public-domain score (the files
are in this folder, sources and licences in SOURCES.md): the highest note
at each moment of the upper part, skipping notes under a higher one still
held, rests under a quarter beat folded into the note before, lengths
snapped to quarter beats. For CPDL editions only the melody is taken (the
public-domain tune); the editor's harmony is not. The bass roots under
every tune are our own: per bar, the main chord of the key that best
fits that bar's melody (chord tones weighted by length, a bonus when the
bar opens on the chord's root, a little preference for I, IV and V).
The file's own timing places the bar lines (a leading rest kept); a file
that starts on a pickup without that rest gets one where the chords fit
clearly better.

  python3 asciiverse/carols/gen_carols.py > src/carols.rs
"""
import hashlib, os, sys, urllib.request
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from midi import read

# The CPDL editions are not committed (their licence covers the editors'
# editions, and only the public-domain melody is taken): fetched from CPDL
# when missing, pinned by SHA-256.
FETCH = {
    "c-jingle.mid": ("https://www.cpdl.org/wiki/images/c/cf/Jingle_Bells_v2_a4.mid",
                     "c25f49d554c5509c6983a3f49723cc93a11de6f02c8a581a5ca38de3f329789d"),
    "c-deck.mid": ("https://www.cpdl.org/wiki/images/9/99/Ws-deckhall.mid",
                   "90aeb1b90653b8f404c340a9b8d9dbd60bc1f65eebf96f9ecdd4ed6a3afe6e1b"),
    "c-godrest.mid": ("https://www.cpdl.org/wiki/images/1/13/Xmas-002.mid",
                      "1039f7f87528b49b43a2f6f2baa9b5caa9f35a4902e2aad9c82305e5df8a81b8"),
}


def source(name):
    path = os.path.join(HERE, name)
    if not os.path.exists(path) and name in FETCH:
        url, want = FETCH[name]
        data = urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"}), timeout=60).read()
        got = hashlib.sha256(data).hexdigest()
        if got != want:
            sys.exit(f"{name}: {url} has sha256 {got}, expected {want}")
        open(path, "wb").write(data)
    return path

MAJOR = [(0, [0, 4, 7], 0.3), (5, [5, 9, 0], 0.25), (7, [7, 11, 2], 0.25), (9, [9, 0, 4], 0.1), (2, [2, 5, 9], 0.05)]
MINOR = [(0, [0, 3, 7], 0.3), (5, [5, 8, 0], 0.2), (7, [7, 11, 2], 0.2), (3, [3, 7, 10], 0.15), (10, [10, 2, 5], 0.1)]

# file, upper tracks, tonic (pitch class), minor, beats per bar, bpm,
# race (else menus), christmas (else all year), name, source note
SONGS = [
    ("c-jingle.mid", [1], 5, False, 4, 168, True, True, "Jingle Bells",
     "James Lord Pierpont, 1857; melody from CPDL, Jingle Bells (James Pierpont), file Jingle_Bells_v2_a4.mid (CPDL licence; melody only)"),
    ("c-deck.mid", [2], 5, False, 4, 150, True, True, "Deck the Hall",
     "traditional Welsh; melody from CPDL, Deck the hall (Traditional), file Ws-deckhall.mid (CPDL licence; melody only)"),
    ("c-godrest.mid", [1], 4, True, 4, 156, True, True, "God Rest You Merry, Gentlemen",
     "traditional English; melody from CPDL, God rest you merry, gentlemen (Traditional), file Xmas-002.mid (CPDL licence; melody only)"),
    ("m905.mid", [1], 9, False, 4, 132, True, True, "Good King Wenceslas",
     "tune Tempus adest floridum (1582), words J. M. Neale 1853; Mutopia 905, public domain"),
    ("m1295.mid", [1], 10, False, 3, 84, False, True, "Stille Nacht",
     "Franz Xaver Gruber, 1818; Mutopia 1295, public domain"),
    ("m644.mid", [1], 5, False, 3, 104, False, True, "The Holly and the Ivy",
     "traditional English; Mutopia 644, public domain"),
    ("m1302.mid", [1], 0, False, 4, 92, False, True, "Vom Himmel hoch",
     "Martin Luther, 1539; Mutopia 1302, public domain"),
    ("m1806.mid", [1], 2, False, 2, 120, True, False, "March of the Wooden Soldiers",
     "P. I. Tchaikovsky, Op. 39 No. 5 (1878); Mutopia 1806, public domain"),
]


def melody(path, tracks_up):
    div, meta, tracks = read(path)
    ns = [x for ti in tracks_up for x in tracks[ti]['notes']]
    by = {}
    for s, d, n, ch in ns:
        if s not in by or n > by[s][1]:
            by[s] = (d, n)
    by = {s: v for s, v in by.items() if not any(s0 < s < s0 + d0 and n0 > v[1] for s0, d0, n0, _ in ns)}
    on = sorted(by.items())
    out, t = [], 0
    for i, (s, (d, n)) in enumerate(on):
        if s > t:
            out.append((0, (s - t) / div))
        nxt = on[i + 1][0] if i + 1 < len(on) else s + d
        dur = min(d, nxt - s)
        out.append((n, dur / div))
        t = s + dur
    tidy = []
    for n, d in out:
        if n == 0 and d < 0.25 and tidy:
            tidy[-1] = (tidy[-1][0], tidy[-1][1] + d)
        else:
            tidy.append((n, d))
    return [(n, max(0.25, round(d * 4) / 4)) for n, d in tidy]


def harmonise(mel, tonic, minor, bar, offset):
    """Roots per bar with the first bar line `offset` beats into the tune."""
    chords = MINOR if minor else MAJOR
    total = sum(d for _, d in mel)
    nbars = int((offset + total) // bar + (1 if (offset + total) % bar else 0))
    bars = [[] for _ in range(nbars)]
    t = offset
    for n, d in mel:
        if n:
            b = int(t // bar)
            if b < nbars:
                bars[b].append((n % 12, d, abs(t - b * bar) < 1e-6))
        t += d
    roots, score = [], 0.0
    for notes in bars:
        best = None
        for deg, tri, pref in chords:
            pcs = [(tonic + x) % 12 for x in tri]
            s = sum(d for pc, d, _ in notes if pc in pcs) + pref
            s += 0.5 if any(first and pc == pcs[0] for pc, d, first in notes) else 0
            if best is None or s > best[0]:
                best = (s, (tonic + deg) % 12)
        score += best[0]
        roots.append(best[1])
    # bass roots between E2 and Eb3, like the original tunes'
    return [40 + (pc - 4) % 12 for pc in roots], score


def place(mel, tonic, minor, bar):
    """Lead-in rest and roots. The file's own timing (its leading rest
    included) is trusted; a whole-beat shift is taken only when its chords
    fit clearly better (15%), for files that start on a pickup without
    the rest before it."""
    roots0, score0 = harmonise(mel, tonic, minor, bar, 0.0)
    best = (score0 * 1.15, 0.0, roots0)
    for k in range(1, int(bar)):
        lead = float(bar - k)
        roots, score = harmonise(mel, tonic, minor, bar, lead)
        if score > best[0]:
            best = (score, lead, roots)
    return best[1], best[2]


def octave(mel):
    """Moved by octaves so the melody sits around G4 to C5, as the others."""
    ns = sorted(n for n, _ in mel if n)
    mid = ns[len(ns) // 2]
    shift = 0
    while mid + shift < 64:
        shift += 12
    while mid + shift > 76:
        shift -= 12
    return [(n + shift if n else 0, d) for n, d in mel]


def main():
    print("//! Carols and winter pieces: public-domain melodies, arranged here.")
    print("//! Generated by asciiverse/carols/gen_carols.py from the MIDI files")
    print("//! beside it (sources and licences in asciiverse/carols/SOURCES.md);")
    print("//! regenerate rather than edit. Melodies as (midi note, beats), 0 a rest;")
    print("//! bass roots one per bar, our own harmony.")
    print()
    print("pub struct Carol {")
    print("    pub name: &'static str,")
    print("    pub source: &'static str,")
    print("    /// Beats (quarter notes) to a bar, and the tempo in those beats.")
    print("    pub bar: f32,")
    print("    pub bpm: f32,")
    print("    /// Played in the race (else on the menus).")
    print("    pub race: bool,")
    print("    /// Only in the Christmas season (else all year).")
    print("    pub christmas: bool,")
    print("    pub notes: &'static [(f32, f32)],")
    print("    pub roots: &'static [f32],")
    print("}")
    print()
    print(f"pub const CAROLS: [Carol; {len(SONGS)}] = [")
    for f, up, tonic, minor, bar, bpm, race, xmas, name, src in SONGS:
        mel = octave(melody(source(f), up))
        lead, roots = place(mel, tonic, minor, bar)
        if lead:
            mel = [(0, lead)] + mel
        # whole bars, so the bass stays in step when the tune comes round
        total = sum(d for _, d in mel)
        if total % bar:
            mel = mel + [(0, bar - total % bar)]
        assert len(roots) * bar == sum(d for _, d in mel), name
        notes = ", ".join(f"({n:.1f}, {float(d)!r})" for n, d in mel)
        print("    Carol {")
        print(f"        name: {name!r},".replace("'", '"'))
        print(f"        source: {src!r},".replace("'", '"'))
        print(f"        bar: {bar:.1f},")
        print(f"        bpm: {bpm:.1f},")
        print(f"        race: {str(race).lower()},")
        print(f"        christmas: {str(xmas).lower()},")
        print(f"        notes: &[{notes}],")
        print(f"        roots: &[{', '.join(f'{r:.1f}' for r in roots)}],")
        print("    },")
    print("];")


if __name__ == "__main__":
    main()
