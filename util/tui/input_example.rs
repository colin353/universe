use std::io::Read;

use raw_tty::{GuardMode, IntoRawMode};
use tui::{Component, Transition};

fn main() {
    let mut term = tui::Terminal::new();
    let ctrl = filter::Filter::new(
        "branch name".to_string(),
        vec![
            "apples".to_string(),
            "avocado".to_string(),
            "bananas".to_string(),
            "grapefruit".to_string(),
            "grapes".to_string(),
        ],
    );

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
            Transition::Finished(final_state) => {
                return;
            }
            Transition::Terminate(_) => {
                return;
            }
            _ => continue,
        }
    }
}
