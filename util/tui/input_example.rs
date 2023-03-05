use std::io::Read;

use raw_tty::{GuardMode, IntoRawMode};
use tui::{Component, Transition};

fn main() -> Result<(), &'static str> {
    let mut term = tui::Terminal::new();
    let prompt = flags::define_flag!("prompt", String::new(), "The prompt to show");
    flags::parse_flags!(prompt);

    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    let choices = buffer
        .split("\n")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let ctrl = filter::Filter::new(prompt.value(), choices.clone());

    let mut app = tui::App::start_with_terminal(Box::new(ctrl), term);
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .unwrap();
    let mut tty_input = tty.try_clone().unwrap().guard_mode().unwrap();
    tty_input.set_raw_mode();

    let stream = tui::KeyboardEventStream::new(&mut tty_input);
    for event in stream {
        match app.handle_event(event) {
            Transition::Terminate(s) => return Err("no item selected"),
            Transition::Finished(s) => {
                let length = match s.items {
                    filter::ItemsState::All => {
                        if choices.len() > s.scroll + s.selected {
                            println!("{}", choices[s.scroll + s.selected]);
                            return Ok(())
                        } else {
                            return Err("no item selected")
                        }
                    }
                    filter::ItemsState::Subset(subset) => {
                        if subset.len() > s.scroll + s.selected {
                            println!("{}", choices[subset[s.scroll + s.selected]]);
                            return Ok(())
                        } else {
                            return Err("no item selected")
                        }
                    }
                };
            }
            _ => continue,
        }
    }

    Err("unreachable state")
}
