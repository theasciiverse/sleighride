//! All sound is synthesised here; Sleighride ships no audio files.
//!
//! - Effects are rendered once at start-up into sample buffers.
//! - One endless source, the band, plays everything that changes from
//!   moment to moment: the rides (sleigh bells, hoofbeats, the snowmobile's
//!   two-stroke, runners hissing, skis swishing, Santa's diesel), and the
//!   music: two original tunes, wintry rather than Christmassy so the game
//!   sits well any time of year. A gentle pentatonic theme on the menus; a
//!   driving one in E minor in the race, faster on the final lap, with a low
//!   warning bass under it while Santa is after you.
//!
//! Without an audio device, or with `--mute`, every call does nothing.

use rodio::buffer::SamplesBuffer;
use rodio::{MixerDeviceSink, Source};
use std::f32::consts::TAU;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const SR: u32 = 44_100;
const SRF: f32 = SR as f32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sfx {
    Beep,
    Go,
    Hop,
    Tier1,
    Tier2,
    Tier3,
    Boost,
    Fanfare,
    Stall,
    Launch,
    Land,
    Trick,
    Thud,
    Thunk,
    Hit,
    Throw,
    Lob,
    Pop,
    Zing,
    Chime,
    Smooch,
    Ding,
    Tick,
    Horn,
    Whack,
    Lap,
    Finish,
    Click,
    Splash,
}

const ALL_SFX: [Sfx; 29] = [
    Sfx::Beep,
    Sfx::Go,
    Sfx::Hop,
    Sfx::Tier1,
    Sfx::Tier2,
    Sfx::Tier3,
    Sfx::Boost,
    Sfx::Fanfare,
    Sfx::Stall,
    Sfx::Launch,
    Sfx::Land,
    Sfx::Trick,
    Sfx::Thud,
    Sfx::Thunk,
    Sfx::Hit,
    Sfx::Throw,
    Sfx::Lob,
    Sfx::Pop,
    Sfx::Zing,
    Sfx::Chime,
    Sfx::Smooch,
    Sfx::Ding,
    Sfx::Tick,
    Sfx::Horn,
    Sfx::Whack,
    Sfx::Lap,
    Sfx::Finish,
    Sfx::Click,
    Sfx::Splash,
];

/// What a ride sounds like.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Voice {
    #[default]
    Silent,
    /// Sleigh bells and runners.
    Sleigh,
    /// Hoofbeats and a few bells.
    Hooves,
    /// A two-stroke engine.
    Motor,
    /// Runners only.
    Runners,
    /// Skis swishing in a skating rhythm.
    Skis,
    /// Santa's plough: a tank's growl and clanking tracks.
    Diesel,
}

/// One ride for the band: its voice, speed and how loud (by distance).
#[derive(Clone, Copy, Default, Debug)]
pub struct Ride {
    pub voice: Voice,
    pub speed: f32,
    pub gain: f32,
}

pub const RIDES: usize = 5;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum Music {
    #[default]
    Off,
    /// The menu theme.
    Menu,
    /// The race theme.
    Race,
    /// The race theme, faster.
    FinalLap,
}

#[derive(Clone, Copy, Default)]
struct Live {
    rides: [Ride; RIDES],
    music: Music,
    /// Santa is after the player: the warning bass.
    chased: bool,
    silent: bool,
    music_off: bool,
}

pub struct Audio {
    // the sink must outlive everything playing on its mixer
    sink: Option<MixerDeviceSink>,
    bank: Vec<SamplesBuffer>,
    live: Arc<Mutex<Live>>,
    pub muted: bool,
    pub music_on: bool,
    last: [Option<Instant>; ALL_SFX.len()],
}

