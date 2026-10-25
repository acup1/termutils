# Terminal graphics protocol

An interactive Kitty graphics protocol demo for `terminalib`. It actively
checks protocol support before sending image commands and demonstrates:

- animated per-pixel RGB matrices with in-place root-frame replacement;
- a one-color RGBA mask with a transparent background;
- a PNG-backed terminal image object.

Run the built-in sample:

```sh
nix run .#tgp
```

Pass a PNG file to display it instead of the built-in sample:

```sh
nix run .#tgp -- path/to/image.png
```

Press `q` to quit. The terminal must support the Kitty graphics protocol and be
at least 68 columns by 15 rows.
