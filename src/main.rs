mod auto;
mod game;
mod input;
mod kart;
mod model;
mod roster;
mod race;
mod render;
mod track;

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
            "sleighride [--race] [--course N]\n\n\
             Sleighride: kart racing with Santa's runaway helpers. In game, H shows the controls.\n\
             --race                      skip the title screen\n\
             --course N                  1 Frozen Lake, 2 Eggnogg\n\
             --shot FILE COLS ROWS SECS  write a PPM of the attract race after SECS\n\
             \x20                           (SECS ending in d: then wait for a gold drift)"
        );
        return Ok(());
    }
    let start = if args.iter().any(|a| a == "--race") { Phase::Race } else { Phase::Title };
    let n = track::courses().len();
    let course = args.iter().position(|a| a == "--course").and_then(|i| args.get(i + 1)).and_then(|v| v.parse::<usize>().ok()).map_or(0, |c| c.clamp(1, n) - 1);
    if let Some(i) = args.iter().position(|a| a == "--shot") {
        let get = |k: usize| args.get(i + k).cloned().unwrap_or_default();
        // SECS ending in "d": wait from then on for a slide with gold sparks
        let secs_arg = get(4);
        let wait_drift = secs_arg.ends_with('d');
        let (cols, rows, secs) = (get(2).parse().unwrap_or(160), get(3).parse().unwrap_or(48), secs_arg.trim_end_matches('d').parse().unwrap_or(10.0));
        let mut g = Game::new(start, course);
        g.run_auto(secs);
        while wait_drift && g.race.racers[0].kart.tier() < 2 && g.race.clock < 600.0 {
            g.run_auto(DT);
        }
        std::fs::write(get(1), g.shot(cols, rows).ppm())?;
        return Ok(());
    }
    let mut terminal = ratatui::init();
    let kitty = supports_keyboard_enhancement().unwrap_or(false);
    if kitty {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES | KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
        execute!(io::stdout(), PushKeyboardEnhancementFlags(flags))?;
    }
    log::info!("start kitty_keyboard={kitty}");
    let mut input = Input::new(kitty);
    input.open_pad();
    let mut game = Game::new(start, course);
    let result = run(&mut terminal, &mut game, &mut input);
    if kitty {
        let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    }
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, game: &mut Game, input: &mut Input) -> io::Result<()> {
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