impl Audio {
    pub fn new(muted: bool) -> Audio {
        let sink = if muted {
            None
        } else {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(mut s) => {
                    s.log_on_drop(false);
                    Some(s)
                }
                Err(e) => {
                    log::warn!("no audio: {e}");
                    None
                }
            }
        };
        let live = Arc::new(Mutex::new(Live::default()));
        let bank = if sink.is_some() { ALL_SFX.iter().map(|&s| buffer(render_sfx(s))).collect() } else { Vec::new() };
        if let Some(s) = &sink {
            s.mixer().add(Band::new(live.clone()));
        }
        Audio { sink, bank, live, muted, music_on: true, last: [None; ALL_SFX.len()] }
    }

    pub fn available(&self) -> bool {
        self.sink.is_some()
    }

    pub fn play(&mut self, sfx: Sfx, volume: f32) {
        if self.muted || volume <= 0.01 {
            return;
        }
        let Some(sink) = &self.sink else { return };
        let i = ALL_SFX.iter().position(|&s| s == sfx).unwrap_or(0);
        let now = Instant::now();
        if self.last[i].is_some_and(|t| now.duration_since(t) < Duration::from_millis(40)) {
            return;
        }
        self.last[i] = Some(now);
        sink.mixer().add(self.bank[i].clone().amplify(volume));
    }

    pub fn set(&mut self, rides: &[Ride], music: Music, chased: bool) {
        if let Ok(mut l) = self.live.lock() {
            l.rides = [Ride::default(); RIDES];
            for (slot, r) in l.rides.iter_mut().zip(rides) {
                *slot = *r;
            }
            l.music = music;
            l.chased = chased;
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Ok(mut l) = self.live.lock() {
            l.silent = self.muted;
        }
    }

    pub fn toggle_music(&mut self) {
        self.music_on = !self.music_on;
        if let Ok(mut l) = self.live.lock() {
            l.music_off = !self.music_on;
        }
    }
}

fn buffer(samples: Vec<f32>) -> SamplesBuffer {
    SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(SR).unwrap(), samples)
}

// ------------------------------------------------------------------ helpers

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

fn square(ph: f32) -> f32 {
    if ph - ph.floor() < 0.5 {
        1.0
    } else {
        -1.0
    }
}

fn saw(ph: f32) -> f32 {
    2.0 * (ph - ph.floor()) - 1.0
}

fn tri(ph: f32) -> f32 {
    let p = ph - ph.floor();
    4.0 * (p - 0.5).abs() - 1.0
}

/// `dur` seconds of samples from `f(t, noise)`.
fn synth(dur: f32, mut f: impl FnMut(f32, &mut Noise) -> f32) -> Vec<f32> {
    let mut n = Noise(0x1234_5678);
    (0..(dur * SRF) as usize).map(|i| f(i as f32 / SRF, &mut n)).collect()
}

/// A little tune: (midi note, seconds) on `wave`, softly enveloped.
fn tune(notes: &[(f32, f32)], gain: f32, wave: fn(f32) -> f32) -> Vec<f32> {
    let mut out = Vec::new();
    let mut ph = 0.0f32;
    let mut lp = 0.0f32;
    for &(n, d) in notes {
        let len = (d * SRF) as usize;
        for i in 0..len {
            let t = i as f32 / SRF;
            if n > 0.0 {
                ph += midi(n) / SRF;
            }
            let env = (t / 0.01).min(1.0) * ((d - t) / 0.03).clamp(0.0, 1.0);
            let x = if n > 0.0 { wave(ph) * env } else { 0.0 };
            lp += (x - lp) * 0.4;
            out.push(lp * gain);
        }
    }
    out
}

/// A sleigh bell: a few bright, inharmonic partials ringing down fast.
fn bell(t: f32, pitch: f32) -> f32 {
    let env = (-t * 22.0).exp();
    ((TAU * 2100.0 * pitch * t).sin() * 0.5 + (TAU * 3370.0 * pitch * t).sin() * 0.3 + (TAU * 4910.0 * pitch * t).sin() * 0.2) * env
}

