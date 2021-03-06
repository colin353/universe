use std::io::Read;
use tui::Component;
use tui::Transition;

#[derive(Clone, PartialEq)]
struct AppState {
    edit_mode: bool,
    query: String,
    selected: usize,
    results: Vec<SearchResult>,
}

impl AppState {
    fn update_selected(&mut self) {
        for (idx, result) in self.results.iter_mut().enumerate() {
            result.selected = !self.edit_mode && idx == self.selected;
        }
    }
}

#[derive(Clone, PartialEq)]
struct SearchResult {
    index: usize,
    filename: String,
    selected: bool,
}

impl SearchResult {
    pub fn new() -> Self {
        Self {
            index: 0,
            filename: String::new(),
            selected: false,
        }
    }
}

struct SearchInput;

impl SearchInput {
    pub fn new() -> Self {
        SearchInput {}
    }
}

impl tui::Component<AppState> for SearchInput {
    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &AppState,
        prev_state: Option<&AppState>,
    ) -> usize {
        if state.edit_mode {
            t.set_focus(19 + state.query.len(), 1);
        } else {
            t.unset_focus();
        }

        if let Some(prev) = prev_state {
            if state == prev {
                return 3;
            }

            t.move_cursor_to(19, 1);
            t.print(&state.query);
            if state.query.len() < prev.query.len() {
                t.print(
                    &(0..prev.query.len() - state.query.len())
                        .map(|_| ' ')
                        .collect::<String>(),
                );
            }
            return 3;
        }

        t.move_cursor_to(0, 0);
        t.print(&(0..t.width).map(|_| '-').collect::<String>());
        t.move_cursor_to(0, 1);
        let query_line = format!("| code search | :: {}", state.query);
        t.print(&query_line);
        t.print(
            &(0..t.width - query_line.len() - 1)
                .map(|_| ' ')
                .collect::<String>(),
        );
        t.print("|");
        t.move_cursor_to(0, 2);
        t.print(&(0..t.width).map(|_| '-').collect::<String>());
        3
    }
}

enum InputEvent {
    Keyboard(char),
}

struct SearchResultComponent {}

impl SearchResultComponent {
    pub fn new() -> Self {
        SearchResultComponent {}
    }
}

impl Component<SearchResult> for SearchResultComponent {
    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &SearchResult,
        prev_state: Option<&SearchResult>,
    ) -> usize {
        if let Some(prev) = prev_state {
            if prev == state {
                return 3;
            }
        }
        t.move_cursor_to(0, 0);
        t.wrap = false;
        t.clear_line();
        t.move_cursor_to(0, 1);
        t.clear_line();
        t.print(&format!("{}. ", state.index));
        if state.selected {
            t.set_inverted();
            t.print(&state.filename);
            t.set_normal();
        } else {
            t.print(&state.filename);
        }
        t.move_cursor_to(0, 2);
        t.flush();
        3
    }
}

fn transform<'a>(s: &'a AppState) -> &'a Vec<SearchResult> {
    &s.results
}

struct CodeContainer();
impl Component<Vec<String>> for CodeContainer {
    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &Vec<String>,
        prev_state: Option<&Vec<String>>,
    ) -> usize {
        let mut t = t.clone();
        t.offset_x += 5;
        t.width -= 10;
        t.wrap = false;
        for (idx, line) in state.iter().enumerate() {
            t.move_cursor_to(0, idx);
            t.print(line);
        }
        state.len()
    }
}

struct App {
    component: tui::Container<AppState>,
}
impl App {
    pub fn new() -> Self {
        let mut s = SearchInput::new();

        let mut r = SearchResultComponent::new();
        let mut v = tui::VecContainer::new(Box::new(r));
        let mut tr = tui::Transformer::new(Box::new(v), transform);

        let mut c = tui::Container::new(vec![Box::new(s), Box::new(tr)]);

        Self { component: c }
    }
}

impl tui::AppController<AppState, InputEvent> for App {
    fn render(
        &mut self,
        term: &mut tui::Terminal,
        state: &AppState,
        prev_state: Option<&AppState>,
    ) {
        self.component.render(term, state, prev_state);
    }

    fn transition(&mut self, state: &AppState, event: InputEvent) -> Transition<AppState> {
        match event {
            InputEvent::Keyboard('\x1B') => {
                let mut new_state = (*state).clone();
                new_state.edit_mode = false;
                Transition::Updated(new_state)
            }
            InputEvent::Keyboard('/') => {
                let mut new_state = (*state).clone();
                if state.edit_mode {
                    new_state.query.push('/');
                } else {
                    new_state.edit_mode = true;
                    new_state.update_selected();
                }
                Transition::Updated(new_state)
            }
            InputEvent::Keyboard('\x17') => {
                let mut new_state = (*state).clone();
                let index = new_state.query.rfind(' ').unwrap_or(0);
                new_state.query = (&new_state.query[0..index]).to_string();
                Transition::Updated(new_state)
            }
            InputEvent::Keyboard('\x7f') => {
                let mut new_state = (*state).clone();
                new_state.query.pop();
                Transition::Updated(new_state)
            }
            InputEvent::Keyboard('\n') => {
                if state.edit_mode {
                    let mut new_state = (*state).clone();
                    let mut sr = SearchResult::new();
                    sr.filename = new_state.query.clone();
                    sr.index = 1 + new_state.results.len();
                    sr.selected = false;
                    new_state.results.push(sr);
                    new_state.query = String::new();
                    new_state.edit_mode = false;
                    new_state.selected = 0;
                    new_state.update_selected();
                    Transition::Updated(new_state)
                } else {
                    println!("{}", state.results[state.selected].filename);
                    Transition::Terminate(state.clone())
                }
            }
            InputEvent::Keyboard('q') => {
                if !state.edit_mode {
                    Transition::Terminate(state.clone())
                } else {
                    let mut new_state = (*state).clone();
                    new_state.query.push('q');
                    Transition::Updated(new_state)
                }
            }
            InputEvent::Keyboard('j') => {
                let mut new_state = (*state).clone();
                if state.edit_mode {
                    new_state.query.push('j');
                    return Transition::Updated(new_state);
                }

                if state.selected < state.results.len() - 1 {
                    new_state.selected += 1;
                    new_state.update_selected();
                    return Transition::Updated(new_state);
                }
                Transition::Nothing
            }
            InputEvent::Keyboard('k') => {
                let mut new_state = (*state).clone();
                if state.edit_mode {
                    new_state.query.push('q');
                    return Transition::Updated(new_state);
                }

                if state.selected > 0 {
                    new_state.selected -= 1;
                    new_state.update_selected();
                    return Transition::Updated(new_state);
                }
                Transition::Nothing
            }
            InputEvent::Keyboard(c) => {
                if state.edit_mode {
                    let mut new_state = (*state).clone();
                    new_state.query.push(c);
                    return Transition::Updated(new_state);
                }
                Transition::Nothing
            }
            _ => Transition::Nothing,
        }
    }

    fn initial_state(&self) -> AppState {
        AppState {
            edit_mode: true,
            query: String::from(""),
            results: vec![],
            selected: 0,
        }
    }
}

fn main() {
    let ctrl = App::new();
    let mut app = tui::App::start(Box::new(ctrl));

    for ch in std::io::stdin().lock().bytes() {
        app.handle_event(InputEvent::Keyboard(ch.unwrap().into()));
    }
}
