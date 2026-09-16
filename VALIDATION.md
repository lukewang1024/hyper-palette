# Prototype validation — 2026-09-16

Historical desktop observations below refer to the original dotfiles-backed
demo, before extraction. The standalone repository now uses generic fixtures.
No screenshots, personal application configurations or host credentials are
included in this repository.

## Verified locally (macOS)

- Rust 1.98.0 / Slint 1.17.1: build, 9 unit tests, strict Clippy and rustfmt pass.
- Real native window inspected using Computer Use, not a mockup.
- Root menu: 680 × 516 logical pixels, eight rows.
- Search `clip`: one result, 680 × 152; placeholder disappears.
- Enter opens the clipboard submenu in the same window: three rows, 680 × 256.
- Empty Backspace returns to the parent with `clip` restored.
- Roles submenu: more than nine items, visible panel capped at 680 × 568.
- Latest header: input/title aligned 22px from the left; 32px rounded vector
  Hyper icon on the right; current-view placeholder.
- Standalone demo emitted `dismissed` with `reason: blur` and exited on loss of
  focus. Resident mode stays alive after hide and accepts subsequent requests.
- Real resident-host protocol smoke test passed ten show/hide cycles, invalid
  request handling, stale-hide handling and orderly quit. Show acknowledgements
  ranged from 2.85–31.68 ms on this run. This is **not** frame/display latency.

Screenshots are local evidence under
`$XDG_STATE_HOME/hyper-palette/20260916/` (default `~/.local/state/…`):

- `06-final-root.jpeg`: final header and eight-row root.
- `07-nine-rows.jpeg`: final header and nine-row viewport.
- `04-search.jpeg`: filtered one-row view, preceding square-icon iteration.
- `05-submenu.jpeg`: three-row submenu, preceding square-icon iteration.

## Not yet verified

Native Windows/Linux builds or deployments, full real-action adapters, physical
Hyper chords, no-key-pass-through against a background editor, CJK composition,
pointer-hover handoff and scroll edge cases, screen-coordinate invariance during
resize, multi-monitor DPI, warm input-to-photon latency, or native shadows.

Read-only host preflight: neither validation host exposed Cargo on PATH.
The Linux host is x86_64/glibc 2.28; X11/xkbcommon pkg-config checks passed. No toolchains,
services, hotkeys, or production menus were changed on either host.
