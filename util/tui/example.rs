use tui::Component;

#[derive(PartialEq)]
struct AppState {
    query: String,
    filename: String,
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

struct SearchResult {}

impl SearchResult {
    pub fn new() -> Self {
        SearchResult {}
    }
}

impl Component<AppState> for SearchResult {
    fn render(
        &mut self,
        t: &mut tui::Terminal,
        state: &AppState,
        prev_state: Option<&AppState>,
    ) -> usize {
        t.move_cursor_to(0, 0);
        t.wrap = false;
        t.clear_line();
        t.move_cursor_to(0, 1);
        t.clear_line();
        t.print("1. ");
        t.print(&state.filename);
        t.move_cursor_to(0, 2);
        t.print("# script to run just executes the binary\n\n");
        t.print("cargo build \\\n");
        t.flush();
        3
    }
}

fn main() {
    let mut t = tui::Terminal::new();
    t.clear_screen();
    let state = AppState {
        query: String::from("hello"),
        filename: String::from("text 1 2 3"),
    };
    let mut s = SearchInput::new();
    let mut r = SearchResult::new();

    let mut c = tui::Container::new(vec![Box::new(s), Box::new(r)]);
    c.render(&mut t, &state, None);
}
