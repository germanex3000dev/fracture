use std::{
    env, fs,
    io::{self, Write},
    path::PathBuf,
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType},
};

struct Terminal;

struct Editor {
    lines: Vec<Vec<char>>,
    cx: usize,
    cy: usize,

    row_off: usize,
    col_off: usize,

    width: usize,
    height: usize,
    path: Option<PathBuf>,
    dirty: bool,
    message: String,
    quit_warn: bool,
    should_quit: bool,
}

impl Terminal {
    fn init() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(io::stdout(), terminal::EnterAlternateScreen)?;
        Ok(Terminal)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

impl Editor {
    fn open(path: Option<PathBuf>) -> io::Result<Self> {
        let mut lines: Vec<Vec<char>> = match &path {
            Some(p) if p.exists() => fs::read_to_string(p)?
                .lines()
                .map(|l| l.chars().collect())
                .collect(),
            _ => Vec::new(),
        };

        if lines.is_empty() {
            lines.push(Vec::new());
        }

        let (w, h) = terminal::size()?;
        Ok(Editor {
            lines,
            cx: 0,
            cy: 0,
            row_off: 0,
            col_off: 0,
            width: w as usize,
            height: h as usize,
            path,
            dirty: false,
            message: "Ctrl-S = save | Ctrl-Q = quit".into(),
            quit_warn: false,
            should_quit: false,
        })
    }

        fn insert_char(&mut self, c: char) {
        self.lines[self.cy].insert(self.cx, c);
        self.cx += 1;
        self.dirty = true;
    }

    fn insert_newline(&mut self) {
        // split the current line at the cursor; the right half becomes a new line
        let rest = self.lines[self.cy].split_off(self.cx);
        self.lines.insert(self.cy + 1, rest);
        self.cy += 1;
        self.cx = 0;
        self.dirty = true;
    }

    fn backspace(&mut self) {
        if self.cx > 0 {
            // delete the char to the left
            self.cx -= 1;
            self.lines[self.cy].remove(self.cx);
            self.dirty = true;
        } else if self.cy > 0 {
            // at the start of a line: glue this line onto the end of the previous one
            let line = self.lines.remove(self.cy);
            self.cy -= 1;
            self.cx = self.lines[self.cy].len(); // cursor lands at the join point
            self.lines[self.cy].extend(line);
            self.dirty = true;
        }
    }

    fn delete(&mut self) {
        if self.cx < self.lines[self.cy].len() {
            self.lines[self.cy].remove(self.cx);
            self.dirty = true;
        } else if self.cy + 1 < self.lines.len() {
            // at the end of a line: pull the next line up
            let next = self.lines.remove(self.cy + 1);
            self.lines[self.cy].extend(next);
            self.dirty = true;
        }
    }

        fn clamp_cx(&mut self) {
        // after moving up/down, the new line might be shorter
        self.cx = self.cx.min(self.lines[self.cy].len());
    }

    fn move_left(&mut self) {
        if self.cx > 0 {
            self.cx -= 1;
        } else if self.cy > 0 {
            self.cy -= 1;
            self.cx = self.lines[self.cy].len(); // wrap to end of previous line
        }
    }

    fn move_right(&mut self) {
        if self.cx < self.lines[self.cy].len() {
            self.cx += 1;
        } else if self.cy + 1 < self.lines.len() {
            self.cy += 1;
            self.cx = 0; // wrap to start of next line
        }
    }

    fn move_up(&mut self) {
        if self.cy > 0 {
            self.cy -= 1;
            self.clamp_cx();
        }
    }

    fn move_down(&mut self) {
        if self.cy + 1 < self.lines.len() {
            self.cy += 1;
            self.clamp_cx();
        }
    }

    fn page(&mut self, up: bool) {
        let step = self.text_rows();
        for _ in 0..step {
            if up { self.move_up() } else { self.move_down() }
        }
    }

        // rows available for text (we reserve 2: status bar + message line)
    fn text_rows(&self) -> usize {
        self.height.saturating_sub(2).max(1)
    }

    fn scroll(&mut self) {
        let rows = self.text_rows();
        let cols = self.width.max(1);

        if self.cy < self.row_off {
            self.row_off = self.cy; // cursor above window: scroll up
        }
        if self.cy >= self.row_off + rows {
            self.row_off = self.cy + 1 - rows; // cursor below: scroll down
        }
        if self.cx < self.col_off {
            self.col_off = self.cx;
        }
        if self.cx >= self.col_off + cols {
            self.col_off = self.cx + 1 - cols;
        }
    }

