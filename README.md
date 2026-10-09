# fracture

A minimal terminal text editor written in Rust.

`fracture` is a small, dependency-light editor for editing plain text files
directly in the terminal. It is built on [crossterm] and currently lives in a
single file (`src/main.rs`).

## Features

- Open an existing file, or start a new buffer by passing a path
- Insert, delete, and split/join lines
- Vim-style cursor movement, including wrapping across line boundaries
- Horizontal and vertical scrolling with a viewport that follows the cursor
- Status bar showing the filename, modified flag, and cursor position
- Unsaved-changes warning on quit
- Automatic terminal restore on exit (raw mode and alternate screen are always
  cleaned up, even on panic)

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) with edition 2024 support
  (Rust 1.85 or newer)

## Build and run

```sh
cargo build --release
cargo run -- path/to/file.txt
```

Or install the binary onto your `PATH`:

```sh
cargo install --path .
fracture path/to/file.txt
```

Running `fracture` with no arguments opens an unnamed buffer, which can be
inspected but not saved — saving prompts you to start with a filename.

## Keybindings

| Key           | Action                          |
| ------------- | ------------------------------- |
| `Ctrl-S`      | Save                            |
| `Ctrl-Q`      | Quit (press twice if unsaved)   |
| Arrows        | Move cursor                     |
| `PageUp/Down` | Move one screen                 |
| `Home`/`End`  | Jump to start/end of line       |
| `Enter`       | Split the line at the cursor    |
| `Backspace`   | Delete left, join with prev line|
| `Delete`      | Delete right, join with next    |
| `Tab`         | Insert four spaces              |

## License

Released under the [MIT License](LICENSE).

[crossterm]: https://github.com/crossterm-rs/crossterm