fn render_sfx(s: Sfx) -> Vec<f32> {
    match s {
        Sfx::Beep => tune(&[(72.0, 0.16)], 0.28, square),
        Sfx::Go => tune(&[(84.0, 0.4)], 0.3, square),
        Sfx::Click => tune(&[(88.0, 0.03)], 0.15, square),
        Sfx::Hop => synth(0.12, |t, _| (TAU * (300.0 + 900.0 * t) * t).sin() * (1.0 - t / 0.12) * 0.2),
        Sfx::Tier1 => tune(&[(79.0, 0.06), (84.0, 0.1)], 0.18, tri),
        Sfx::Tier2 => tune(&[(84.0, 0.06), (88.0, 0.1)], 0.2, tri),
        Sfx::Tier3 => tune(&[(88.0, 0.05), (91.0, 0.05), (96.0, 0.12)], 0.22, tri),
        Sfx::Boost => synth(0.5, |t, n| {
            let sweep = (TAU * (180.0 + 500.0 * t) * t).sin() * 0.15;
            (sweep + n.next() * 0.12 * (1.0 - t * 2.0).max(0.0)) * (1.0 - t / 0.5)
        }),
        Sfx::Fanfare => tune(&[(72.0, 0.09), (76.0, 0.09), (79.0, 0.09), (84.0, 0.35)], 0.25, square),
        Sfx::Stall => synth(0.8, |t, n| {
            let put = if (t * 14.0).fract() < 0.35 { 1.0 } else { 0.0 };
            (square(t * 50.0) * 0.3 + n.next() * 0.2) * put * (1.0 - t / 0.8) * 0.5
        }),
        Sfx::Launch => synth(0.35, |t, n| n.next() * 0.25 * (t / 0.05).min(1.0) * (1.0 - t / 0.35) * (0.4 + 0.6 * (TAU * 6.0 * t).sin().abs())),
        Sfx::Land => synth(0.3, |t, n| {
            // snow crunch: noise bursts, a soft thump under them
            let crunch = if (t * 90.0).fract() < 0.5 { n.next() } else { n.next() * 0.3 };
            (crunch * 0.35 + (TAU * 70.0 * t).sin() * 0.4) * (-t * 14.0).exp()
        }),
        Sfx::Trick => synth(0.45, |t, _| bell(t % 0.09, 1.0 + (t / 0.09).floor() * 0.25) * 0.35),
        Sfx::Thud => synth(0.18, |t, n| ((TAU * 80.0 * t).sin() * 0.5 + n.next() * 0.15) * (-t * 25.0).exp()),
        Sfx::Thunk => synth(0.35, |t, n| {
            let wood = (TAU * 140.0 * t).sin() * (-t * 18.0).exp() * 0.5;
            let snow = n.next() * 0.25 * (-((t - 0.08).max(0.0)) * 10.0).exp() * (t > 0.06) as i32 as f32;
            wood + snow
        }),
        Sfx::Hit => synth(0.35, |t, n| {
            let pow = (TAU * (240.0 - 300.0 * t) * t).sin() * 0.4;
            (pow + n.next() * 0.2) * (-t * 9.0).exp()
        }),
        Sfx::Throw => synth(0.18, |t, n| n.next() * 0.25 * (1.0 - t / 0.18) * (t / 0.02).min(1.0)),
        Sfx::Lob => synth(0.3, |t, _| (TAU * (500.0 - 900.0 * t) * t).sin() * 0.18 * (1.0 - t / 0.3)),
        Sfx::Pop => synth(0.5, |t, n| {
            let pop = n.next() * (-t * 40.0).exp() * 0.5;
            // and a tinkle of confetti
            pop + bell((t - 0.05).max(0.0) % 0.12, 1.3) * 0.15 * (t > 0.05) as i32 as f32
        }),
        Sfx::Zing => synth(0.4, |t, _| ((TAU * (900.0 + 1200.0 * t) * t).sin() + (TAU * 1810.0 * t).sin() * 0.3) * 0.15 * (1.0 - t / 0.4)),
        Sfx::Chime => tune(&[(84.0, 0.08), (91.0, 0.08), (96.0, 0.25)], 0.2, tri),
        Sfx::Smooch => synth(0.3, |t, n| {
            // mwah: a sweep up through a wet click
            let mw = (TAU * (180.0 + 600.0 * t) * t).sin() * (t / 0.3) * 0.25;
            let click = n.next() * (-((t - 0.24).abs()) * 200.0).exp() * 0.5;
            (mw + click) * (1.0 - t / 0.3).max(0.0)
        }),
        Sfx::Ding => synth(0.6, |t, _| bell(t, 1.0) * 0.4),
        Sfx::Tick => tune(&[(96.0, 0.03)], 0.2, square),
        Sfx::Horn => synth(1.1, |t, _| {
            // a big two-tone truck horn
            let env = (t / 0.04).min(1.0) * ((1.1 - t) / 0.1).clamp(0.0, 1.0) * if t < 0.45 || t > 0.55 { 1.0 } else { 0.0 };
            (saw(t * 155.0) * 0.5 + saw(t * 196.0) * 0.5) * 0.25 * env
        }),
        Sfx::Whack => synth(0.7, |t, n| {
            let whack = n.next() * (-t * 50.0).exp() * 0.6;
            // a cartoon boing after it
            let tb = (t - 0.08).max(0.0);
            let boing = (TAU * (300.0 + 120.0 * (TAU * 14.0 * tb).sin()) * tb).sin() * (-tb * 4.0).exp() * 0.2 * (t > 0.08) as i32 as f32;
            whack + boing
        }),
        Sfx::Lap => synth(0.8, |t, _| (bell(t, 0.8) + bell((t - 0.15).max(0.0), 1.0) * (t > 0.15) as i32 as f32) * 0.35),
        Sfx::Splash => synth(0.9, |t, n| {
            // the plunge, then bubbles
            let plunge = n.next() * (-t * 9.0).exp() * 0.45;
            let bub = (TAU * (500.0 + 300.0 * (t * 37.0).sin()) * t).sin() * 0.12 * ((t * 23.0).fract() < 0.3) as i32 as f32 * (1.0 - t / 0.9);
            plunge + bub
        }),
        Sfx::Finish => tune(&[(72.0, 0.12), (76.0, 0.12), (79.0, 0.12), (84.0, 0.12), (79.0, 0.12), (84.0, 0.5)], 0.25, square),
    }
}

