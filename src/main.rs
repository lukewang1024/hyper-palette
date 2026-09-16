#[cfg(target_os = "linux")]
mod linux_theme;
mod model;

use model::{Request, State};
use serde::Deserialize;
use serde_json::json;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{
  cell::RefCell,
  io::{self, BufRead, Read, Write},
  rc::Rc,
};

slint::include_modules!();

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
  Show { request: Request },
  Push { request: Request },
  Navigate { request_id: String, action: String },
  Hide { request_id: String },
  Quit,
}

#[derive(Default)]
struct Runtime {
  state: Option<State>,
  focused: bool,
  anchor: Option<slint::PhysicalPosition>,
  theme_override: Option<bool>,
}
type Shared = Rc<RefCell<Runtime>>;

fn resolved_theme(explicit: Option<bool>, native: Option<bool>, current: bool) -> bool {
  explicit.or(native).unwrap_or(current)
}

#[cfg(test)]
mod theme_tests {
  use super::resolved_theme;

  #[test]
  fn native_theme_arriving_after_window_creation_replaces_initial_default() {
    let before_window = resolved_theme(None, None, false);
    assert!(resolved_theme(None, Some(true), before_window));
  }

  #[test]
  fn unknown_theme_does_not_reset_dark_and_reopening_tracks_both_directions() {
    assert!(resolved_theme(None, None, true));
    assert!(!resolved_theme(None, Some(false), true));
    assert!(resolved_theme(None, Some(true), false));
  }

  #[test]
  fn explicit_theme_wins_over_system_events() {
    assert!(!resolved_theme(Some(false), Some(true), true));
    assert!(resolved_theme(Some(true), Some(false), false));
  }
}

fn sync_theme(ui: &Palette, rt: &Shared) {
  // Quick menu keys are commands, not IME composition. Enable composition
  // only after entering search so Chinese input cannot swallow bare letters.
  ui.window()
    .with_winit_window(|window| window.set_ime_allowed(!ui.get_quick()));
  #[cfg(target_os = "macos")]
  clip_native_corners(ui);
  // Before the event loop starts, Slint may not have a native window yet.
  // Unknown is not Light: preserve the last value until the window is ready.
  let native = ui
    .window()
    .with_winit_window(|w| w.theme())
    .flatten()
    .map(|theme| theme == winit::window::Theme::Dark);
  ui.set_dark(resolved_theme(
    rt.borrow().theme_override,
    native,
    ui.get_dark(),
  ));
}

#[cfg(target_os = "macos")]
fn clip_native_corners(ui: &Palette) {
  use objc2::{MainThreadMarker, msg_send, rc::Retained, runtime::AnyObject};
  use objc2_quartz_core::CALayer;
  use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

  // Softbuffer's CoreGraphics surface uses NoneSkipFirst (no alpha).
  // Clip its opaque sublayer at the native view boundary, in logical points.
  // The layer mask automatically follows bounds changes as results resize.
  let Some(_main_thread) = MainThreadMarker::new() else {
    return;
  };
  ui.window().with_winit_window(|window| {
    let Ok(handle) = window.window_handle() else {
      return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
      return;
    };
    // SAFETY: winit lends a live NSView on the UI/main thread for this closure.
    let view: &AnyObject = unsafe { handle.ns_view.cast().as_ref() };
    let layer: Option<Retained<CALayer>> = unsafe { msg_send![view, layer] };
    if let Some(layer) = layer {
      layer.setCornerRadius(ui.get_corner_radius() as f64);
      layer.setMasksToBounds(true);
    }
  });
}

fn emit(value: serde_json::Value) {
  let mut out = io::stdout().lock();
  let _ = writeln!(out, "{value}");
  let _ = out.flush();
}

