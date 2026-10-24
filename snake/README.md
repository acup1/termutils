# Snake

A terminal Snake game built against `termutils` directly from GitHub.

```sh
cargo run --manifest-path snake/Cargo.toml
```

The game uses the whole terminal. Every snake segment is rendered as its own
5×3 `termutils::Window`, which appears approximately square with typical
terminal fonts. Resizing the terminal rebuilds the grid and starts a new game.
Rendering uses the library framebuffer, so unchanged cells are not written and
adjacent changes share one cursor move.

The terminal must be at least 40 columns by 15 rows and use a Nerd Font to show
the apple icon. At startup, choose Easy, Normal, or Hard with the arrow keys and
Enter, the number keys `1`–`3`, or a mouse click. Hard keeps the original 90 ms
movement speed. During the game, use the arrow keys or WASD to move, Space to
pause, `r` to restart after a collision, and `q` to quit.