// ---------------------------------------------------------------- the band

/// A melody note in beats: (midi, start, length).
type Note = (f32, f32, f32);

/// A tune: its notes, its length in beats, and a bass root per 4-beat bar.
struct Song {
    notes: Vec<Note>,
    len: f32,
    roots: Vec<f32>,
}

/// "First Light", the menu theme: airy, in C major pentatonic.
fn first_light() -> Song {
    let a: &[(f32, f32)] = &[
        (64.0, 1.0), (67.0, 1.0), (69.0, 2.0),
        (67.0, 1.0), (64.0, 1.0), (62.0, 2.0),
        (60.0, 1.0), (62.0, 1.0), (64.0, 1.0), (67.0, 1.0),
        (64.0, 4.0),
    ];
    let b: &[(f32, f32)] = &[
        (64.0, 1.0), (67.0, 1.0), (69.0, 2.0),
        (72.0, 1.0), (69.0, 1.0), (67.0, 2.0),
        (64.0, 1.0), (62.0, 1.0), (60.0, 1.0), (62.0, 1.0),
        (60.0, 4.0),
    ];
    let c: &[(f32, f32)] = &[
        (69.0, 1.5), (72.0, 0.5), (74.0, 2.0),
        (72.0, 1.0), (69.0, 1.0), (67.0, 2.0),
        (69.0, 1.0), (67.0, 1.0), (64.0, 1.0), (62.0, 1.0),
        (64.0, 2.0), (67.0, 2.0),
    ];
    sequence(&[a, b, c, b], &[48.0, 45.0, 41.0, 48.0, 48.0, 41.0, 43.0, 48.0, 45.0, 41.0, 48.0, 43.0, 48.0, 41.0, 43.0, 48.0])
}

/// "Runners", the race theme: driving, in E minor.
fn runners() -> Song {
    let a: &[(f32, f32)] = &[
        (64.0, 0.5), (67.0, 0.5), (71.0, 1.0), (69.0, 0.5), (67.0, 0.5), (64.0, 1.0),
        (62.0, 0.5), (64.0, 0.5), (67.0, 1.0), (69.0, 2.0),
        (64.0, 0.5), (67.0, 0.5), (71.0, 1.0), (74.0, 0.5), (71.0, 0.5), (69.0, 1.0),
        (67.0, 0.5), (69.0, 0.5), (66.0, 1.0), (64.0, 2.0),
    ];
    let b: &[(f32, f32)] = &[
        (71.0, 1.0), (72.0, 0.5), (71.0, 0.5), (69.0, 1.0), (67.0, 1.0),
        (69.0, 1.0), (71.0, 0.5), (69.0, 0.5), (67.0, 1.0), (66.0, 1.0),
        (67.0, 0.5), (69.0, 0.5), (71.0, 1.0), (76.0, 1.0), (74.0, 1.0),
        (71.0, 3.0), (66.0, 1.0),
    ];
    sequence(&[a, b, a, b], &[40.0, 36.0, 40.0, 35.0, 36.0, 38.0, 40.0, 35.0, 40.0, 36.0, 40.0, 35.0, 36.0, 38.0, 40.0, 35.0])
}

