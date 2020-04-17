use std::io::Read;
use tui::Component;
use tui::Transition;

#[macro_use]
extern crate flags;

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
    snippet: Vec<String>,
    snippet_starting_line: u32,
    selected: bool,
}

impl SearchResult {
    pub fn new() -> Self {
        Self {
            index: 0,
            filename: String::new(),
            snippet: Vec::new(),
            snippet_starting_line: 0,
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
            t.set_focus(17 + state.query.len(), 1);
        } else {
            t.unset_focus();
        }

        if let Some(prev) = prev_state {
            if state == prev {
                return 3;
            }

            t.move_cursor_to(17, 1);
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
        t.print("┌");
        t.print(&(0..t.width - 2).map(|_| '─').collect::<String>());
        t.print("┐");
        t.move_cursor_to(0, 1);
        let query_line = format!("│ code search │  {}", state.query);
        t.print(&query_line);
        t.print(
            &(0..t.width - query_line.len() + 3)
                .map(|_| ' ')
                .collect::<String>(),
        );
        t.print("│");
        t.move_cursor_to(0, 2);
        t.print("└");
        t.print(&(0..t.width - 2).map(|_| '─').collect::<String>());
        t.print("┘");
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
                return t.get_rendered_size();
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
        let mut size = 3;
        t.move_cursor_to(0, 2);
        t.clear_line();
        for (idx, line) in state.snippet.iter().enumerate() {
            t.set_grey();
            t.move_cursor_to(5, 3 + idx);
            t.clear_line();
            t.print(line);
            t.set_normal();
            size += 1;
        }
        t.flush();

        t.set_rendered_size(size)
    }
}

fn transform<'a>(s: &'a AppState) -> &'a Vec<SearchResult> {
    &s.results
}

fn selected_index<'a>(s: &'a AppState) -> usize {
    s.selected
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
    client: search_client::SearchClient,
    terminal_size_override: (usize, usize),
    initial_query: String,
}
impl App {
    pub fn new(client: search_client::SearchClient) -> Self {
        let mut s = SearchInput::new();

        let mut r = SearchResultComponent::new();
        let mut scroll_view = tui::ScrollContainer::new(Box::new(r), transform, selected_index);

        let mut c = tui::Container::new(vec![Box::new(s), Box::new(scroll_view)]);

        Self {
            component: c,
            client: client,
            terminal_size_override: (0, 0),
            initial_query: String::new(),
        }
    }

    fn search(&self, query: String, state: &AppState) -> AppState {
        let mut new_state = (*state).clone();

        let mut req = search_grpc_rust::SearchRequest::new();
        req.set_query(query);
        let mut results = self.client.search(req);

        new_state.results = results
            .take_candidates()
            .into_iter()
            .enumerate()
            .map(|(index, mut candidate)| {
                let mut sr = SearchResult::new();
                sr.filename = candidate.take_filename();
                sr.index = 1 + index;
                sr.snippet = candidate.take_snippet().into_iter().collect();
                sr.snippet_starting_line = candidate.get_snippet_starting_line();
                sr
            })
            .collect();

        new_state.edit_mode = false;
        new_state.selected = 0;
        new_state.update_selected();

        new_state
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
                    let new_state = self.search(state.query.clone(), state);
                    Transition::Updated(new_state)
                } else {
                    let result = &state.results[state.selected];
                    println!(
                        "{}#L{}",
                        result.filename,
                        result.snippet_starting_line
                            + (result.snippet.len() / 2 + result.snippet.len() % 2 + 1) as u32
                    );
                    Transition::Terminate(0)
                }
            }
            InputEvent::Keyboard('q') => {
                if !state.edit_mode {
                    Transition::Terminate(1)
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

                if state.results.len() > 0 && state.selected < state.results.len() - 1 {
                    new_state.selected += 1;
                    new_state.update_selected();
                    return Transition::Updated(new_state);
                }
                Transition::Nothing
            }
            InputEvent::Keyboard('k') => {
                let mut new_state = (*state).clone();
                if state.edit_mode {
                    new_state.query.push('k');
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
        let mut state = AppState {
            edit_mode: true,
            query: String::from(""),
            results: vec![],
            selected: 0,
        };
        if !self.initial_query.is_empty() {
            state.query = self.initial_query.clone();
            return self.search(self.initial_query.clone(), &state);
        }
        state
    }

    fn get_terminal_size(&self) -> (usize, usize) {
        self.terminal_size_override
    }
}

fn main() {
    let app_width = define_flag!("app_width", 0, "If specified, overrides the terminal width");
    let app_height = define_flag!(
        "app_height",
        0,
        "If specified, overrides the terminal width"
    );
    let query = define_flag!("query", String::new(), "A query to load initially");
    let host = define_flag!(
        "host",
        String::from("search.colinmerkel.xyz"),
        "The hostname of the search service"
    );
    let port = define_flag!("port", 50002, "The port of the search service");
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "The hostname of the authentication service"
    );
    let auth_port = define_flag!("auth_port", 8888, "The port of the authentication service");
    parse_flags!(
        app_width,
        app_height,
        query,
        host,
        port,
        auth_hostname,
        auth_port
    );

    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    let token = cli::load_and_check_auth(auth);

    let client = search_client::SearchClient::new_tls(&host.value(), port.value(), token);
    let mut ctrl = App::new(client);
    if app_width.value() > 0 {
        ctrl.terminal_size_override = (app_width.value(), app_height.value());
    }
    ctrl.initial_query = query.value();

    let mut app = tui::App::start(Box::new(ctrl));

    for ch in std::io::stdin().lock().bytes() {
        app.handle_event(InputEvent::Keyboard(ch.unwrap().into()));
    }
}
