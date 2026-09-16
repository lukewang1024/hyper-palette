# Hyper Palette prototype

Standalone Rust + Slint menu host, extracted from dotfiles. **Not installed into
H+0 yet.** Existing Hammerspoon, AHK and Xfce menus remain untouched. The prototype never launches
applications or executes shell commands; it only returns IDs to its caller.

## Try it

```sh
cd hyper-palette
export CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/hyper-palette/target"
cargo run --locked -- --demo
# Optional: --lang=zh, --lang=en, --dark, --light
```

Language follows the OS preferred locale; appearance follows the window-system
theme. Generic bilingual fixtures live in `assets/demo.json`; no dotfiles files
or personal application preferences are compiled into the program. All demo
actions are **read-only previews**, not application discovery or preference editing. Selection closes the demo and
prints an ID. Re-run to open it again. No startup service or global shortcut is
registered.

On macOS, `sh macos/bundle.sh` builds a double-clickable test app under the XDG
cache directory. It is unsigned and is not installed into Applications.

## Interaction

- Search titles, details, shortcut text, and both Chinese/English aliases.
- Up/Down select, Enter opens, Esc dismisses. Empty Backspace goes back and
  restores the parent query and selection. Hyper+H/J/K/L is also handled in the
  UI when the platform delivers the modifier chord as keyboard input.
- Menus with `quick: true` use their configured bare-letter shortcuts; `/`
  enters search mode. Unmatched text also enters search; clearing the query
  stays in search mode. Submenus start fresh and Backspace restores the parent.
- By default, Alt+letter activates the indicated item without consuming ordinary search
  letters. On macOS, Option text translation can differ with keyboard layout;
  arrow navigation and search are the primary prototype controls.
- Pointer movement selects; clicking activates. Disabled items cannot select.
- At most nine rows, scroll for more. Short lists shrink down from a fixed input
  position. Zero results keeps one message row (no blank giant panel).
- Focus loss hides the palette, without reactivating the previous application.
  The platform adapter owns target-window capture and action-time restoration.
- Slint native TextInput handles text editing/IME; menu key interception is
  suppressed during Winit preedit. Real CJK candidate-window behaviour still
  needs acceptance on each platform.

## Resident protocol v1

Start **without `--demo`**. The process stays alive when its window is hidden.
The caller owns stdin/stdout pipes (not a public TCP listener). One UTF-8 JSON
object per line, max 1 MiB. No idle polling; the reader wakes the UI event loop.
EOF and `quit` terminate the host. Diagnostics go to stderr.

```json
{"type":"show","request":{"request_id":"example-1","root":"config","menus":{"config":{"title":"Hyper","items":[{"id":"clipboard.history","title":"Clipboard history","detail":"剪贴板历史","shortcut":"Alt+V","keywords":"clipboard 剪贴板"}]}}}}
{"type":"hide","request_id":"example-1"}
{"type":"quit"}
```

An item may include `disabled: true` or `submenu: "another-menu-id"`. Item IDs
must be unique within their menu. Menu graph targets are validated; navigation
depth is capped at 32. Maximum 64 menus / 4096 total items. Unknown fields are
rejected. Invalid requests leave the active menu intact. Use unique request IDs
per invocation; stale `hide` IDs are ignored. Submenus remain inside the window.

Output:

```json
{"type":"ready","protocol":1}
{"type":"shown","request_id":"example-1"}
{"type":"action","request_id":"example-1","action":"clipboard.history"}
```

Cancellation emits `dismissed` with reason `escape`, `blur`, `client`, `close`,
`replaced`, or `shutdown`. Failure emits `error` with `message` and, if available,
`request_id`. `shown` acknowledges show/focus dispatch, **not a rendered-frame
latency measurement or proof that foreground activation succeeded**.

Adapters must whitelist returned IDs against the original request and disregard
stale responses. Never interpolate an action ID into a shell command. No
clipboard content, app scanning, password handling, or window management belongs
in this process.

## Checks and remaining acceptance

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3 tests/protocol_smoke.py "$CARGO_TARGET_DIR/debug/hyper-palette"
```

The smoke test needs a desktop: it repeatedly opens/hides a real window and
measures IPC acknowledgement (not input-to-photon latency). Unit tests cover
row-height limits, bilingual filtering, disabled navigation/wrap, empty results,
parent-state restoration, malformed menus, and navigation-depth bounds.

The renderer is Winit + software, with no WebView dependency. Slint is pinned
because the Winit integration API is explicitly unstable. Windows and Linux
still require native builds and real desktop acceptance; compilation on macOS
does not establish portability. Debian 10/glibc 2.28 needs particular
attention. Wayland does not allow ordinary application-controlled placement or
focus and recreates windows on hide: this prototype's current target is X11/Xfce,
Windows and macOS, **not full Wayland parity**.

Before replacing any current menus, validate:

- Linux Xfce/VNC and native Windows builds, physical CapsLock chords;
- IME Enter/Esc, candidate-window focus, keyboard-layout-specific accelerators;
- multi-monitor working-area/DPI placement, shadows and fractional scaling;
- hover/keyboard handoff, long-list scrolling, parent selection restoration;
- actual cold/warm visual latency, no Enter passing into the previous app;
- adapter-owned foreground restoration and all real H+0 actions.

Placement currently uses the menu window's monitor, not the original target
window's monitor. Platform focus policy may deny `focus_window()`; no global
input grab or OS-specific activation workaround is added in the prototype.

## Distribution

See [RELEASING.md](RELEASING.md) for four-platform CI, immutable version tags,
archive checksums and draft Releases. The binary remains a prototype; packages
are unsigned and publishing is manual. Licensing selection is pending (see
[THIRD_PARTY.md](THIRD_PARTY.md)). There is no automatic installer, startup
registration or `latest` auto-update.

Shortcut definitions, roles and platform actions belong to the caller. There
is no submodule or build-time link back to dotfiles; the only integration
boundary is the JSONL protocol above.
