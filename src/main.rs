mod audio;
mod auto;
mod birds;
mod folklore;
mod game;
mod input;
mod kart;
mod model;
mod proto;
mod netplay;
mod net;
mod lan;
mod roster;
mod story;
mod race;
mod render;
mod track;
mod weather;
mod wildlife;

use game::{Game, Phase};
use input::Input;
use kart::DT;
use ratatui::crossterm::event::{self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use std::io;
use std::time::{Duration, Instant};

const SNAPSHOT_EVERY: Duration = Duration::from_millis(500);
const FRAME: Duration = Duration::from_micros(16_667);

fn main() -> io::Result<()> {
    // Session logging: RUST_LOG=debug sleighride 2>sleighride.log
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "sleighride [--race] [--course N] [--as NAME] [--mute] [--host | --join [ID]] [--name NAME]\n\n\
             Sleighride: kart racing with Santa's runaway helpers. In game, H shows the controls.\n\
             --race                      skip the title screen\n\
             --course N                  1 Frozen Lake, 2 Eggnogg, 3 Thin Ice, 4 Candy Cane Forest\n\
             --as NAME                   race as Bob, Holly, Sprocket, Pip, Juniper or Nutmeg\n\
             --mute                      no sound (F8 toggles sound, M the music)\n\
             --host                      host a race others can join (local network or internet)\n\
             --join                      look for races on the local network\n\
             --join ID                   join a host by its iroh id (works across the internet)\n\
             --name NAME                 your name in network races\n\
             --shot FILE COLS ROWS SECS  write a PPM of the attract race after SECS\n\
             \x20                           (SECS ending in d: then wait for a gold drift)"
        );
        return Ok(());
    }
    let start = if args.iter().any(|a| a == "--race") { Phase::Race } else { Phase::Title };
    let n = track::courses().len();
    let course = args.iter().position(|a| a == "--course").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<usize>().ok()).map_or(0, |c| c.clamp(1, n) - 1);
    let pick = args.iter().position(|a| a == "--as").and_then(|i| args.get(i + 1)).and_then(|v| roster::ROSTER.iter().position(|w| w.name.eq_ignore_ascii_case(v))).unwrap_or(0);
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        let get = |k: usize| args.get(i + k).cloned().unwrap_or_default();
        // SECS ending in "d": wait from then on for a slide with gold sparks
        let secs_arg = get(4);
        // ...or ending in "a": wait for the racer to be in the air
        let wait_drift = secs_arg.ends_with('d');
        let wait_air = secs_arg.ends_with('a');
        // ...or "c": wait for Santa to have just caught the racer; "i": for
        // the racer to be just short of an inlet
        let wait_clip = secs_arg.ends_with('c');
        let wait_inlet = secs_arg.ends_with('i');
        // ...or "t": for a snow tornado just ahead
        let wait_twister = secs_arg.ends_with('t');
        // ...or "p": for penguins on the road just ahead
        let wait_penguins = secs_arg.ends_with('p');
        // ...or "k": for the Yule Cat just ahead ("K": mid-pounce)
        let wait_cat = secs_arg.ends_with('k') || secs_arg.ends_with('K');
        let pounce = secs_arg.ends_with('K');
        let (cols, rows, secs) = (get(2).parse().unwrap_or(160), get(3).parse().unwrap_or(48), secs_arg.trim_end_matches(['d', 'a', 'c', 'i', 't', 'p', 'k', 'K']).parse().unwrap_or(10.0));
        let mut g = Game::new(start, course, pick, 1);
        if args.iter().any(|a| a == "--select") {
            g.phase = Phase::Select;
        }
        if args.iter().any(|a| a == "--story") {
            g.show_story(secs);
            std::fs::write(get(1), g.shot(cols, rows).ppm())?;
            return Ok(());
        }
        g.run_auto(secs);
        let me = g.race.player().unwrap_or(pick);
        while wait_drift && g.race.racers[me].kart.tier() < 2 && g.race.clock < 600.0 {
            g.run_auto(DT);
        }
        while wait_air && !(g.race.racers[me].kart.air && g.race.racers[me].kart.vz < 2.0) && g.race.clock < 600.0 {
            g.run_auto(DT);
        }
        while wait_inlet && g.race.clock < 600.0 && !g.race.track.inlets.iter().any(|&(s0, _)| (s0 + g.race.track.len() - g.race.racers[me].idx) % g.race.track.len() < std::env::var("SLEIGHRIDE_INLET_AHEAD").ok().and_then(|v| v.parse().ok()).unwrap_or(45)) {
            g.run_auto(DT);
        }
        while wait_clip && !(g.race.racers[me].clipped > 0.0 && g.race.racers[me].clipped < race::CLIP_TIME - 0.2) && g.race.clock < 600.0 {
            g.run_auto(DT);
        }
        // something just ahead of the racer, within a lane or two
        let ahead_of = |g: &Game, x: f32, y: f32| {
            let k = &g.race.racers[me].kart;
            let (dx, dy) = (x - k.x, y - k.y);
            let along = dx * k.heading.cos() + dy * k.heading.sin();
            let side = -dx * k.heading.sin() + dy * k.heading.cos();
            along > 15.0 && along < 40.0 && side.abs() < 16.0
        };
        while wait_twister && g.race.clock < 600.0 && !g.race.weather.twisters.iter().any(|t| ahead_of(&g, t.x, t.y)) {
            g.run_auto(DT);
        }
        while wait_penguins && g.race.clock < 600.0 && !g.race.wildlife.penguins.iter().any(|p| p.gait == wildlife::Gait::Waddling && ahead_of(&g, p.x, p.y)) {
            g.run_auto(DT);
        }
        while wait_cat && g.race.clock < 900.0 && !g.race.folklore.cat.is_some_and(|c| ahead_of(&g, c.x, c.y) && (!pounce || matches!(c.state, folklore::Cat::Pouncing(t) if t > 0.3))) {
            g.run_auto(DT);
        }
        if std::env::var("SLEIGHRIDE_SHOT_DEBUG").is_ok() {
            let k = &g.race.racers[me].kart;
            eprintln!("shot at clock {:.2}: racer {me} pos ({:.0},{:.0}) heading {:.2}", g.race.clock, k.x, k.y, k.heading);
            if let Some(c) = g.race.folklore.cat {
                eprintln!("cat at ({:.0},{:.0}) z {:.1} {:?}", c.x, c.y, c.z, c.state);
            }
        }
        std::fs::write(get(1), g.shot(cols, rows).ppm())?;
        return Ok(());
    }
    // open audio before the alternate screen, so any ALSA chatter lands above it
    let mut audio = audio::Audio::new(args.iter().any(|a| a == "--mute"));
    log::info!("audio available {} muted {}", audio.available(), audio.muted);
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    log::info!("start kitty_keyboard={kitty}");
    let mut input = Input::new(kitty);
    input.open_pad();
    let mut game = Game::new(start, course, pick, 1);
    let name = args
        .iter()
        .position(|a| a == "--name")
        .and_then(|i| args.get(i + 1).cloned())
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "elf".into());
    if args.iter().any(|a| a == "--host") {
        game.link = Some(Box::new(netplay::Link::host(name)));
    } else if let Some(i) = args.iter().position(|a| a == "--join") {
        match args.get(i + 1).filter(|v| !v.starts_with("--")) {
            Some(id) => match id.parse::<iroh::EndpointId>() {
                Ok(id) => game.link = Some(Box::new(netplay::Link::join(iroh::EndpointAddr::new(id), name))),
                Err(e) => {
                    ratatui::restore();
                    eprintln!("not an iroh id: {id}: {e}");
                    std::process::exit(2);
                }
            },
            None => game.link = Some(Box::new(netplay::Link::browse(name))),
        }
    }
    let result = run(&mut terminal, &mut game, &mut input, &mut audio);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, game: &mut Game, input: &mut Input, audio: &mut audio::Audio) -> io::Result<()> {
    let mut last = Instant::now();
    let mut acc = 0.0f32;
    let mut snapshot = Instant::now();
    loop {
        let frame_start = Instant::now();
        input.poll_pad();
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(k) => {
                    log::debug!("key {:?} {:?} {:?}", k.code, k.kind, k.modifiers);
                    input.on_key(k, Instant::now());
                }
                Event::Resize(w, h) => log::info!("resize {w}x{h}"),
                _ => {}
            }
        }
        let now = Instant::now();
        acc += now.duration_since(last).as_secs_f32().min(0.1);
        last = now;
        while acc >= DT {
            acc -= DT;
            game.tick(input);
            if game.exit {
                return Ok(());
            }
        }
        for c in std::mem::take(&mut game.audio_cmds) {
            match c {
                input::Cmd::Mute => audio.toggle_mute(),
                input::Cmd::Music => audio.toggle_music(),
                _ => {}
            }
            log::info!("audio muted {} music {}", audio.muted, audio.music_on);
        }
        for (sfx, volume) in std::mem::take(&mut game.sounds) {
            audio.play(sfx, volume);
        }
        let (rides, music, chased) = game.soundscape();
        audio.set(&rides, music, chased, game.wind());
        terminal.draw(|f| game.draw(f, input))?;
        if snapshot.elapsed() >= SNAPSHOT_EVERY {
            snapshot = Instant::now();
            game.snapshot();
        }
        let spent = frame_start.elapsed();
        if spent < FRAME {
            // wait for input or the next frame, whichever comes first
            let _ = event::poll(FRAME - spent)?;
        }
    }
}
