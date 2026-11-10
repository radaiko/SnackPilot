//! Pure navigation model for the app shell: four tabs, a push/pop stack per tab.
//!
//! UI-agnostic so it runs headless on every platform.
//! @req REQ-UI-03 @req REQ-UI-04

/// The four root tabs, in bar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tab {
    Menus,
    Orders,
    Billing,
    Settings,
}

/// A screen reachable in the stack. Root screens mirror the tabs; pushable
/// screens are added as later tasks grow the flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    Menus,
    Orders,
    Billing,
    Settings,
    SettingsDetail,
}

/// How the last navigation happened, so the shell can animate forward vs back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Back,
}

/// Open section of the settings screen (desktop-ui-redesign D1). On wide
/// layouts it is a containment field of the settings workspace; on narrow
/// layouts it is projected as a pushed [`Screen::SettingsDetail`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsDetail {
    Profile,
    Security,
    Notifications,
    Data,
    Advanced,
}

/// The three long lists with an identity-stable preview and cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListKind {
    Menus,
    Orders,
    Receipts,
}

impl ListKind {
    /// Index into [`crate::state::State::cursors`].
    pub fn index(self) -> usize {
        match self {
            ListKind::Menus => 0,
            ListKind::Orders => 1,
            ListKind::Receipts => 2,
        }
    }

    /// The list the tab shows, if it has one.
    pub fn for_tab(tab: Tab) -> Option<ListKind> {
        match tab {
            Tab::Menus => Some(ListKind::Menus),
            Tab::Orders => Some(ListKind::Orders),
            Tab::Billing => Some(ListKind::Receipts),
            Tab::Settings => None,
        }
    }
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Menus, Tab::Orders, Tab::Billing, Tab::Settings];

    /// Tab titles per docs/requirements/04-ui-ux.md §2.
    pub fn title(self) -> &'static str {
        match self {
            Tab::Menus => "Menüs",
            Tab::Orders => "Bestellungen",
            Tab::Billing => "Abrechnung",
            Tab::Settings => "Einstellungen",
        }
    }

    /// Lucide icon name for the tab, kebab-case as in the gpui-kit asset set.
    pub fn icon_name(self) -> &'static str {
        match self {
            Tab::Menus => "utensils",
            Tab::Orders => "receipt",
            Tab::Billing => "wallet",
            Tab::Settings => "settings",
        }
    }

    pub fn screen(self) -> Screen {
        Screen::from(self)
    }
}

impl From<Tab> for Screen {
    fn from(tab: Tab) -> Screen {
        match tab {
            Tab::Menus => Screen::Menus,
            Tab::Orders => Screen::Orders,
            Tab::Billing => Screen::Billing,
            Tab::Settings => Screen::Settings,
        }
    }
}

/// Active tab plus its navigation stack. The stack always holds at least the
/// tab's root screen; the bottom entry is that root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigation {
    active: Tab,
    stack: Vec<Screen>,
    direction: Direction,
}

impl Default for Navigation {
    fn default() -> Self {
        Self::new()
    }
}

impl Navigation {
    pub fn new() -> Self {
        Navigation {
            active: Tab::Menus,
            stack: vec![Screen::Menus],
            direction: Direction::Forward,
        }
    }

    pub fn active_tab(&self) -> Tab {
        self.active
    }

    pub fn stack(&self) -> &[Screen] {
        &self.stack
    }

    pub fn top(&self) -> Screen {
        *self.stack.last().expect("stack never empty")
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Tab-bar tap. Re-tapping the active tab is a no-op (v1 parity); any other
    /// tab resets the stack to that tab's root screen.
    pub fn select_tab(&mut self, tab: Tab) {
        if tab == self.active {
            return;
        }
        self.active = tab;
        self.stack = vec![tab.screen()];
        self.direction = Direction::Forward;
    }

    pub fn push(&mut self, screen: Screen) {
        self.stack.push(screen);
        self.direction = Direction::Forward;
    }

    /// Pop one screen; the root screen never pops. Returns the popped screen.
    pub fn pop(&mut self) -> Option<Screen> {
        if self.stack.len() <= 1 {
            return None;
        }
        self.direction = Direction::Back;
        self.stack.pop()
    }
}
