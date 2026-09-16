//! X11's winit backend returns no theme. Follow desktop settings without polling.
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};

pub struct Watchers(Vec<Child>);
impl Drop for Watchers {
  fn drop(&mut self) {
    for child in &mut self.0 {
      let _ = child.kill();
      let _ = child.wait();
    }
  }
}

fn output(program: &str, args: &[&str]) -> Option<String> {
  let result = Command::new(program)
    .args(args)
    .stderr(Stdio::null())
    .output()
    .ok()?;
  result
    .status
    .success()
    .then(|| String::from_utf8_lossy(&result.stdout).trim().to_owned())
}

fn dark_name(name: &str) -> bool {
  let name = name.to_lowercase();
  name.contains("dark") || name.contains("black")
}

fn current(xfce: bool) -> Option<bool> {
  if xfce {
    return output("xfconf-query", &["-c", "xsettings", "-p", "/Net/ThemeName"])
      .map(|name| dark_name(&name));
  }
  let scheme = output(
    "gsettings",
    &["get", "org.gnome.desktop.interface", "color-scheme"],
  );
  match scheme.as_deref().map(|s| s.trim_matches('\'')) {
    Some("prefer-dark") => Some(true),
    Some("prefer-light") => Some(false),
    _ => output(
      "gsettings",
      &["get", "org.gnome.desktop.interface", "gtk-theme"],
    )
    .map(|name| dark_name(&name)),
  }
}

pub fn watch(ui: slint::Weak<crate::Palette>) -> Watchers {
  let xfce = std::env::var("XDG_CURRENT_DESKTOP")
    .unwrap_or_default()
    .to_lowercase()
    .contains("xfce")
    || (std::env::var("XDG_CURRENT_DESKTOP")
      .unwrap_or_default()
      .is_empty()
      && output("xfconf-query", &["-c", "xsettings", "-p", "/Net/ThemeName"]).is_some());
  let (program, args) = if xfce {
    ("xfconf-query", vec!["-c", "xsettings", "-m"])
  } else {
    ("gsettings", vec!["monitor", "org.gnome.desktop.interface"])
  };
  let mut watchers = Watchers(Vec::new());
  if let Ok(mut child) = Command::new(program)
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
  {
    let stdout = child.stdout.take().expect("piped stdout");
    watchers.0.push(child);
    std::thread::spawn(move || {
      let update = || {
        if let Some(dark) = current(xfce) {
          let _ = ui.upgrade_in_event_loop(move |ui| ui.set_dark(dark));
        }
      };
      update();
      for line in BufReader::new(stdout).lines() {
        if line.is_err() {
          break;
        }
        update();
      }
    });
  }
  watchers
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn conventional_gtk_theme_names() {
    assert!(dark_name("Adwaita-dark"));
    assert!(dark_name("Yaru-Dark"));
    assert!(!dark_name("Xfce"));
    assert!(!dark_name("Adwaita"));
  }
}