fn sequence(parts: &[&[(f32, f32)]], roots: &[f32]) -> Song {
    let mut notes = Vec::new();
    let mut at = 0.0;
    for part in parts {
        for &(n, d) in *part {
            notes.push((n, at, d));
            at += d;
        }
    }
    Song { notes, len: at, roots: roots.to_vec() }
}

#[derive(Clone, Copy)]
struct Tone {
    freq: f32,
    ph: f32,
    age: f32,
    len: f32,
    gain: f32,
    wave: fn(f32) -> f32,
}

#[derive(Clone, Copy, Default)]
struct RideState {
    ph: f32,
    ph2: f32,
    lp: f32,
    /// Time since the last bell / hoofbeat, and the next one's due time.
    since: f32,
    due: f32,
    beat: u32,
}

struct Band {
    live: Arc<Mutex<Live>>,
    state: Live,
    rides: [RideState; RIDES],
    /// Ringing bells from the rides: (age, pitch, gain).
    bells: Vec<(f32, f32, f32)>,
    /// Hoofbeats and kicks: (age, gain).
    thumps: Vec<(f32, f32)>,
    /// Hi-hats: age.
    hats: Vec<f32>,
    tones: Vec<Tone>,
    song: Music,
    notes: Vec<Note>,
    song_len: f32,
    roots: Vec<f32>,
    beat: f32,
    next_note: usize,
    noise: Noise,
    hiss_lp: f32,
    buf: Vec<f32>,
    pos: usize,
}

impl Band {
    fn new(live: Arc<Mutex<Live>>) -> Band {
        Band {
            live,
            state: Live::default(),
            rides: [RideState::default(); RIDES],
            bells: Vec::new(),
            thumps: Vec::new(),
            hats: Vec::new(),
            tones: Vec::new(),
            song: Music::Off,
            notes: Vec::new(),
            song_len: 1.0,
            roots: Vec::new(),
            beat: 0.0,
            next_note: 0,
            noise: Noise(0x2468_ACE1),
            hiss_lp: 0.0,
            buf: vec![0.0; 1024],
            pos: 1024,
        }
    }

