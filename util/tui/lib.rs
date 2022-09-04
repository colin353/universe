use raw_tty::{GuardMode, IntoRawMode};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::rc::Rc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Terminal {
    pub width: usize,
    pub height: usize,
    pub offset_x: usize,
    pub offset_y: usize,
    pos_x: usize,
    pos_y: usize,
    pub wrap: bool,
    stdout: Rc<raw_tty::TtyWithGuard<std::io::Stderr>>,
    prefix: String,
    tree: Rc<Mutex<HashMap<String, usize>>>,
    focus: Rc<Mutex<Option<(usize, usize)>>>,
}

impl Terminal {
    pub fn new() -> Self {
        let mut t = Terminal {
            width: 80,
            height: 80,
            offset_x: 0,
            offset_y: 0,
            pos_x: 0,
            pos_y: 0,
            wrap: true,
            stdout: Rc::new(std::io::stderr().guard_mode().unwrap()),
            prefix: String::new(),
            tree: Rc::new(Mutex::new(HashMap::new())),
            focus: Rc::new(Mutex::new(None)),
        };
        t.determine_terminal_size();
        t.disable_echo();
        t
    }

    pub fn derive(&self, prefix: String) -> Self {
        let mut t = self.clone();
        t.prefix += "::";
        t.prefix += &prefix;
        t
    }

    pub fn set_rendered_size(&self, size: usize) -> usize {
        self.tree.lock().unwrap().insert(self.prefix.clone(), size);
        size
    }

    pub fn get_rendered_size(&self) -> usize {
        *self.tree.lock().unwrap().get(&self.prefix).unwrap()
    }

    pub fn set_focus(&self, x: usize, y: usize) {
        *self.focus.lock().unwrap() = Some((x + self.offset_x, y + self.offset_y));
    }

    pub fn unset_focus(&self) {
        *self.focus.lock().unwrap() = None;
    }

    pub fn disable_echo(&mut self) {
        Rc::get_mut(&mut self.stdout)
            .unwrap()
            .modify_mode(|mut ios| {
                ios.c_lflag &= !0000010;
                ios
            });
    }

    pub fn determine_terminal_size(&mut self) {
        unsafe {
            let mut winsize: libc::winsize = std::mem::zeroed();

            libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ.into(), &mut winsize);
            if winsize.ws_row > 0 && winsize.ws_col > 0 {
                self.width = winsize.ws_col as usize;
                self.height = winsize.ws_row as usize;
            }
        }
    }

    pub fn clear_line(&self) {
        eprint!("\r\x1b[2K");
    }

    pub fn clear_screen(&self) {
        eprint!("\r\x1b[2J\r\x1b[H");
    }

    pub fn show_cursor(&self) {
        eprint!("\x1b[?25h");
    }

    pub fn hide_cursor(&self) {
        let esc = "\u{001B}";
        eprint!("{}[?25l", esc)
    }

    pub fn get_cursor_pos(&self) -> (usize, usize) {
        let mut tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .unwrap();
        let mut tty_input = tty.try_clone().unwrap();
        write!(tty, "\x1B[6n");

        let buf: Vec<_> = tty_input
            .bytes()
            .map(|b| b.unwrap())
            .take_while(|b| *b != 0x52)
            .collect();

        let response = std::str::from_utf8(&buf[2..buf.len()]).unwrap();
        let mut iter = response.split(";");
        let y = iter.next().unwrap().parse().unwrap();
        let x = iter.next().unwrap().parse().unwrap();
        (x, y)
    }

    pub fn move_cursor_to(&mut self, x: usize, y: usize) {
        self.pos_x = x;
        self.pos_y = y;
        eprint!("\x1B[{};{}H", y + 1 + self.offset_y, x + 1 + self.offset_x)
    }

    pub fn set_bold(&self) {
        eprint!("\x1b[{}m", 1);
    }

    pub fn set_underline(&self) {
        eprint!("\x1b[{}m", 4);
    }

    pub fn set_grey(&self) {
        eprint!("\x1b[38;5;245m");
    }

    pub fn set_normal(&self) {
        eprint!("\x1b[{}m", 0);
    }

    pub fn set_inverted(&self) {
        eprint!("\x1b[{}m", 40);
        eprint!("\x1b[{}m", 37);
    }

    pub fn flush(&self) {
        std::io::stdout().flush().unwrap()
    }

    pub fn print(&mut self, content: &str) {
        let mut has_printed = false;
        for line in content.lines() {
            if has_printed {
                self.move_cursor_to(0, self.pos_y + 1);
            }

            if self.pos_y > self.height {
                break;
            }

            let line_width = line.chars().count();
            let mut line_chars = line.chars();
            let space_left = self.width - self.pos_x;
            if line_width > space_left {
                eprint!(
                    "{}",
                    &line_chars.by_ref().take(space_left).collect::<String>()
                );
                self.move_cursor_to(0, self.pos_y + 1);
                has_printed = true;
            }

            if self.pos_y >= self.height {
                break;
            }

            if has_printed && !self.wrap {
                return;
            }

            let c: Vec<_> = line_chars.collect();
            for chunk in c.chunks(self.width) {
                eprint!("{}", chunk.iter().collect::<String>());
                self.pos_x += chunk.len();
                if self.wrap {
                    break;
                }
            }

            has_printed = true;
        }
    }
}

