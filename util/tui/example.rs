use std::io::Read;
use tui::Component;

#[derive(Clone, PartialEq)]
struct AppState {
    query: String,
    filename: String,
    results: Vec<String>,
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

struct SearchResult {}

impl SearchResult {
    pub fn new() -> Self {
        SearchResult {}
    }
}

impl Component<String> for SearchResult {
    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &String,
        prev_state: Option<&String>,
    ) -> usize {
        t.move_cursor_to(0, 0);
        t.wrap = false;
        t.clear_line();
        t.move_cursor_to(0, 1);
        t.clear_line();
        t.print("1. ");
        t.print(state);
        t.move_cursor_to(0, 2);
        t.flush();
        3
    }
}

fn transform<'a>(s: &'a AppState) -> &'a Vec<String> {
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

        let mut r = SearchResult::new();
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

    fn transition(&mut self, state: &AppState, event: InputEvent) -> Option<AppState> {
        match event {
            InputEvent::Keyboard('\x7f') => {
                let mut new_state = (*state).clone();
                new_state.query.pop();
                return Some(new_state);
            }
            InputEvent::Keyboard('\n') => {
                let mut new_state = (*state).clone();
                new_state.results.push(new_state.query.clone());
                new_state.query = String::new();
                return Some(new_state);
            }
            InputEvent::Keyboard('q') => {
                std::process::exit(0);
            }
            InputEvent::Keyboard(c) => {
                let mut new_state = (*state).clone();
                new_state.query.push(c);
                return Some(new_state);
            }
            _ => None,
        }
    }

    fn initial_state(&self) -> AppState {
        AppState {
            query: String::from("hello"),
            filename: String::from("text 1 2 3"),
            results: vec![
                String::from("/util/sstable.rs"),
                String::from("/tmp/largetable.txt"),
            ],
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