    fn render(&mut self) {
        if let Ok(l) = self.live.lock() {
            self.state = *l;
        }
        let music = if self.state.music_off { Music::Off } else { self.state.music };
        let song = match music {
            Music::FinalLap => Music::Race,
            m => m,
        };
        if song != self.song {
            self.song = song;
            let tune = match song {
                Music::Menu => first_light(),
                Music::Race | Music::FinalLap => runners(),
                Music::Off => Song { notes: Vec::new(), len: 1.0, roots: Vec::new() },
            };
            self.notes = tune.notes;
            self.song_len = tune.len;
            self.roots = tune.roots;
            self.beat = 0.0;
            self.next_note = 0;
        }
        let bpm = match music {
            Music::Menu => 96.0,
            Music::Race => 140.0,
            Music::FinalLap => 172.0,
            Music::Off => 0.0,
        };
        for i in 0..self.buf.len() {
            let mut x = 0.0;
            if self.state.silent {
                self.buf[i] = 0.0;
                continue;
            }
            // the music
            if !self.notes.is_empty() {
                let before = self.beat;
                self.beat += bpm / 60.0 / SRF;
                while self.next_note < self.notes.len() && self.notes[self.next_note].1 <= self.beat {
                    let (n, _, d) = self.notes[self.next_note];
                    let secs = d * 60.0 / bpm;
                    let (gain, wave): (f32, fn(f32) -> f32) = if self.song == Music::Menu { (0.11, tri) } else { (0.065, square) };
                    self.tones.push(Tone { freq: midi(n), ph: 0.0, age: 0.0, len: secs * 0.92, gain, wave });
                    self.next_note += 1;
                }
                // bass every beat (eighths in the race), a soft hi-hat on
                // the off-beats, kick on 1 and 3 in the race
                let racing = self.song != Music::Menu;
                let sub = if racing { 2.0 } else { 1.0 };
                if (before * sub).floor() != (self.beat * sub).floor() {
                    let b = self.beat.floor();
                    let bar = (b / 4.0).floor() as usize;
                    let root = self.roots.get(bar % self.roots.len().max(1)).copied().unwrap_or(48.0);
                    let step = (self.beat * sub).floor() as i32;
                    let n = match (racing, step % 4) {
                        (true, 1) | (true, 3) => root + 12.0,
                        (true, 2) => root + 7.0,
                        (false, s) if s % 2 == 1 => root + 7.0,
                        _ => root,
                    };
                    self.tones.push(Tone { freq: midi(n), ph: 0.0, age: 0.0, len: 60.0 / bpm / sub * 0.8, gain: 0.09, wave: tri });
                }
                if before.floor() != self.beat.floor() {
                    let b = self.beat.floor();
                    let bar = (b / 4.0).floor() as usize;
                    let root = self.roots.get(bar % self.roots.len().max(1)).copied().unwrap_or(48.0);
                    if racing && (b as i32) % 2 == 0 {
                        self.thumps.push((0.0, 0.35));
                    }
                    if self.state.chased {
                        self.tones.push(Tone { freq: midi(root - 12.0), ph: 0.0, age: 0.0, len: 60.0 / bpm * 0.5, gain: 0.12, wave: saw });
                    }
                }
                if (before * 2.0).floor() != (self.beat * 2.0).floor() && (self.beat * 2.0).floor() as i32 % 2 == 1 {
                    self.hats.push(0.0);
                }
                if self.beat >= self.song_len {
                    self.beat -= self.song_len;
                    self.next_note = 0;
                }
            }
            for t in &mut self.tones {
                t.ph += t.freq / SRF;
                t.age += 1.0 / SRF;
                let env = (t.age / 0.01).min(1.0) * ((t.len - t.age) / 0.04).clamp(0.0, 1.0);
                x += (t.wave)(t.ph) * t.gain * env;
            }
            self.tones.retain(|t| t.age < t.len);
            // the rides
            let mut hiss = 0.0;
            for k in 0..RIDES {
                let r = self.state.rides[k];
                if r.voice == Voice::Silent || r.gain <= 0.0 {
                    continue;
                }
                let sp = (r.speed.abs() / 65.0).min(1.5);
                let st = &mut self.rides[k];
                st.since += 1.0 / SRF;
                match r.voice {
                    Voice::Sleigh | Voice::Hooves => {
                        // bells shaken faster with speed
                        let rate = 2.0 + sp * 9.0;
                        if sp > 0.05 && st.since >= st.due {
                            st.since = 0.0;
                            st.due = (1.0 + self.noise.next() * 0.35) / rate;
                            let pitch = 1.0 + self.noise.next() * 0.08;
                            let g = if r.voice == Voice::Sleigh { 0.16 } else { 0.07 };
                            self.bells.push((0.0, pitch, g * r.gain));
                        }
                        if r.voice == Voice::Hooves && sp > 0.05 {
                            // a gallop: three beats and a gap, one stride per
                            // 11 units, as the legs are drawn
                            st.ph += r.speed.abs() / 11.0 / SRF;
                            let beat = ((st.ph.fract()) * 4.0) as u32;
                            if beat != st.beat {
                                st.beat = beat;
                                if beat < 3 {
                                    self.thumps.push((0.0, 0.3 * r.gain));
                                }
                            }
                        }
                        hiss += sp * r.gain * 0.5;
                    }
                    Voice::Motor => {
                        st.ph += (52.0 + 120.0 * sp) / SRF;
                        st.ph2 += (25.0 + 50.0 * sp) / SRF;
                        let raw = saw(st.ph) * 0.6 + square(st.ph2) * 0.4;
                        st.lp += (raw - st.lp) * 0.12;
                        x += st.lp * 0.09 * r.gain * (0.5 + 0.5 * sp.min(1.0));
                        hiss += sp * r.gain * 0.2;
                    }
                    Voice::Runners => hiss += sp * r.gain,
                    Voice::Skis => {
                        st.ph += r.speed.abs() / 10.0 * 2.0 / SRF;
                        let swish = (st.ph.fract() * std::f32::consts::PI).sin().powi(2);
                        hiss += sp * r.gain * (0.3 + 1.2 * swish);
                    }
                    Voice::Diesel => {
                        // a tank: two detuned square waves beating low, with
                        // rumble underneath...
                        st.ph += (30.0 + 18.0 * sp) / SRF;
                        st.ph2 += (31.3 + 18.0 * sp) / SRF;
                        let raw = (square(st.ph) + square(st.ph2)) * 0.5 + self.noise.next() * 0.3;
                        st.lp += (raw - st.lp) * 0.07;
                        x += st.lp * 0.34 * r.gain * (0.55 + 0.45 * sp.min(1.0));
                        // ...and the tracks clanking round, faster with speed
                        let rate = 5.0 + 16.0 * sp;
                        if st.since >= 1.0 / rate {
                            st.since = 0.0;
                            self.bells.push((0.0, 0.32 + self.noise.next() * 0.03, 0.22 * r.gain));
                        }
                    }
                    Voice::Silent => {}
                }
            }
            // runners on snow: one shared, filtered hiss
            self.hiss_lp += (self.noise.next() - self.hiss_lp) * 0.25;
            x += self.hiss_lp * 0.05 * hiss.min(2.0);
            for b in &mut self.bells {
                x += bell(b.0, b.1) * b.2;
                b.0 += 1.0 / SRF;
            }
            self.bells.retain(|b| b.0 < 0.3);
            for t in &mut self.thumps {
                x += ((TAU * 75.0 * t.0).sin() * 0.8 + self.noise.next() * 0.1) * (-t.0 * 30.0).exp() * t.1;
                t.0 += 1.0 / SRF;
            }
            self.thumps.retain(|t| t.0 < 0.2);
            for h in &mut self.hats {
                let n = self.noise.next();
                x += (n - self.hiss_lp * 0.8) * (-*h * 60.0).exp() * 0.05;
                *h += 1.0 / SRF;
            }
            self.hats.retain(|h| *h < 0.08);
            self.buf[i] = x.clamp(-1.0, 1.0);
        }
    }
}