pub trait Component<T> {
    fn render(&mut self, term: &mut Terminal, state: &T, prev_state: Option<&T>) -> usize;
}

pub struct Container<T> {
    components: Vec<Box<dyn Component<T>>>,
}

impl<T> Container<T> {
    pub fn new(components: Vec<Box<dyn Component<T>>>) -> Self {
        Self {
            components: components,
        }
    }
}

impl<T> Component<T> for Container<T>
where
    T: PartialEq,
{
    fn render(&mut self, term: &mut Terminal, state: &T, prev_state: Option<&T>) -> usize {
        if let Some(p) = prev_state {
            if state == p {
                return term.get_rendered_size();
            }
        }

        let mut size = 0;
        for (idx, component) in self.components.iter_mut().enumerate() {
            let mut t = term.derive(format!("{}", idx));
            t.offset_y += size;
            t.height -= size;
            let offset = component.render(&mut t, state, prev_state);
            size += offset;
        }

        term.set_rendered_size(size)
    }
}

pub struct ScrollContainer<T, F, G> {
    selected: F,
    transformer: G,
    component: Box<dyn Component<T>>,
    view_position: usize,
    num_rendered_components: usize,
}

impl<T, F, G> ScrollContainer<T, F, G> {
    pub fn new(component: Box<dyn Component<T>>, transformer: G, selected: F) -> Self {
        ScrollContainer {
            selected: selected,
            transformer: transformer,
            component: component,
            view_position: 0,
            num_rendered_components: 0,
        }
    }
}

impl<S, T, F, G> Component<S> for ScrollContainer<T, F, G>
where
    T: PartialEq,
    F: Fn(&S) -> usize,
    G: Fn(&S) -> &Vec<T>,
{
    fn render(&mut self, term: &mut Terminal, state: &S, prev_state: Option<&S>) -> usize {
        let selected_index = (self.selected)(state);
        let component_state = (self.transformer)(state);

        let prev_component_state = match prev_state {
            Some(x) => Some((self.transformer)(x)),
            None => None,
        };
        let prev_selected_index = match prev_state {
            Some(x) => Some((self.selected)(x)),
            None => None,
        };

        if selected_index < self.view_position {
            self.view_position = selected_index;
        }

        if selected_index > self.view_position + self.num_rendered_components {
            self.view_position = selected_index - self.num_rendered_components;
        }

        if let Some(prev) = prev_state {
            if *prev_selected_index.as_ref().unwrap() == selected_index
                && &component_state == prev_component_state.as_ref().unwrap()
            {
                return term.height;
            }
        }

        let mut size = 0;
        let mut fully_rendered_components = 0;
        for (index, s_i) in component_state.iter().enumerate().skip(self.view_position) {
            let mut t = term.derive(format!("{}", index));
            t.offset_y += size;

            let prev_item = match prev_selected_index {
                Some(idx) if idx == selected_index => {
                    prev_component_state.as_ref().map(|s| s.get(idx)).flatten()
                }
                _ => None,
            };
            let offset = self.component.render(&mut t, s_i, prev_item);
            t.offset_y += offset;
            size += offset;
            if t.offset_y <= t.height {
                fully_rendered_components += 1;
            } else {
                break;
            }
        }

        self.num_rendered_components = fully_rendered_components;

        term.height
    }
}

pub struct VecContainer<T> {
    component: Box<dyn Component<T>>,
}

impl<T> VecContainer<T> {
    pub fn new(component: Box<dyn Component<T>>) -> Self {
        Self {
            component: component,
        }
    }
}

impl<T> Component<Vec<T>> for VecContainer<T>
where
    T: PartialEq,
{
    fn render(
        &mut self,
        term: &mut Terminal,
        state: &Vec<T>,
        prev_state: Option<&Vec<T>>,
    ) -> usize {
        if let Some(prev) = prev_state {
            if state == prev {
                return term.get_rendered_size();
            }
        }

        let mut size = 0;
        for (index, s_i) in state.iter().enumerate() {
            let mut t = term.derive(format!("{}", index));
            t.offset_y += size;
            let prev_item = prev_state.as_ref().map(|s| s.get(index)).flatten();
            let offset = self.component.render(&mut t, s_i, prev_item);
            t.offset_y += offset;
            size += offset;
        }

        term.set_rendered_size(size)
    }
}

pub struct Transformer<F, T2> {
    transformer: F,
    component: Box<dyn Component<T2>>,
}

impl<F, T2> Transformer<F, T2> {
    pub fn new(component: Box<dyn Component<T2>>, transformer: F) -> Self {
        Self {
            component: component,
            transformer: transformer,
        }
    }
}

