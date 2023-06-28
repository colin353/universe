use raw_tty::GuardMode;
use tui::Transition;

pub fn choose_space(mut spaces: Vec<(String, service::Space)>) -> Option<(String, service::Space)> {
    let choices: Vec<_> = spaces
        .iter()
        .map(|(name, c)| format!("{}\t\t\t{}", name, core::fmt_basis(c.basis.as_view())))
        .collect();
    let ctrl = filter::Filter::new("pick a space".to_string(), choices.clone());

    let term = tui::Terminal::new();
    let mut app = tui::App::start_with_terminal(Box::new(ctrl), term);
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .unwrap();
    let mut tty_input = tty.try_clone().unwrap().guard_mode().unwrap();
    tty_input.set_raw_mode().unwrap();

    let stream = tui::KeyboardEventStream::new(&mut tty_input);
    for event in stream {
        match app.handle_event(event) {
            Transition::Terminate(_) => return None,
            Transition::Finished(s) => {
                let idx = match s.items {
                    filter::ItemsState::All => {
                        if choices.len() > s.scroll + s.selected {
                            s.scroll + s.selected
                        } else {
                            return None;
                        }
                    }
                    filter::ItemsState::Subset(subset) => {
                        if subset.len() > s.scroll + s.selected {
                            subset[s.scroll + s.selected]
                        } else {
                            return None;
                        }
                    }
                };
                return Some(spaces.remove(idx));
            }
            _ => continue,
        }
    }
    None
}