impl Iterator for Band {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.pos >= self.buf.len() {
            self.render();
            self.pos = 0;
        }
        self.pos += 1;
        Some(self.buf[self.pos - 1])
    }
}

impl Source for Band {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(1).unwrap()
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(SR).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effects_render_and_stay_in_range() {
        for s in ALL_SFX {
            let v = render_sfx(s);
            assert!(!v.is_empty(), "{s:?} is silent");
            let peak = v.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.02 && peak <= 1.0, "{s:?} peak {peak}");
        }
    }

    fn rms(live: Live, secs: f32) -> f32 {
        let mut b = Band::new(Arc::new(Mutex::new(live)));
        let n = (secs * SRF) as usize;
        let sum: f32 = (0..n).map(|_| b.next().unwrap().powi(2)).sum();
        (sum / n as f32).sqrt()
    }

    #[test]
    fn the_band_plays_every_ride_and_every_tune() {
        assert_eq!(rms(Live::default(), 0.2), 0.0, "nothing set, nothing heard");
        for voice in [Voice::Sleigh, Voice::Hooves, Voice::Motor, Voice::Runners, Voice::Skis, Voice::Diesel] {
            let mut l = Live::default();
            l.rides[0] = Ride { voice, speed: 60.0, gain: 1.0 };
            assert!(rms(l, 1.0) > 0.003, "{voice:?} is heard");
        }
        for music in [Music::Menu, Music::Race, Music::FinalLap] {
            let l = Live { music, ..Default::default() };
            assert!(rms(l, 2.0) > 0.01, "{music:?} plays");
        }
        let muted = Live { music: Music::Race, silent: true, ..Default::default() };
        assert_eq!(rms(muted, 0.5), 0.0);
    }

    #[test]
    fn the_songs_loop_on_whole_bars_with_a_root_for_each() {
        for song in [first_light(), runners()] {
            assert!(song.notes.len() > 30);
            let last = song.notes.last().unwrap();
            assert!((last.1 + last.2 - song.len).abs() < 1e-3);
            assert_eq!(song.len % 4.0, 0.0, "whole bars");
            assert_eq!(song.roots.len() as f32, song.len / 4.0, "a bass root per bar");
        }
    }
}
