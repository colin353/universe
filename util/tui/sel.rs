use tui::{Component, Transition};

#[derive(Clone, PartialEq)]
pub struct SelectionState {
    selected: usize,
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
        }

        // If there is no previous state, then we must do the initial render
        if let Some(prev) = prev_state {
            t.move_cursor_to(1, prev.selected);
            t.print(" ");
            t.move_cursor_to(1, state.selected);
            t.print("x");
        } else {
            for (idx, choice) in self.choices.iter().enumerate() {
                t.move_cursor_to(0, idx);
                if idx == state.selected {
                    t.print("[x] ");
                } else {
                    t.print("[ ] ");
                }
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

impl tui::AppController<SelectionState, char> for App {
    fn render(
        &mut self,
        term: &mut tui::Terminal,
        state: &SelectionState,
        prev_state: Option<&SelectionState>,
    ) {
        self.component.render(term, state, prev_state);
    }

    fn transition(&mut self, state: &SelectionState, event: char) -> Transition<SelectionState> {
        match event {
            'k' => {
                let mut new_state = (*state).clone();
                if new_state.selected == 0 {
                    new_state.selected = self.component.choices.len() - 1;
                } else {
                    new_state.selected -= 1;
                }
                Transition::Updated(new_state)
            }
            'j' => {
                let mut new_state = (*state).clone();
                new_state.selected = (new_state.selected + 1) % self.component.choices.len();
                Transition::Updated(new_state)
            }
            _ => Transition::Nothing,
        }
    }

    fn initial_state(&self) -> SelectionState {
        SelectionState { selected: 0 }
    }
}
