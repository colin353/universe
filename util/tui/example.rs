use tui::Component;

#[derive(PartialEq)]
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

fn main() {
    let mut t = tui::Terminal::new();
    t.clear_screen();
    let state = AppState {
        query: String::from("hello"),
        filename: String::from("text 1 2 3"),
        results: vec![
            String::from("/util/sstable.rs"),
            String::from("/tmp/largetable.txt"),
        ],
    };
    let mut s = SearchInput::new();

    let mut r = SearchResult::new();
    let mut v = tui::VecContainer::new(Box::new(r));
    let mut tr = tui::Transformer::new(Box::new(v), transform);

    let mut c = tui::Container::new(vec![Box::new(s), Box::new(tr)]);
    c.render(&mut t, &state, None);
}
