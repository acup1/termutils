# Paint

A fullscreen terminal paint application demonstrating nested percentage
`termutils::Layout` regions, RGB framebuffer cells, mouse input, and popup
windows.

Run it directly from GitHub:

```sh
nix run github:acup1/termutils/examples#paint
```

Or from a checkout of the `examples` branch:

```sh
nix run .#paint
```

The terminal must be at least 80 columns by 24 rows. Left-click or drag to
paint and right-click the canvas for its context menu. The File menu and the
`O`/`S` keys open and save the custom text-based `.tpaint` format. Use the
tool and color panels with the mouse; `P`, `E`, and `F` select Pencil, Eraser,
and Fill, while `X` swaps foreground and background colors.