fn render(ui: &Palette, rt: &Runtime, zh: bool, scroll_to_selection: bool) {
  let Some(state) = &rt.state else {
    return;
  };
  let rows = state.rows();
  ui.set_rows(ModelRc::new(VecModel::from(
    rows
      .iter()
      .map(|item| Row {
        title: item.title.clone().into(),
        detail: item.detail.clone().into(),
        shortcut: item.shortcut.clone().into(),
        disabled: item.disabled,
        submenu: item.submenu.is_some() || item.navigate,
      })
      .collect::<Vec<_>>(),
  )));
  ui.set_query(state.query().into());
  ui.set_quick(state.quick());
  ui.window()
    .with_winit_window(|window| window.set_ime_allowed(!state.quick()));
  ui.set_heading(state.title().into());
  ui.set_selected(state.selected().map(|i| i as i32).unwrap_or(-1));
  ui.set_empty_label(if zh { "没有匹配项" } else { "No matches" }.into());
  ui.set_footer(
    format!(
      "{}  ·  {}",
      if state.quick() && zh {
        "字母 快捷操作   / 搜索   Esc 关闭"
      } else if state.quick() {
        "Letter Quick action   / Search   Esc Dismiss"
      } else if zh {
        "↑↓ 选择   ↵ 确认   Esc 关闭"
      } else {
        "↑↓ Select   ↵ Open   Esc Dismiss"
      },
      if state.depth() > 0 {
        if zh {
          "空输入 ⌫ 返回"
        } else {
          "Empty ⌫ Back"
        }
      } else {
        "Hyper"
      }
    )
    .into(),
  );
  ui.set_count_label(
    if zh {
      format!("{} 项", rows.len())
    } else {
      format!("{} results", rows.len())
    }
    .into(),
  );
  let viewport = 52.0 * rows.len().clamp(1, model::MAX_ROWS) as f32;
  let min_y = (viewport - rows.len() as f32 * 52.0).min(0.0);
  let mut y = ui.get_scroll_y().clamp(min_y, 0.0);
  if scroll_to_selection && let Some(index) = state.selected() {
    let top = index as f32 * 52.0;
    if top + y < 0.0 {
      y = -top;
    }
    if top + 52.0 + y > viewport {
      y = viewport - top - 52.0;
    }
  }
  ui.set_scroll_y(y);
  // Binding-driven resizes may be centred by a window manager. Preserve top-left.
  ui.window().set_size(slint::LogicalSize::new(
    680.0,
    model::height(rows.len()) as f32,
  ));
  if let Some(anchor) = rt.anchor {
    ui.window().set_position(anchor);
  }
}

fn finish(ui: &Palette, rt: &Shared, reason: &str, action: Option<String>) {
  let state = {
    let mut rt = rt.borrow_mut();
    rt.focused = false;
    rt.anchor = None;
    rt.state.take()
  };
  if let Some(state) = state {
    let _ = ui.hide();
    ui.set_composing(false);
    emit(match action {
      Some(action) => {
        json!({"type":"action", "request_id":state.request.request_id, "action":action})
      }
      None => json!({"type":"dismissed", "request_id":state.request.request_id, "reason":reason}),
    });
  }
}

fn activate(ui: &Palette, rt: &Shared, zh: bool) {
  if ui.get_composing() {
    return;
  }
  let keep_open = rt.borrow().state.as_ref().is_some_and(State::keep_open);
  let action = rt.borrow_mut().state.as_mut().and_then(State::activate);
  if keep_open && let Some(action) = action {
    let id = rt
      .borrow()
      .state
      .as_ref()
      .unwrap()
      .request
      .request_id
      .clone();
    emit(json!({"type":"action", "request_id":id, "action":action, "keep_open":true}));
    return;
  }
  if action.is_some() {
    finish(ui, rt, "action", action);
  } else {
    render(ui, &rt.borrow(), zh, true);
    ui.invoke_focus_input();
  }
}

