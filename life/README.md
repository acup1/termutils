# Life

Conway's Game of Life on an unbounded field. Every live cell is rendered as a
separate 2×3 `termutils::Window` (two terminal rows by three columns), which
looks square with typical terminal fonts. The framebuffer redraws only changed
terminal cells.

Run it directly from GitHub:

```sh
nix run github:acup1/termutils/examples#life
```

Or from a checkout of the `examples` branch:

```sh
nix run .#life
```

The application starts paused. Use the arrow keys to move the camera. Enter
toggles the cell at the yellow viewport cursor. With the mouse, left-click
toggles a cell, left-drag draws live cells, and right-click or right-drag erases
them. Space pauses or resumes simulation, and `N` advances one generation while
paused. `C` clears the field and `Q` quits.
`Shift+R` replaces the field with a random pattern covering the visible camera
area and pauses the simulation.

`S` saves the complete field to `life.world`, including cells outside the
viewport, and `L` loads it. Pass another file name after `--` to use a different
path:

```sh
nix run .#life -- patterns/glider.world
```

The terminal must be at least 30 columns by 11 rows.
