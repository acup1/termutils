# Paint

A fullscreen terminal paint application demonstrating nested percentage
`terminalib::Layout` regions, RGB framebuffer cells, mouse input, and popup
windows.

Run it directly from GitHub:

```sh
nix run github:acup1/terminalib/examples#paint
```

Or from a checkout of the `examples` branch:

```sh
nix run .#paint
```

The terminal must be at least 80 columns by 24 rows. Left-click or drag to
paint; right-drag erases regardless of the selected tool, and middle-click
opens the canvas context menu. Fast pointer movement is joined into a solid
stroke. Click either foreground or background to open the HSV color picker;
the small swatches underneath are recent colors.

The File menu and the `O`/`S` keys open and save the custom text-based
`.tpaint` format. `P`, `E`, and `F` select Pencil, Eraser, and Fill, while `X`
swaps foreground and background colors. In the color picker, arrows choose
saturation/value, `[` and `]` change hue, Enter accepts, and Escape cancels.
