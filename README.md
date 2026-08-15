# txtwrk

txtwrk is a simple text editor with the power of vim and emacs but with
simplicity similar to nano. It runs in your terminal.

## Install

### Auto-install (build.sh)

Compile and install `txtwrk` so you can just type `txtwrk` from anywhere:

```sh
./build.sh
```

This builds the release binary and installs it to `~/.local/bin/txtwrk`
(which is on your PATH). You can override the install directory with the
`TXTWRK_INSTALL_DIR` environment variable:

```sh
TXTWRK_INSTALL_DIR="$HOME/bin" ./build.sh
```

### Manual build

```sh
cargo build --release
# binary at target/release/txtwrk
```

## Usage

```sh
txtwrk                 # start with an empty buffer
txtwrk file.txt        # open a file (creates it on first save if missing)
txtwrk --tutorial      # open the built-in interactive tutorial (read-only)
txtwrk -t              # shorthand for --tutorial
```

## Keybindings

### NORMAL mode (default)

| Key | Action |
| --- | --- |
| `←` `→` `↑` `↓` | Move cursor |
| `C-←` `C-→` | Move backward/forward one word (Emacs-style) |
| `Home` / `End` | Beginning / end of line |
| `PageUp` / `PageDown` | One screen up / down |
| `C-G` | Goto line prompt (`T` = top, `B` = bottom, number + `Enter` = line) |
| typing | Insert text at cursor |
| `Insert` | Toggle insert / replace mode |
| `Tab` | Insert spaces (tab width configurable) |

### Selection

| Key | Action |
| --- | --- |
| `S-←` `S-→` `S-↑` `S-↓` | Select characters |
| `A-W` | Select word |
| `A-L` | Select line |
| `C-D` / `Backspace` / `Delete` | Delete selection |
| typing | Replace selection |
| `A-←` `A-→` | Move selection left/right (swap with neighbor word) |
| `A-↑` `A-↓` | Move selection up/down (swap with neighbor line) |

### Finding

| Key | Action |
| --- | --- |
| `C-F` | Enter FIND mode, type a pattern |
| `Enter` | Jump to next match (wraps around) |
| `C-F` | Start a new search |
| `Esc` | Clear the query and return to NORMAL |

### Files

| Key | Action |
| --- | --- |
| `C-S` | Save (fails on read-only buffers; falls back to save-as if unnamed) |
| `CA-S` | Save as... |
| `C-N` | New empty file |
| `C-O` | OPEN mode (file browser) |

### OPEN mode

| Key | Action |
| --- | --- |
| `Enter` | Enter directory / open file |
| `Backspace` | Go to parent directory |
| `C-R` | Rename selected file/dir |
| `Delete` | Delete selected file/dir (asks for confirmation) |
| `Esc` | Back to NORMAL |

### SHELL mode

| Key | Action |
| --- | --- |
| `C-X` | Enter SHELL mode, type a command |
| `Enter` | Run command via `sh -c`, insert stdout+stderr at cursor |
| `Esc` | Cancel |

### Quitting

| Key | Action |
| --- | --- |
| `C-Q` | Quit (asks for confirmation) |

## Configuration

txtwrk reads `~/.config/txtwrk/config.toml`. Example:

```toml
tab_width = 4

[theme]
fg = "White"
bg = "Black"
selection_fg = "Black"
selection_bg = "Cyan"
status_fg = "Black"
status_bg = "White"
match_fg = "Black"
match_bg = "Yellow"

[bindings]
move_left = "left"
move_right = "right"
move_up = "up"
move_down = "down"
select_left = "s-left"
select_right = "s-right"
select_up = "s-up"
select_down = "s-down"
word_forward = "c-right"
word_backward = "c-left"
line_start = "home"
line_end = "end"
page_up = "pageup"
page_down = "pagedown"
goto = "c-g"
backspace = "backspace"
delete = "delete"
insert_toggle = "insert"
select_word = "a-w"
select_line = "a-l"
move_text_left = "a-left"
move_text_right = "a-right"
move_text_up = "a-up"
move_text_down = "a-down"
find = "c-f"
save = "c-s"
save_as = "ca-s"
new_file = "c-n"
open = "c-o"
shell = "c-x"
quit = "c-q"
```

Key spec syntax: modifiers `c-` (ctrl), `a-` (alt), `s-` (shift), combined
like `ca-s`. Keys: `left`, `right`, `up`, `down`, `home`, `end`, `pageup`,
`pagedown`, `insert`, `delete`, `backspace`, `enter`, `esc`, `tab`, `space`,
`f1`..`f12`, or a single character.

## Architecture

- `src/gap.rs` — gap buffer data model (O(1) insert/delete at cursor)
- `src/buffer.rs` — cursor, selection, word/line ops, find, save/load
- `src/app.rs` — mode state machine and key dispatch
- `src/config.rs` — keybinding/theme config
- `src/ui.rs` — ratatui rendering
- `assets/tutorial.txt` — bundled interactive tutorial

## Tests

```sh
cargo test
```