        fn draw(&mut self) -> io::Result<()> {
        self.scroll();
        let mut out = io::stdout();
        queue!(out, cursor::Hide, cursor::MoveTo(0, 0))?;

        // 1) text area
        for screen_row in 0..self.text_rows() {
            let file_row = screen_row + self.row_off;
            queue!(out, terminal::Clear(ClearType::CurrentLine))?;
            match self.lines.get(file_row) {
                Some(line) => {
                    let visible: String =
                        line.iter().skip(self.col_off).take(self.width).collect();
                    queue!(out, Print(visible))?;
                }
                None => queue!(out, Print("~"))?, // past end of file, vim style
            }
            queue!(out, Print("\r\n"))?;
        }

        // 2) status bar
        let name = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "[No Name]".into());
        let status = format!(
            " {}{}  |  line {}/{}  col {}",
            name,
            if self.dirty { " [+]" } else { "" },
            self.cy + 1,
            self.lines.len(),
            self.cx + 1,
        );
        let bar: String = format!("{:<w$}", status, w = self.width)
            .chars()
            .take(self.width)
            .collect();
        queue!(
            out,
            SetAttribute(Attribute::Reverse),
            Print(bar),
            SetAttribute(Attribute::Reset),
            Print("\r\n"),
        )?;

        // 3) message line
        let msg: String = self.message.chars().take(self.width).collect();
        queue!(out, terminal::Clear(ClearType::CurrentLine), Print(msg))?;

        // 4) put the real cursor where our logical cursor is (file coords -> screen coords)
        queue!(
            out,
            cursor::MoveTo(
                (self.cx - self.col_off) as u16,
                (self.cy - self.row_off) as u16
            ),
            cursor::Show
        )?;
        out.flush()
    }

        fn save(&mut self) {
        let Some(path) = self.path.clone() else {
            self.message = "No filename! Run: fracture <file>".into();
            return;
        };
        let text = self
            .lines
            .iter()
            .map(|l| l.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        match fs::write(&path, text + "\n") {
            Ok(_) => {
                self.dirty = false;
                self.message = format!("Saved {}", path.display());
            }
            Err(e) => self.message = format!("Save failed: {e}"),
        }
    }

    fn quit(&mut self) {
        if self.dirty && !self.quit_warn {
            self.message = "Unsaved changes! Press Ctrl-Q again to quit anyway.".into();
            self.quit_warn = true;
        } else {
            self.should_quit = true;
        }
    }

        fn handle_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // any key other than Ctrl-Q cancels the "are you sure" state
        if !(ctrl && key.code == KeyCode::Char('q')) {
            self.quit_warn = false;
        }

        match (key.code, ctrl) {
            (KeyCode::Char('q'), true) => self.quit(),
            (KeyCode::Char('s'), true) => self.save(),

            (KeyCode::Char(c), false) => self.insert_char(c),
            (KeyCode::Tab, _) => (0..4).for_each(|_| self.insert_char(' ')),
            (KeyCode::Enter, _) => self.insert_newline(),
            (KeyCode::Backspace, _) => self.backspace(),
            (KeyCode::Delete, _) => self.delete(),

            (KeyCode::Left, _) => self.move_left(),
            (KeyCode::Right, _) => self.move_right(),
            (KeyCode::Up, _) => self.move_up(),
            (KeyCode::Down, _) => self.move_down(),
            (KeyCode::Home, _) => self.cx = 0,
            (KeyCode::End, _) => self.cx = self.lines[self.cy].len(),
            (KeyCode::PageUp, _) => self.page(true),
            (KeyCode::PageDown, _) => self.page(false),

            _ => {}
        }
    }

    fn run(&mut self) -> io::Result<()> {
        while !self.should_quit {
            self.draw()?;
            match event::read()? {
                // Windows sends Press AND Release events; only handle Press
                Event::Key(k) if k.kind == KeyEventKind::Press => self.handle_key(k),
                Event::Resize(w, h) => {
                    self.width = w as usize;
                    self.height = h as usize;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let path = env::args().nth(1).map(PathBuf::from);
    let mut editor = Editor::open(path)?;
    let _term = Terminal::init()?;
    editor.run()
}
