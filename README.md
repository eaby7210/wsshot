# wsshot

Screenshot an **i3 / Regolith workspace by number** from the command line, even when that workspace isn't the one you're looking at.

```
$ wsshot 11
/home/you/Pictures/wsshot-11-1794063427.png

$ wsshot 99
wsshot: workspace 99 does not exist      # exit code 1
```

## Why it switches workspaces

i3 unmaps the windows of hidden workspaces, so X holds no pixels for them
(`xwininfo` reports `IsUnviewable`; `import -window <id>` fails). The only way to
get their contents is to make the workspace visible. `wsshot` does that as fast as possible:

1. read the workspace list over i3 IPC (error out if the number doesn't exist)
2. if the workspace is already visible: just capture it, no switching
3. otherwise switch to it, wait until one of its windows is mapped, wait ~60 ms for the repaint
4. grab the workspace rect from the X root window
5. switch back to the original workspace, *then* encode the PNG

Time spent on the target workspace is roughly 0.1 s.

## Install

Needs i3 (or Regolith), an X11 session, and a Rust toolchain.

```sh
git clone https://github.com/eaby7210/wsshot
cd wsshot
cargo build --release
install -m755 target/release/wsshot ~/.local/bin/wsshot
```

## Usage

```
wsshot <workspace-number> [outfile]
```

- `workspace-number` — the i3 workspace *number* (the leading digits of its name, so Regolith names like `3:<span>…` work).
- `outfile` — default `~/Pictures/wsshot-<N>-<unix-time>.png`.
- prints the output path on success. Exit codes: `0` ok, `1` workspace missing, `2` bad usage.

Bind it in your i3 config if you like:

```
bindsym $mod+Shift+s exec --no-startup-id wsshot 2
```

## Speed

Measured on a 1920x1080 AMD laptop, 5 runs per workspace, from hidden workspace to saved PNG:

| Version | Time |
|---|---|
| `scripts/wsshot-bash` (one `jq` call) | ~320–550 ms |
| `wsshot` (Rust) | ~125–150 ms |

The bash version is dominated by starting ~15 processes (`jq`, `i3-msg`, `xwininfo`, `import`).
The Rust version talks i3 IPC and X11 directly in one process. What remains is mostly
the 60 ms repaint wait and i3's own switch.

## Also in this repo

| File | What |
|---|---|
| `scripts/wsshot-bash` | Same behaviour in bash (needs `i3-msg`, `jq`, `xwininfo`, ImageMagick `import`). No build step. |
| `scripts/wsshot-virtual` | Experimental: captures a hidden workspace **without switching your screen**, by briefly moving it to a software monitor (`xrandr --setmonitor`) and back. Works, but resizing the X framebuffer causes visible flicker, so it is not the recommended path. |

`wsshot-virtual` takes a lock file at `$XDG_RUNTIME_DIR/wsshot.lock` while the virtual
monitor exists. If you have scripts that move workspaces to "the next output" (for example
a `$mod+m` keybinding), have them exit when that file exists, otherwise they could throw
workspaces onto the temporary monitor.

## Limitations

- X11 only (no Wayland/sway).
- Captures one output's worth of pixels: the workspace's rect. Floating windows are included because they're part of the screen.
- Assumes a 32-bit BGRX root window (depth 24/32, little-endian), which is the norm.
- The 60 ms repaint wait is a constant. If you get a half-drawn frame from a slow app, raise it in `src/main.rs`.
- Screenshots contain whatever is on screen. Mind where you save them.

## License

MIT
