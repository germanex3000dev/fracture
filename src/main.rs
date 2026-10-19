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

const VERSION: &str = env!("CARGO_PKG_VERSION");
const INDENT: usize = 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Normal,
    Insert,
    Command,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
            Mode::Command => "COMMAND",
        }
    }
}

fn default_message() -> String {
    format!("fracture v{VERSION} — press i to insert, : for commands")
}

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
    should_quit: bool,
    mode: Mode,
    command: String,
}

impl Terminal {
    fn init() -> io::Result<Self> {
        terminal::enable_raw_mode()?;

        if let Err(e) = execute!(io::stdout(), terminal::EnterAlternateScreen) {
            let _ = terminal::disable_raw_mode();
            return Err(e);
        }

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
            message: default_message(),
            should_quit: false,
            mode: Mode::Normal,
            command: String::new(),
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

    /// Insert an empty line below the cursor and drop into Insert mode on it.
    fn open_line_below(&mut self) {
        self.lines.insert(self.cy + 1, Vec::new());
        self.cy += 1;
        self.cx = 0;
        self.dirty = true;
        self.mode = Mode::Insert;
    }

    /// Insert an empty line above the cursor and drop into Insert mode on it.
    fn open_line_above(&mut self) {
        self.lines.insert(self.cy, Vec::new());
        self.cx = 0;
        self.dirty = true;
        self.mode = Mode::Insert;
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

    fn move_line_start(&mut self) {
        self.cx = 0;
    }

    fn move_line_end(&mut self) {
        self.cx = self.lines[self.cy].len();
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

        // 2) status bar: mode, filename, modified flag, cursor position
        let name = self
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "[No Name]".into());
        let status = format!(
            " -- {} --  {}{}  |  line {}/{}  col {}",
            self.mode.label(),
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

        // 3) message line — in Command mode this is the ':' prompt we type into
        let line = if self.mode == Mode::Command {
            format!(":{}", self.command)
        } else {
            self.message.clone()
        };
        let total = line.chars().count();
        let msg: String = if total > self.width {
            // too long to fit: drop the oldest characters so the tail stays visible
            line.chars()
                .skip(total - self.width)
                .take(self.width)
                .collect()
        } else {
            line
        };
        queue!(out, terminal::Clear(ClearType::CurrentLine), Print(msg))?;

        // 4) put the real cursor where our logical cursor is (file coords -> screen coords)
        if self.mode == Mode::Command {
            queue!(out, cursor::Hide)?; // no text cursor while typing a command
        } else {
            queue!(
                out,
                cursor::MoveTo(
                    (self.cx - self.col_off) as u16,
                    (self.cy - self.row_off) as u16
                ),
                cursor::Show
            )?;
        }
        out.flush()
    }

    /// Write the buffer to disk. Returns whether the file was actually written.
    fn save(&mut self) -> bool {
        let Some(path) = self.path.clone() else {
            self.message = "No filename! Run: fracture <file>".into();
            return false;
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
                true
            }
            Err(e) => {
                self.message = format!("Save failed: {e}");
                false
            }
        }
    }

    /// Quit the editor. Without `force` a dirty buffer blocks the quit.
    fn quit(&mut self, force: bool) {
        if self.dirty && !force {
            self.message = "No write since last change (use :q! to quit anyway)".into();
        } else {
            self.should_quit = true;
        }
    }

    /// Execute whatever was typed after the ':'.
    fn run_command(&mut self, raw: &str) {
        match raw.trim() {
            "" => {} // bare ':' just leaves command mode
            "w" => {
                self.save();
            }
            "q" => self.quit(false),
            "q!" => self.quit(true),
            // only quit when the write actually landed, otherwise keep the error
            "wq" | "x" => {
                if self.save() {
                    self.quit(false);
                }
            }
            other => self.message = format!("Unknown command: :{other}"),
        }
    }

    /// Leave Command mode without running anything.
    fn cancel_command(&mut self) {
        self.command.clear();
        self.mode = Mode::Normal;
        self.message = default_message();
    }

    fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Normal => self.normal_key(key),
            Mode::Insert => self.insert_key(key),
            Mode::Command => self.command_key(key),
        }
    }

    /// Movement keys, shared by Normal and Insert mode.
    fn move_cursor(&mut self, code: KeyCode) {
        match code {
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Home => self.move_line_start(),
            KeyCode::End => self.move_line_end(),
            KeyCode::PageUp => self.page(true),
            KeyCode::PageDown => self.page(false),
            _ => {}
        }
    }

    fn normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('i') => self.mode = Mode::Insert,
            KeyCode::Char('a') => {
                // insert *after* the cursor, but never wrap onto the next line
                if self.cx < self.lines[self.cy].len() {
                    self.cx += 1;
                }
                self.mode = Mode::Insert;
            }
            KeyCode::Char('A') => {
                self.move_line_end();
                self.mode = Mode::Insert;
            }
            KeyCode::Char('o') => self.open_line_below(),
            KeyCode::Char('O') => self.open_line_above(),
            KeyCode::Char(':') => {
                self.mode = Mode::Command;
                self.command.clear();
            }
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            code => self.move_cursor(code),
        }
    }

    fn insert_key(&mut self, key: KeyEvent) {
        // never let Ctrl-<key> slip in as a literal character
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return;
        }

        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char(c) => self.insert_char(c),
            KeyCode::Enter => self.insert_newline(),
            KeyCode::Tab => (0..INDENT).for_each(|_| self.insert_char(' ')),
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete(),
            code => self.move_cursor(code),
        }
    }

    fn command_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return;
        }

        match key.code {
            KeyCode::Esc => self.cancel_command(),
            KeyCode::Enter => {
                let command = std::mem::take(&mut self.command);
                self.mode = Mode::Normal;
                self.run_command(&command);
            }
            KeyCode::Backspace => {
                self.command.pop();
            }
            KeyCode::Char(c) => self.command.push(c),
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
