use std::io::Read;
use tui::{Component, KeyboardEvent, Transition};

pub struct Input {
    prompt: String,
    placeholder: String,
    initial_value: String,
}

impl Input {
    pub fn new(prompt: String, placeholder: String, initial_value: String) -> Self {
        Self {
            prompt,
            placeholder,
            initial_value,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct InputState {
    pub focused: bool,
    pub value: String,
    pub cursor: usize,
}

impl tui::AppController<InputState, KeyboardEvent> for Input {
    fn initial_state(&self) -> InputState {
        InputState {
            focused: true,
            value: String::new(),
            cursor: 0,
        }
    }

    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &InputState,
        prev_state: Option<&InputState>,
    ) {
        if let Some(prev) = prev_state {
            if state == prev {
                return;
            }

            // Clear out any excess characters if value is shorter
            if prev.value.len() > state.value.len() {
                t.move_cursor_to(self.prompt.len() + 2 + state.value.len(), 0);
                for _ in state.value.len()..prev.value.len() {
                    t.print(" ");
                }
            }
        }

        // Draw the framing if we've not drawn before
        if prev_state.is_none() {
            t.move_cursor_to(0, 0);
            t.clear_line();
            t.print(&self.prompt);
            t.print(">");
            t.move_cursor_to(0, 1);
            t.clear_line();
            t.print("────────────");
        }

        t.move_cursor_to(self.prompt.len() + 2, 0);
        t.print(&state.value);
        t.set_focus(self.prompt.len() + 2 + state.cursor, 0);
    }

    fn transition(&mut self, state: &InputState, event: KeyboardEvent) -> Transition<InputState> {
        match event {
            KeyboardEvent::Enter => Transition::Finished((*state).clone()),
            KeyboardEvent::CtrlC | KeyboardEvent::CtrlD => Transition::Terminate((*state).clone()),
            KeyboardEvent::Backspace => {
                let mut new_state = state.clone();
                new_state.value.pop();
                new_state.cursor -= 1;
                Transition::Updated(new_state)
            }
            // Jump to start of line
            KeyboardEvent::CtrlA => {
                let mut new_state = state.clone();
                new_state.cursor = 0;
                Transition::Updated(new_state)
            }
            // Jump to end of line
            KeyboardEvent::CtrlE => {
                let mut new_state = state.clone();
                new_state.cursor = state.value.len();
                Transition::Updated(new_state)
            }
            KeyboardEvent::AltF => {
                let mut new_state = (*state).clone();
                new_state.cursor = find_next_termpos(&state.value, state.cursor);
                Transition::Updated(new_state)
            }
            KeyboardEvent::AltB => {
                let mut new_state = (*state).clone();
                new_state.cursor = find_prev_termpos(&state.value, state.cursor);
                Transition::Updated(new_state)
            }
            // Delete prev word
            KeyboardEvent::CtrlW => {
                let mut new_state = (*state).clone();
                let mut new_cursor = find_prev_termpos(&state.value, state.cursor);
                new_state.cursor = new_cursor;
                new_state.value = format!(
                    "{}{}",
                    &state.value[0..new_cursor],
                    &state.value[state.cursor..]
                );
                Transition::Updated(new_state)
            }
            KeyboardEvent::Character(x) => {
                let mut new_state = state.clone();
                new_state.value.insert(state.cursor, x);
                new_state.cursor += 1;
                Transition::Updated(new_state)
            }
            _ => Transition::Nothing,
        }
    }
}

fn find_prev_termpos(input: &str, pos: usize) -> usize {
    let mut new_pos = pos;
    let mut iter = input[0..pos].chars().rev().peekable();
    while let Some(ch) = iter.peek() {
        if !ch.is_alphanumeric() {
            iter.next();
            new_pos -= 1;
        } else {
            break;
        }
    }

    while let Some(ch) = iter.peek() {
        if ch.is_alphanumeric() {
            iter.next();
            new_pos -= 1;
        } else {
            break;
        }
    }

    new_pos
}

fn find_next_termpos(input: &str, pos: usize) -> usize {
    let mut new_pos = pos;
    let mut iter = input[pos..].chars().peekable();
    while let Some(ch) = iter.peek() {
        if ch.is_alphanumeric() {
            iter.next();
            new_pos += 1;
        } else {
            break;
        }
    }

    while let Some(ch) = iter.peek() {
        if !ch.is_alphanumeric() {
            iter.next();
            new_pos += 1;
        } else {
            break;
        }
    }

    new_pos
}
