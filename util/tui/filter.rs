use tui::{KeyboardEvent, Transition};

pub struct Filter {
    input: input::Input,
    items: Vec<String>,
    last_length: usize,
    last_height: usize,
}

impl Filter {
    pub fn new(prompt: String, items: Vec<String>) -> Self {
        Self {
            input: input::Input::new(">".to_string(), prompt, String::new()),
            items,
            last_length: 0,
            last_height: 0,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct FilterState {
    items: ItemsState,
    input_state: input::InputState,
    selected: usize,
    scroll: usize,
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

struct Query {
    terms: Vec<String>,
}

impl Query {
    fn new(s: &str) -> Self {
        Self {
            terms: s
                .split(" ")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_lowercase())
                .collect(),
        }
    }

    fn compatible(&self, data: &str) -> bool {
        for term in &self.terms {
            if !data.contains(term) {
                return false;
            }
        }
        return true;
    }
}

impl tui::AppController<FilterState, KeyboardEvent> for Filter {
    fn initial_state(&self) -> FilterState {
        FilterState {
            items: ItemsState::All,
            input_state: self.input.initial_state(),
            selected: 0,
            scroll: 0,
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
            if p.scroll == state.scroll && p.selected == state.selected && p.items == state.items {
                return;
            }
        }

        let mut iter = ItemsIterator::new(&state.items, &self.items);
        let mut count = 0;

        for (idx, item) in iter.skip(state.scroll).take(t.height - 3).enumerate() {
            count += 1;
            t.move_cursor_to(0, t.height - 3 - idx);
            t.clear_line();
            t.set_inverted();
            t.print(" ");
            if idx == state.selected {
                t.print(" ");
                t.print(item);
                t.set_normal();
            } else {
                t.set_normal();
                t.print(" ");
                t.print(item);
            }
        }

        self.last_height = t.height - 3;

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
            match event {
                KeyboardEvent::UpArrow => {
                    let mut new_state = state.clone();
                    new_state.selected += 1;
                    if new_state.selected == self.last_length {
                        new_state.selected -= 1;
                    }

                    if new_state.selected == self.last_height - 1 {
                        new_state.scroll += 1;
                    }

                    return Transition::Updated(new_state);
                }
                KeyboardEvent::DownArrow => {
                    let mut new_state = state.clone();
                    if new_state.selected == 0 {
                        if new_state.scroll > 0 {
                            new_state.scroll -= 1;
                        }
                    } else {
                        new_state.selected -= 1;
                    }
                    return Transition::Updated(new_state);
                }
                _ => (),
            }

            return match self.input.transition(&state.input_state, event) {
                Transition::Updated(new_input_state) => {
                    // Redo filtering
                    let mut new_state = (*state).clone();
                    new_state.input_state = new_input_state;
                    if new_state.input_state.value != state.input_state.value {
                        if new_state.input_state.value.is_empty() {
                            new_state.items = ItemsState::All;
                        } else {
                            let query = Query::new(&new_state.input_state.value);
                            let subset: Vec<_> = self
                                .items
                                .iter()
                                .enumerate()
                                .filter(|(_, i)| query.compatible(&i.to_lowercase()))
                                .map(|(idx, _)| idx)
                                .collect();
                            new_state.scroll = 0;
                            new_state.selected =
                                std::cmp::min(new_state.selected, subset.len() - 1);
                            new_state.items = ItemsState::Subset(subset);
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
