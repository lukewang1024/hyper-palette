use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ROWS: usize = 9;
pub fn height(count: usize) -> u32 {
  100 + 52 * count.clamp(1, MAX_ROWS) as u32
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
  pub id: String,
  pub title: String,
  #[serde(default)]
  pub detail: String,
  #[serde(default)]
  pub shortcut: String,
  #[serde(default)]
  pub keywords: String,
  #[serde(default)]
  pub disabled: bool,
  #[serde(default)]
  pub submenu: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Menu {
  pub title: String,
  pub items: Vec<Item>,
  #[serde(default)]
  pub quick: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
  pub request_id: String,
  pub root: String,
  pub menus: BTreeMap<String, Menu>,
}

impl Request {
  pub fn validate(&self) -> Result<(), &'static str> {
    if self.request_id.is_empty() || !self.menus.contains_key(&self.root) {
      return Err("missing request_id or root menu");
    }
    if self.menus.len() > 64 || self.menus.values().map(|m| m.items.len()).sum::<usize>() > 4096 {
      return Err("too many menus or items");
    }
    for menu in self.menus.values() {
      let mut ids = BTreeSet::new();
      for item in &menu.items {
        if item.id.is_empty() || !ids.insert(&item.id) {
          return Err("item IDs must be nonempty and unique within each menu");
        }
        if item
          .submenu
          .as_ref()
          .is_some_and(|id| !self.menus.contains_key(id))
        {
          return Err("unknown submenu");
        }
      }
    }
    Ok(())
  }
}

#[derive(Clone, Debug, Default)]
struct Page {
  menu: String,
  query: String,
  searching: bool,
  selected: Option<usize>, // index into filtered rows, not the original menu
}

pub struct State {
  pub request: Request,
  page: Page,
  parents: Vec<Page>,
}

impl State {
  pub fn new(request: Request) -> Result<Self, &'static str> {
    request.validate()?;
    let page = Page {
      menu: request.root.clone(),
      ..Page::default()
    };
    let mut state = Self {
      request,
      page,
      parents: vec![],
    };
    state.select_first();
    Ok(state)
  }
  pub fn title(&self) -> &str {
    &self.request.menus[&self.page.menu].title
  }
  pub fn query(&self) -> &str {
    &self.page.query
  }
  pub fn quick(&self) -> bool {
    self.request.menus[&self.page.menu].quick && !self.page.searching
  }
  pub fn begin_search(&mut self) {
    self.page.searching = true;
  }
  pub fn quick_key(&mut self, key: &str) -> bool {
    if !self.quick() {
      return false;
    }
    let index = self
      .rows()
      .iter()
      .position(|item| !item.disabled && item.shortcut.eq_ignore_ascii_case(key));
    if let Some(index) = index {
      self.select(index);
      true
    } else {
      false
    }
  }
  pub fn selected(&self) -> Option<usize> {
    self.page.selected
  }
  pub fn depth(&self) -> usize {
    self.parents.len()
  }
  pub fn rows(&self) -> Vec<&Item> {
    let query = self.page.query.to_lowercase();
    self.request.menus[&self.page.menu]
      .items
      .iter()
      .filter(|item| {
        let haystack = format!(
          "{} {} {} {}",
          item.title, item.detail, item.keywords, item.shortcut
        )
        .to_lowercase();
        query.split_whitespace().all(|word| haystack.contains(word))
      })
      .collect()
  }
  fn select_first(&mut self) {
    self.page.selected = self.rows().iter().position(|item| !item.disabled);
  }
  pub fn search(&mut self, query: String) {
    self.begin_search();
    self.page.query = query;
    self.select_first();
  }
  pub fn select(&mut self, index: usize) {
    if self.rows().get(index).is_some_and(|item| !item.disabled) {
      self.page.selected = Some(index);
    }
  }
  pub fn step(&mut self, delta: i32) {
    let rows = self.rows();
    if rows.is_empty() {
      return;
    }
    let n = rows.len() as i32;
    let start = self
      .page
      .selected
      .map(|i| i as i32)
      .unwrap_or(if delta > 0 { -1 } else { 0 });
    let next = (1..=n)
      .map(|offset| (start + offset * delta).rem_euclid(n) as usize)
      .find(|&i| !rows[i].disabled);
    self.page.selected = next;
  }
  // A submenu stays in the same native window. Only leaf actions leave the UI.
  pub fn activate(&mut self) -> Option<String> {
    let item = self.rows().get(self.page.selected?).copied()?.clone();
    if item.disabled {
      return None;
    }
    if let Some(menu) = item.submenu {
      if self.parents.len() >= 32 {
        return None;
      }
      self.parents.push(self.page.clone());
      self.page = Page {
        menu,
        ..Page::default()
      };
      self.select_first();
      None
    } else {
      Some(item.id)
    }
  }
  pub fn back(&mut self) -> bool {
    if !self.page.query.is_empty() {
      return false;
    }
    if let Some(page) = self.parents.pop() {
      self.page = page;
      true
    } else {
      false
    }
  }
}

