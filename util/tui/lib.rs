use std::io::Write;

#[derive(Clone)]
pub struct Terminal {
    pub width: usize,
    pub height: usize,
    pub offset_x: usize,
    pub offset_y: usize,
    pos_x: usize,
    pos_y: usize,
    pub wrap: bool,
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
        };
        t.determine_terminal_size();
        t
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
        let esc = "\u{001B}";
        eprint!("{}[?25h", esc)
    }

    pub fn hide_cursor(&self) {
        let esc = "\u{001B}";
        eprint!("{}[?25l", esc)
    }

    pub fn move_cursor_to(&mut self, x: usize, y: usize) {
        self.pos_x = x;
        self.pos_y = y;
        eprint!("\x1B[{};{}H", y + 1 + self.offset_y, x + 1 + self.offset_x)
    }

    pub fn set_bold(&self) {
        eprint!("\x1b[{}m", 1);
    }

    pub fn set_normal(&self) {
        eprint!("\x1b[{}m", 1);
        eprint!("\x1b[{}m", 49);
        eprint!("\x1b[{}m", 30);
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
                panic!("overflow!");
            }

            let line = if line.len() > self.width - self.pos_x {
                eprint!("{}", &line[0..self.width - self.pos_x]);
                self.move_cursor_to(0, self.pos_y + 1);
                &line[self.width - self.pos_x..]
            } else {
                line
            };

            if self.pos_y > self.height {
                panic!("overflow!");
            }

            let c: Vec<_> = line.chars().collect();
            for chunk in c.chunks(self.width) {
                eprint!("{}", chunk.iter().collect::<String>());
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
    size: usize,
}

impl<T> Container<T> {
    pub fn new(components: Vec<Box<dyn Component<T>>>) -> Self {
        Self {
            components: components,
            size: 0,
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
                return self.size;
            }
        }

        let mut t = term.clone();
        self.size = 0;
        for component in &mut self.components {
            let offset = component.render(&mut t, state, prev_state);
            t.offset_y += offset;
            self.size += offset;
        }

        self.size
    }
}