fn show(ui: &Palette, rt: &Shared, request: Request, zh: bool) {
  let id = request.request_id.clone();
  let state = match State::new(request) {
    Ok(state) => state,
    Err(error) => {
      emit(json!({"type":"error", "request_id":id, "message":error}));
      return;
    }
  };
  finish(ui, rt, "replaced", None);
  rt.borrow_mut().state = Some(state);
  ui.set_scroll_y(0.0);
  render(ui, &rt.borrow(), zh, true);
  if let Err(error) = ui.show() {
    emit(json!({"type":"error", "request_id":id, "message":error.to_string()}));
    rt.borrow_mut().state = None;
    return;
  }
  sync_theme(ui, rt);
  ui.window().with_winit_window(|window| {
    // The invoking adapter can later supply its target-monitor anchor. For now
    // use this window's monitor and pin its top edge at the upper quarter.
    if let Some(monitor) = window.current_monitor() {
      let scale = window.scale_factor();
      let pos = monitor.position();
      let size = monitor.size();
      let x = pos.x + ((size.width as f64 - 680.0 * scale) / 2.0).max(0.0) as i32;
      let y = pos.y + (size.height as f64 * 0.20) as i32;
      let anchor = slint::PhysicalPosition::new(x, y);
      ui.window().set_position(anchor);
      rt.borrow_mut().anchor = Some(anchor);
    }
    window.focus_window();
  });
  ui.invoke_focus_input();
  emit(json!({"type":"shown", "request_id":id}));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.iter().any(|arg| arg == "--help") {
    println!(
      "hyper-palette [--demo | --dump-demo] [--lang=zh|en] [--dark|--light]\nWithout --demo: resident JSONL stdin/stdout host. No actions are executed."
    );
    return Ok(());
  }
  for arg in &args {
    if ![
      "--demo",
      "--dump-demo",
      "--lang=zh",
      "--lang=en",
      "--dark",
      "--light",
    ]
    .contains(&arg.as_str())
    {
      return Err(format!("unknown argument: {arg}").into());
    }
  }
  let zh = if args.contains(&"--lang=zh".into()) {
    true
  } else if args.contains(&"--lang=en".into()) {
    false
  } else {
    sys_locale::get_locale()
      .unwrap_or_default()
      .to_lowercase()
      .starts_with("zh")
  };
  if args.contains(&"--dump-demo".into()) {
    emit(json!({"type":"show", "request":model::demo(zh)}));
    return Ok(());
  }
  slint::BackendSelector::new()
    .backend_name("winit".into())
    .renderer_name("software".into())
    .select()?;
  let ui = Palette::new()?;
  let rt = Shared::default();
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_search(move |query| {
    let ui = weak.unwrap();
    if let Some(state) = &mut cloned.borrow_mut().state {
      state.search(query.to_string());
    }
    ui.set_scroll_y(0.0);
    render(&ui, &cloned.borrow(), zh, true);
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_navigate(move |delta| {
    if let Some(state) = &mut cloned.borrow_mut().state {
      state.step(delta.signum());
    }
    render(&weak.unwrap(), &cloned.borrow(), zh, true);
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_hover(move |index| {
    if let Some(state) = &mut cloned.borrow_mut().state {
      state.select(index as usize);
    }
    // Do not rebuild the model under a stationary pointer.
    if let Some(state) = &cloned.borrow().state {
      weak
        .unwrap()
        .set_selected(state.selected().map(|i| i as i32).unwrap_or(-1));
    }
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_choose(move |index| {
    if let Some(state) = &mut cloned.borrow_mut().state {
      state.select(index as usize);
    }
    activate(&weak.unwrap(), &cloned, zh);
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_activate(move || activate(&weak.unwrap(), &cloned, zh));
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_back(move || {
    if let Some(state) = &mut cloned.borrow_mut().state {
      state.back();
    }
    render(&weak.unwrap(), &cloned.borrow(), zh, true);
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_dismiss(move || finish(&weak.unwrap(), &cloned, "escape", None));
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_accelerator(move |key| {
    let mut rt = cloned.borrow_mut();
    let Some(state) = &mut rt.state else {
      return;
    };
    let shortcut = format!("Alt+{}", key.to_uppercase());
    let index = state
      .rows()
      .iter()
      .position(|item| !item.disabled && item.shortcut == shortcut);
    if let Some(index) = index {
      state.select(index);
    }
    drop(rt);
    if index.is_some() {
      activate(&weak.unwrap(), &cloned, zh);
    }
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_quick_key(move |key| {
    let matched = cloned
      .borrow_mut()
      .state
      .as_mut()
      .is_some_and(|state| state.quick_key(&key));
    if matched {
      activate(&weak.unwrap(), &cloned, zh);
    }
    matched
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.on_begin_search(move || {
    if let Some(state) = &mut cloned.borrow_mut().state {
      state.begin_search();
    }
    render(&weak.unwrap(), &cloned.borrow(), zh, false);
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  let explicit_theme = args.iter().any(|arg| arg == "--dark" || arg == "--light");
  ui.window().on_winit_window_event(move |_, event| {
    let Some(ui) = weak.upgrade() else {
      return EventResult::Propagate;
    };
    use winit::event::{Ime, WindowEvent};
    match event {
      WindowEvent::Focused(true) => {
        cloned.borrow_mut().focused = true;
        sync_theme(&ui, &cloned);
      }
      WindowEvent::RedrawRequested => sync_theme(&ui, &cloned),
      WindowEvent::Focused(false) => {
        if cloned.borrow().focused {
          finish(&ui, &cloned, "blur", None);
        }
      }
      WindowEvent::Ime(Ime::Preedit(text, _)) => ui.set_composing(!text.is_empty()),
      WindowEvent::Ime(Ime::Commit(_) | Ime::Disabled) => ui.set_composing(false),
      WindowEvent::ThemeChanged(theme) if !explicit_theme => {
        ui.set_dark(*theme == winit::window::Theme::Dark)
      }
      _ => {}
    }
    EventResult::Propagate
  });
  let weak = ui.as_weak();
  let cloned = rt.clone();
  ui.window().on_close_requested(move || {
    finish(&weak.unwrap(), &cloned, "close", None);
    slint::CloseRequestResponse::KeepWindowShown
  });
  // Create and reuse the native window, including while the palette is hidden.
  ui.show()?;
  rt.borrow_mut().theme_override = explicit_theme.then(|| args.contains(&"--dark".into()));
  sync_theme(&ui, &rt);
  ui.hide()?;
  #[cfg(target_os = "linux")]
  let _theme_watchers = (!explicit_theme).then(|| linux_theme::watch(ui.as_weak()));
  rt.borrow_mut().focused = false;
  if args.contains(&"--demo".into()) {
    show(&ui, &rt, model::demo(zh), zh);
  } else {
    let weak = ui.as_weak();
    let cloned = rt.clone();
    ui.on_dispatch_command(move |frame| {
      let ui = weak.unwrap();
      match serde_json::from_str::<Command>(&frame) {
        Ok(Command::Show { request }) => show(&ui, &cloned, request, zh),
        Ok(Command::Push { request }) => {
          let id = request.request_id.clone();
          let result = cloned
            .borrow_mut()
            .state
            .as_mut()
            .ok_or("no active session")
            .and_then(|state| state.push(request));
          match result {
            Ok(()) => {
              render(&ui, &cloned.borrow(), zh, true);
              ui.invoke_focus_input();
            }
            Err(error) => emit(json!({"type":"error", "request_id":id, "message":error})),
          }
        }
        Ok(Command::Navigate { request_id, action }) => {
          if cloned
            .borrow()
            .state
            .as_ref()
            .is_some_and(|state| state.request.request_id == request_id)
          {
            match action.as_str() {
              "edit.up" | "select.up" => ui.invoke_navigate(-1),
              "edit.down" | "select.down" => ui.invoke_navigate(1),
              "edit.left" | "back" => ui.invoke_back(),
              "edit.right" | "accept" => ui.invoke_activate(),
              "close" => ui.invoke_dismiss(),
              _ => {}
            }
          }
        }
        Ok(Command::Hide { request_id }) => {
          if cloned
            .borrow()
            .state
            .as_ref()
            .is_some_and(|s| s.request.request_id == request_id)
          {
            finish(&ui, &cloned, "client", None);
          }
        }
        Ok(Command::Quit) => {
          finish(&ui, &cloned, "shutdown", None);
          let _ = slint::quit_event_loop();
        }
        Err(error) => emit(json!({"type":"error", "message":error.to_string()})),
      }
    });
    let weak = ui.as_weak();
    std::thread::spawn(move || {
      let mut stdin = io::stdin().lock();
      loop {
        let mut bytes = Vec::new();
        // One pending frame, no polling/wakeups while idle.
        let read = (&mut stdin)
          .take(1024 * 1024 + 1)
          .read_until(b'\n', &mut bytes);
        let (frame, last) = match read {
          Ok(0) | Err(_) => ("{\"type\":\"quit\"}".to_string(), true),
          Ok(_) if bytes.len() > 1024 * 1024 => {
            emit(json!({"type":"error", "message":"frame exceeds 1 MiB; host closing"}));
            ("{\"type\":\"quit\"}".to_string(), true)
          }
          Ok(_) => match String::from_utf8(bytes) {
            Ok(frame) => (frame, false),
            Err(_) => {
              emit(json!({"type":"error", "message":"invalid UTF-8"}));
              continue;
            }
          },
        };
        let (ack, done) = std::sync::mpsc::channel();
        if weak
          .upgrade_in_event_loop(move |ui| {
            ui.invoke_dispatch_command(frame.into());
            let _ = ack.send(());
          })
          .is_err()
        {
          break;
        }
        if done.recv().is_err() || last {
          break;
        }
      }
    });
    emit(
      json!({"type":"ready", "protocol":1, "capabilities":["dynamic_pages", "keep_open", "navigate", "quick_keys"]}),
    );
    slint::run_event_loop_until_quit()?;
    return Ok(());
  }
  slint::run_event_loop()?;
  Ok(())
}