impl<F, T1, T2> Component<T1> for Transformer<F, T2>
where
    F: Fn(&T1) -> &T2,
{
    fn render(&mut self, term: &mut Terminal, state: &T1, prev_state: Option<&T1>) -> usize {
        let transformed = (self.transformer)(state);
        self.component
            .render(term, transformed, prev_state.map(|s| (self.transformer)(s)))
    }
}

pub trait AppController<S, E> {
    fn render(&mut self, term: &mut Terminal, state: &S, prev_state: Option<&S>);
    fn initial_state(&self) -> S;
    fn transition(&mut self, state: &S, event: E) -> Transition<S>;

    fn get_terminal_size(&self) -> (usize, usize) {
        (0, 0)
    }

    fn clean_up(&self, term: &mut Terminal) {}
}

pub struct App<S, E> {
    terminal: Terminal,
    state: S,
    controller: Box<dyn AppController<S, E>>,
}

pub enum Transition<S> {
    Updated(S),
    // Terminate the program with the provided exit code
    Terminate(S),
    // Program is finished, clean up and quit
    Finished(S),
    // No state update
    Nothing,
}

impl<S, E> App<S, E>
where
    S: Clone,
{
    pub fn start(controller: Box<dyn AppController<S, E>>) -> Self {
        let mut term = Terminal::new();
        term.clear_screen();
        Self::start_with_terminal(controller, term)
    }

    pub fn start_with_terminal(controller: Box<dyn AppController<S, E>>, term: Terminal) -> Self {
        let mut app = Self {
            terminal: term,
            state: controller.initial_state(),
            controller: controller,
        };

        let terminal_size_override = app.controller.get_terminal_size();
        if terminal_size_override != (0, 0) {
            app.terminal.width = terminal_size_override.0;
            app.terminal.height = terminal_size_override.1;
        }
        app.controller.render(&mut app.terminal, &app.state, None);
        let focus = *app.terminal.focus.lock().unwrap();
        if let Some((x, y)) = focus {
            app.terminal.move_cursor_to(x, y);
            app.terminal.show_cursor();
        } else {
            app.terminal.hide_cursor();
        }
        app
    }

    pub fn handle_event(&mut self, event: E) -> Transition<S> {
        let t = self.controller.transition(&self.state, event);
        match t {
            Transition::Updated(ref new_state) => {
                self.terminal.hide_cursor();
                self.controller
                    .render(&mut self.terminal, &new_state, Some(&self.state));
                let focus = *self.terminal.focus.lock().unwrap();
                if let Some((x, y)) = focus {
                    self.terminal.move_cursor_to(x, y);
                    self.terminal.show_cursor();
                } else {
                    self.terminal.hide_cursor();
                }
                self.state = new_state.clone();
            }
            Transition::Terminate(_) => {
                self.controller.clean_up(&mut self.terminal);
            }
            Transition::Nothing => (),
            Transition::Finished(_) => {
                self.controller.clean_up(&mut self.terminal);
            }
        };

        t
    }
}

impl<S, E> Drop for App<S, E> {
    fn drop(&mut self) {
        self.controller.clean_up(&mut self.terminal)
    }
}

pub enum KeyboardEvent {
    Character(char),
    Enter,
    CtrlC,
    CtrlD,
    Backspace,
    CtrlA,
    CtrlE,
    CtrlW,
    AltF,
    AltB,
    UpArrow,
    DownArrow,
    LeftArrow,
    RightArrow,
    UnknownControl(char),
}

pub struct KeyboardEventStream<R: std::io::Read> {
    reader: R,
}

impl<R: std::io::Read> KeyboardEventStream<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: std::io::Read> Iterator for KeyboardEventStream<R> {
    type Item = KeyboardEvent;
    fn next(&mut self) -> Option<Self::Item> {
        KeyboardEvent::from_reader(&mut self.reader)
    }
}

impl KeyboardEvent {
    fn from_reader<R: std::io::Read>(reader: &mut R) -> Option<Self> {
        let mut b = reader.bytes();
        let ch: u8 = b.next()?.unwrap();
        Some(match ch.into() {
            '\n' | '\x0d' => Self::Enter,
            '\x03' => Self::CtrlC,
            '\x04' => Self::CtrlD,
            '\x7f' => Self::Backspace,
            '\x01' => Self::CtrlA,
            '\x05' => Self::CtrlE,
            '\x17' => Self::CtrlW,
            '\x17' => Self::CtrlW,
            '\x1b' => {
                // Control sequence
                let ch = b.next()?.unwrap();
                match ch.into() {
                    'f' => Self::AltF,
                    'b' => Self::AltB,
                    '[' => {
                        let ch = b.next()?.unwrap();
                        match ch.into() {
                            'C' => Self::RightArrow,
                            'D' => Self::LeftArrow,
                            'A' => Self::UpArrow,
                            'B' => Self::DownArrow,
                            _ => Self::UnknownControl('['),
                        }
                    }
                    _ => Self::UnknownControl('\x1b'),
                }
            }
            x if x.is_ascii_control() => Self::UnknownControl(x),
            x => Self::Character(x),
        })
    }
}
