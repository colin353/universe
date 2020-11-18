use sel::*;
use std::io::Read;

fn main() {
    let options = vec![
        "colin".to_string(),
        "another person".to_string(),
        "third option".to_string(),
        "none of the above".to_string(),
    ];
    let size = options.len();

    let ctrl = sel::App::new(options);

    print!("{}", (0..size + 1).map(|_| "\n").collect::<String>());
    let mut term = tui::Terminal::new();
    let (x, y) = term.get_cursor_pos();
    term.offset_y = y - size - 1;

    let mut app = tui::App::start_with_terminal(Box::new(ctrl), term);
    for ch in std::io::stdin().lock().bytes() {
        let ch: char = ch.unwrap().into();

        if ch == '\n' {
            break;
        }
        app.handle_event(ch);
    }
}
