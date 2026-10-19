# fracture

A minimal terminal text editor written in Rust.

`fracture` is a small, dependency-light editor for editing plain text files
directly in the terminal. It is built on [crossterm] and currently lives in a
single file (`src/main.rs`).

It may look plain and simple, but keep in mind, this is a first version. There will
be many features later.

> **Status: very early.** fracture is at version `0.1.0`. It opens a file, lets you
> edit it in three modes, and saves it with a handful of `:` commands — but there is
> no undo, no search, and no syntax highlighting. Expect things to break and the
> keybindings to change.
>
> The running log of what actually landed is in [Current Progress](#current-progress).

## Features

- Open an existing file, or start a new buffer by passing a path
- Modal editing: Normal mode for movement, Insert mode for text, and a `:`
  command line
- Insert, delete, and split/join lines
- Vim-style cursor movement, including wrapping across line boundaries
- Horizontal and vertical scrolling with a viewport that follows the cursor
- Status bar showing the current mode, filename, modified flag, and cursor
  position
- `:w`, `:q`, `:q!`, `:wq` and `:x` commands, with a safe default that refuses to
  quit while there are unsaved changes
- Automatic terminal restore on exit (raw mode and alternate screen are always
  cleaned up, even on panic)

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) with edition 2024 support
  (Rust 1.85 or newer)

## Build and run

```sh
cargo build --release
```
```sh
cargo run -- path/to/file.txt
```

Or install the binary onto your `PATH`:

```sh
cargo install --path .
```
```sh
fracture path/to/file.txt
```

Or you can take the binary and move/copy it manually into your path

Running `fracture` with no arguments opens an unnamed buffer, which can be
inspected but not saved — saving prompts you to start with a filename.

## Keybindings

### Normal mode

| Key                       | Action                          |
| ------------------------- | ------------------------------- |
| `i`                       | Insert before the cursor        |
| `a`                       | Insert after the cursor         |
| `A`                       | Insert at the end of the line   |
| `o` / `O`                 | Open a line below / above       |
| `:`                       | Start a command                 |
| Arrows                    | Move cursor                     |
| `Home`/`End`              | Jump to start/end of line       |
| `PageUp`/`PageDown`       | Move one screen                 |
| `Backspace`/`Delete`      | Delete left/right, join lines   |

### Insert mode

| Key                 | Action                          |
| ------------------- | ------------------------------- |
| Printable characters| Insert them                     |
| `Enter`             | Split the line at the cursor    |
| `Tab`               | Insert four spaces              |
| `Backspace`/`Delete`| Delete left/right, join lines   |
| `Esc`               | Back to Normal mode             |

Movement keys (arrows, `Home`/`End`, `PageUp`/`PageDown`) work in both modes.

## Commands

Press `:` in Normal mode to open the command line on the bottom row. Type a
command and press `Enter` to run it, `Esc` to cancel, or `Backspace` to fix a
typo.

| Command   | Action                                         |
| --------- | ---------------------------------------------- |
| `:w`      | Save the buffer                                |
| `:q`      | Quit; refuses while there are unsaved changes  |
| `:q!`     | Quit without saving                            |
| `:wq`/`:x`| Save, then quit (stays open if the save fails) |

## Current Progress

> This is the section I'll keep rewriting as things land — the running log of small steps.
> For the bigger picture, see [Features](#features) and [Commands](#commands).

### Latest(major) 19.10.2026, 13:14: the `:` command line

fracture has a command line now, and it is the only way to save or quit:

- **Commands.** `:w` saves, `:q` quits, `:q!` quits without saving, and `:wq` / `:x`
  save and then quit.
- **A live prompt.** `:` in Normal mode turns the bottom row into a prompt, and the
  command is drawn as you type it — the text cursor hides while the command line has
  focus.
- **Backspace and Esc.** `Backspace` fixes a typo, `Esc` throws the command away and
  drops you back into Normal mode.
- **Safe by default.** `:q` refuses to quit while the buffer has unsaved changes and
  tells you to use `:q!` instead — no more "press quit twice".
- **Unknown commands say so.** `:foo` reports `Unknown command: :foo` instead of
  quietly doing nothing.

### Latest(major) 19.10.2026, 11:54: modal editing, finished

The editor is properly modal now. Every mode gets its own key handling, and the old
global handler — where `Ctrl-S` saved and `Ctrl-Q` quit from anywhere — is gone:

- **Normal mode** is the default. `i` inserts before the cursor, `a` after it, `A` at
  the end of the line, `o` and `O` open a line below and above, and `:` starts a
  command.
- **Insert mode** takes printable characters, `Enter` splits the line, `Tab` inserts
  four spaces, and `Esc` goes back to Normal mode.
- **Movement works in both modes.** Arrows, `Home`/`End`, and `PageUp`/`PageDown`,
  with the same wrapping across line boundaries as before.
- **Deletion works in both modes.** `Backspace` deletes to the left and joins the
  previous line, `Delete` deletes to the right and pulls the next line up.

### Latest(small) 16.10.2026, 03:25: the status bar says which mode you are in

- **Mode indicator.** The status bar shows `NORMAL`, `INSERT`, or `COMMAND`.
- **One version string.** The banner is built from `CARGO_PKG_VERSION` instead of a
  hardcoded string, so it can't drift away from `Cargo.toml` again. We are at `0.1.0`.

### Latest(small) 10.10.2026, 00:48: the first version

The original editor: load a file, draw it, scroll with the cursor, split and join
lines, and save or quit with `Ctrl-S` / `Ctrl-Q`. Everything above replaced or
extended that.

### Known Limitations

Deliberately minimal for a first cut, but worth knowing:

- **No undo.** There is no way to take back a change yet, which is the single biggest
  gap.
- **No search, no replace, no highlighting.** The buffer is plain text.
- **No clipboard.** No cut, copy, or paste — text only moves via the keys.
- **No vim-style movement keys.** `h`/`j`/`k`/`l`, `x`, `dd` and friends are not
  implemented; movement is arrow-key based.
- **One buffer.** There is no way to open a second file without restarting, and no
  split views.
- **The command line is tiny.** No ranges, no counts (`:10` does not jump to line 10),
  and no way to run a shell command.

## License

Released under the [MIT License](LICENSE).

[crossterm]: https://github.com/crossterm-rs/crossterm
