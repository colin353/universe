use tui::{KeyboardEvent, Transition};

pub struct Filter {
    input: input::Input,
    items: Vec<String>,
    last_length: usize,
}

impl Filter {
    pub fn new(prompt: String, items: Vec<String>) -> Self {
        Self {
            input: input::Input::new(">".to_string(), prompt, String::new()),
            items,
            last_length: 0,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct FilterState {
    items: ItemsState,
    input_state: input::InputState,
}

#[derive(Clone, PartialEq)]
enum ItemsState {
    All,
    Subset(Vec<usize>),
}

struct ItemsIterator<'a> {
    state: &'a ItemsState,
    items: &'a [String],
    pos: usize,
}

impl<'a> ItemsIterator<'a> {
    fn new(state: &'a ItemsState, items: &'a [String]) -> Self {
        Self {
            state,
            items,
            pos: 0,
        }
    }

    fn len(&self) -> usize {
        match self.state {
            ItemsState::All => self.items.len(),
            ItemsState::Subset(s) => s.len(),
        }
    }
}

impl<'a> Iterator for ItemsIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            ItemsState::All => {
                self.pos += 1;
                Some(self.items.get(self.pos - 1)?.as_str())
            }
            ItemsState::Subset(offsets) => {
                self.pos += 1;
                Some(self.items[*offsets.get(self.pos - 1)?].as_str())
            }
        }
    }
}

impl tui::AppController<FilterState, KeyboardEvent> for Filter {
    fn initial_state(&self) -> FilterState {
        FilterState {
            items: ItemsState::All,
            input_state: self.input.initial_state(),
        }
    }

    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &FilterState,
        prev_state: Option<&FilterState>,
    ) {
        let mut input_t = t.derive(String::new());
        input_t.offset_y = t.height - 2;
        input_t.height = 2;
        self.input.render(
            &mut input_t,
            &state.input_state,
            prev_state.map(|s| &s.input_state),
        );

        // If the items list didn't change, don't re-render
        if let Some(p) = prev_state {
            if p.items == state.items {
                return;
            }
        }

        let mut iter = ItemsIterator::new(&state.items, &self.items);
        let count = iter.len();

        for (idx, item) in iter.take(t.height - 3).enumerate() {
            t.move_cursor_to(0, t.height - 3 - idx);
            t.clear_line();
            t.print(" ");
            t.print(item);
        }

        if self.last_length > count {
            for idx in count..self.last_length {
                t.move_cursor_to(0, t.height - 3 - idx);
                t.clear_line();
            }
        }
        self.last_length = count;
    }

    fn transition(&mut self, state: &FilterState, event: KeyboardEvent) -> Transition<FilterState> {
        if state.input_state.focused {
            return match self.input.transition(&state.input_state, event) {
                Transition::Updated(new_input_state) => {
                    // Redo filtering
                    let mut new_state = (*state).clone();
                    new_state.input_state = new_input_state;
                    if new_state.input_state.value != state.input_state.value {
                        if new_state.input_state.value.is_empty() {
                            new_state.items = ItemsState::All;
                        } else {
                            new_state.items = ItemsState::Subset(
                                self.items
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, i)| i.starts_with(&new_state.input_state.value))
                                    .map(|(idx, _)| idx)
                                    .collect(),
                            );
                        }
                    }

                    Transition::Updated(new_state)
                }
                Transition::Terminate(_) => Transition::Terminate(state.clone()),
                Transition::Nothing => Transition::Nothing,
                Transition::Finished(_) => Transition::Finished(state.clone()),
            };
        }
        Transition::Nothing
    }
}
