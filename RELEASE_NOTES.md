# Standalone palette prototype

Resident Rust/Slint command palette with a versioned JSONL interface, bilingual
demo, search, menu history, hover, dynamic nine-row viewport and blur dismissal.

This is a renderer prototype, not a global hotkey manager. Real application and
window actions are supplied by external platform adapters. Existing dotfiles
menus are not replaced automatically.

Assets: macOS ARM64 / x86_64, Windows x64, Linux x64 (glibc 2.28 build baseline).
All archives include a manifest; check SHA256SUMS before extracting. Build and
unit checks do not establish physical-key or IME compatibility on every desktop.

Unsigned and not notarized. macOS packages include a demo .app plus the resident
CLI host. Windows packages contain an .exe, not an installer. Linux requires X11
libraries, fontconfig and installed fonts. Full Wayland parity is not supported.

This release is created as a draft. Complete licensing review, native desktop
acceptance and signing policy before publishing or marking it stable.
