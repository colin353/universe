use raw_tty::{GuardMode, IntoRawMode};
use std::io::Read;
use tui::{Component, KeyboardEvent, Transition};

#[derive(Clone, PartialEq)]
pub struct SelectionState {
    pub focused: bool,
    pub selected: usize,
}

struct Selector {
    choices: Vec<String>,
}

impl Selector {
    fn new(choices: Vec<String>) -> Self {
        Self { choices }
    }
}

impl tui::Component<SelectionState> for Selector {
    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &SelectionState,
        prev_state: Option<&SelectionState>,
    ) -> usize {
        if let Some(prev) = prev_state {
            if state == prev {
                return self.choices.len();
            }
        } else {
            t.disable_wrap();
        }

        // If there is no previous state, then we must do the initial render
        for (idx, choice) in self.choices.iter().enumerate() {
            t.move_cursor_to(0, idx);
            t.clear_line();
            t.set_inverted();
            t.print(" ");

            if idx == state.selected {
                t.print(" ");
                t.print(choice);
                t.set_normal();
            } else {
                t.set_normal();
                t.print(" ");
                t.print(choice);
            }
        }

        self.choices.len()
    }
}

pub struct App {
    component: Selector,
}
impl App {
    pub fn new(choices: Vec<String>) -> Self {
        Self {
            component: Selector::new(choices),
        }
    }
}

impl tui::AppController<SelectionState, KeyboardEvent> for App {
    fn render(
        &mut self,
        term: &mut tui::Terminal,
        state: &SelectionState,
        prev_state: Option<&SelectionState>,
    ) {
        self.component.render(term, state, prev_state);
    }

    fn clean_up(&self, term: &mut tui::Terminal) {
        term.move_cursor_to(0, self.component.choices.len());
        term.print("\r\n");
        term.enable_wrap();
        term.show_cursor();
    }

    fn transition(
        &mut self,
        state: &SelectionState,
        event: KeyboardEvent,
    ) -> Transition<SelectionState> {
        match event {
            KeyboardEvent::Character('k') | KeyboardEvent::UpArrow => {
                let mut new_state = (*state).clone();
                if new_state.selected == 0 {
                    new_state.selected = self.component.choices.len() - 1;
                } else {
                    new_state.selected -= 1;
                }
                Transition::Updated(new_state)
            }
            KeyboardEvent::Character('j') | KeyboardEvent::DownArrow => {
                let mut new_state = (*state).clone();
                new_state.selected = (new_state.selected + 1) % self.component.choices.len();
                Transition::Updated(new_state)
            }
            KeyboardEvent::CtrlC | KeyboardEvent::CtrlD => Transition::Terminate((*state).clone()),
            KeyboardEvent::Enter => Transition::Finished((*state).clone()),
            _ => Transition::Nothing,
        }
    }

    fn initial_state(&self) -> SelectionState {
        SelectionState {
            selected: 0,
            focused: false,
        }
    }
}

pub fn select(choices: Vec<String>) -> Option<usize> {
    let size = choices.len();
    let ctrl = App::new(choices.clone());

    match choices.len() {
        0 => None,
        1 => Some(0),
        _ => {
            eprint!("{}", (0..size).map(|_| "\n").collect::<String>());
            let mut term = tui::Terminal::new();
            let (x, y) = term.get_cursor_pos();
            term.offset_y = y - size - 1;

            let mut app = tui::App::start_with_terminal(Box::new(ctrl), term);
            let mut tty = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
                .unwrap();
            let mut tty_input = tty.try_clone().unwrap().guard_mode().unwrap();
            tty_input.set_raw_mode();

            for e in tui::KeyboardEventStream::new(&mut tty_input) {
                match app.handle_event(e) {
                    Transition::Finished(final_state) => {
                        return Some(final_state.selected);
                    }
                    Transition::Terminate(_) => {
                        return None;
                    }
                    _ => continue,
                }
            }
            None
        }
    }
}