pub fn demo(zh: bool) -> Request {
  // Generic fixtures only. Real applications supply all business data over IPC.
  let fixture: serde_json::Value =
    serde_json::from_str(include_str!("../assets/demo.json")).unwrap();
  let keys = &fixture["keys"];
  let locales: BTreeMap<String, [String; 2]> =
    serde_json::from_value(fixture["labels"].clone()).unwrap();
  let label = |id: &str, fallback: &str| {
    locales
      .get(id)
      .map(|v| v[usize::from(zh)].clone())
      .unwrap_or_else(|| fallback.into())
  };
  let keywords = |id: &str| locales.get(id).map(|v| v.join(" ")).unwrap_or_default();
  let mut menus = BTreeMap::new();
  for (name, entries) in keys["menus"].as_object().unwrap() {
    let id = format!("menu.{name}");
    let items = entries
      .as_array()
      .unwrap()
      .iter()
      .map(|entry| {
        let action = entry[2].as_str().unwrap();
        Item {
          id: action.into(),
          title: label(action, entry[1].as_str().unwrap()),
          detail: action.into(),
          shortcut: entry[0].as_str().unwrap().into(),
          keywords: keywords(action),
          disabled: false,
          submenu: action
            .strip_prefix("menu.")
            .filter(|name| keys["menus"].get(name).is_some() || *name == "roles")
            .map(|_| action.into()),
        }
      })
      .collect();
    menus.insert(
      id.clone(),
      Menu {
        quick: true,
        title: label(&id, name),
        items,
      },
    );
  }
  // Role choices are a read-only preview; actual app discovery remains in adapters.
  let items = keys["roles"]
    .as_object()
    .unwrap()
    .iter()
    .map(|(role, entry)| {
      let id = format!("role.{role}");
      Item {
        id: format!("prototype.configure.{role}"),
        title: label(&id, role),
        detail: entry["apps"]
          .as_array()
          .map(|apps| {
            apps
              .iter()
              .filter_map(|v| v.as_str())
              .collect::<Vec<_>>()
              .join(" · ")
          })
          .unwrap_or_default(),
        shortcut: String::new(),
        keywords: keywords(&id),
        disabled: false,
        submenu: None,
      }
    })
    .collect();
  menus.insert(
    "menu.roles".into(),
    Menu {
      quick: false,
      title: label("menu.roles", "Roles"),
      items,
    },
  );
  Request {
    request_id: "demo".into(),
    root: "menu.config".into(),
    menus,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn quick_keys_follow_menu_and_submenu_configuration() {
    let mut state = State::new(demo(false)).unwrap();
    assert!(state.quick_key("c"));
    assert_eq!(state.activate(), None);
    assert_eq!(state.depth(), 1);
    assert!(state.quick_key("v"));
    assert_eq!(state.activate().as_deref(), Some("clipboard.history"));
    assert!(state.back());
    assert!(state.quick());
  }
  #[test]
  fn search_mode_never_dispatches_bare_letters_even_after_clearing_query() {
    let mut state = State::new(demo(false)).unwrap();
    state.begin_search();
    assert!(!state.quick_key("c"));
    state.search("clipboard".into());
    state.search(String::new());
    assert!(!state.quick_key("v"));
  }
  #[test]
  fn quick_keys_ignore_disabled_items_and_are_opt_in() {
    let mut request = demo(false);
    let menu = request.menus.get_mut("menu.config").unwrap();
    menu
      .items
      .iter_mut()
      .find(|item| item.shortcut == "c")
      .unwrap()
      .disabled = true;
    let mut state = State::new(request.clone()).unwrap();
    assert!(!state.quick_key("c"));
    request.menus.get_mut("menu.config").unwrap().quick = false;
    assert!(!State::new(request).unwrap().quick_key("k"));
  }
  #[test]
  fn bounded_height() {
    assert_eq!(
      [0, 1, 8, 9, 10, 100].map(height),
      [152, 152, 516, 568, 568, 568]
    );
  }
  #[test]
  fn bilingual_search() {
    let mut state = State::new(demo(true)).unwrap();
    state.search("CLIPBOARD".into());
    assert_eq!(state.rows().len(), 1);
    state.search("剪贴板".into());
    assert_eq!(state.rows().len(), 1);
    state.search("no such action".into());
    state.step(1);
    assert_eq!(state.activate(), None);
  }
  #[test]
  fn submenu_restores_search_and_selection() {
    let mut state = State::new(demo(false)).unwrap();
    state.search("Clipboard".into());
    assert_eq!(state.activate(), None);
    assert_eq!(state.depth(), 1);
    state.search("history".into());
    assert!(!state.back());
    state.search(String::new());
    assert!(state.back());
    assert_eq!(state.query(), "Clipboard");
    assert_eq!(state.selected(), Some(0));
  }
  #[test]
  fn disabled_navigation_and_wrap() {
    let mut req = demo(false);
    req.menus.get_mut("menu.config").unwrap().items[0].disabled = true;
    let mut state = State::new(req).unwrap();
    assert_eq!(state.selected(), Some(1));
    state.select(0);
    assert_eq!(state.selected(), Some(1));
    state.step(-1);
    assert_eq!(state.selected(), Some(7));
    state.step(1);
    assert_eq!(state.selected(), Some(1));
  }
  #[test]
  fn invalid_menu_rejected() {
    let mut req = demo(false);
    req.root = "absent".into();
    assert!(State::new(req).is_err());
  }
  #[test]
  fn cycles_are_bounded() {
    let mut req = demo(false);
    req.menus.get_mut("menu.config").unwrap().items[0].submenu = Some("menu.config".into());
    let mut state = State::new(req).unwrap();
    for _ in 0..100 {
      state.activate();
    }
    assert_eq!(state.depth(), 32);
  }

  #[test]
  fn leaf_returns_exact_action_id() {
    let mut state = State::new(demo(true)).unwrap();
    state.search("reload".into());
    assert_eq!(state.activate().as_deref(), Some("system.reload"));
  }

  #[test]
  fn all_disabled_is_inert() {
    let mut req = demo(false);
    for item in &mut req.menus.get_mut("menu.config").unwrap().items {
      item.disabled = true;
    }
    let mut state = State::new(req).unwrap();
    state.step(1);
    state.step(-1);
    state.select(0);
    assert_eq!(state.selected(), None);
    assert_eq!(state.activate(), None);
  }

  #[test]
  fn bad_submenu_and_duplicate_ids_rejected() {
    let mut req = demo(false);
    req.menus.get_mut("menu.config").unwrap().items[0].submenu = Some("missing".into());
    assert!(req.validate().is_err());
    let mut req = demo(false);
    let menu = req.menus.get_mut("menu.config").unwrap();
    menu.items.push(menu.items[0].clone());
    assert!(req.validate().is_err());
  }
}
