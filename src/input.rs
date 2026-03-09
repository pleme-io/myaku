//! Keyboard input handling with vim-style navigation.
//!
//! Supports multiple modes: Dashboard, Process, and Filter.
//! Uses `awase::Hotkey` for key binding definitions.

use madori::event::{KeyCode, KeyEvent};

/// Application mode determines which keybindings are active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Dashboard view: overview of CPU, memory, disk, network.
    Dashboard,
    /// Process table view: scrollable, sortable, filterable.
    Process,
    /// Filter input mode: typing a filter string.
    Filter,
}

/// Actions that can be triggered by keyboard input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Quit the application.
    Quit,
    /// Switch to dashboard view.
    SwitchDashboard,
    /// Switch to process view.
    SwitchProcess,
    /// Force refresh all metrics.
    ForceRefresh,
    /// Cycle focus to next panel (Tab).
    FocusNext,
    /// Cycle focus to previous panel (Shift+Tab).
    FocusPrev,
    /// Navigate up in a list.
    Up,
    /// Navigate down in a list.
    Down,
    /// Page up.
    PageUp,
    /// Page down.
    PageDown,
    /// Go to first item.
    First,
    /// Go to last item.
    Last,
    /// Cycle sort column in process view.
    CycleSort,
    /// Toggle sort direction.
    ToggleSortDirection,
    /// Enter filter mode.
    EnterFilter,
    /// Exit current mode (Escape).
    Back,
    /// Confirm / select.
    Confirm,
    /// Character typed (for filter input).
    Char(char),
    /// Backspace (for filter input).
    Backspace,
    /// No action recognized.
    None,
}

/// Convert a madori `KeyCode` to an `awase::Key`.
fn to_awase_key(key: &KeyCode) -> Option<awase::Key> {
    match key {
        KeyCode::Char(c) => match c.to_ascii_lowercase() {
            'a' => Some(awase::Key::A),
            'b' => Some(awase::Key::B),
            'c' => Some(awase::Key::C),
            'd' => Some(awase::Key::D),
            'e' => Some(awase::Key::E),
            'f' => Some(awase::Key::F),
            'g' => Some(awase::Key::G),
            'h' => Some(awase::Key::H),
            'i' => Some(awase::Key::I),
            'j' => Some(awase::Key::J),
            'k' => Some(awase::Key::K),
            'l' => Some(awase::Key::L),
            'm' => Some(awase::Key::M),
            'n' => Some(awase::Key::N),
            'o' => Some(awase::Key::O),
            'p' => Some(awase::Key::P),
            'q' => Some(awase::Key::Q),
            'r' => Some(awase::Key::R),
            's' => Some(awase::Key::S),
            't' => Some(awase::Key::T),
            'u' => Some(awase::Key::U),
            'v' => Some(awase::Key::V),
            'w' => Some(awase::Key::W),
            'x' => Some(awase::Key::X),
            'y' => Some(awase::Key::Y),
            'z' => Some(awase::Key::Z),
            '/' => Some(awase::Key::Slash),
            '+' | '=' => Some(awase::Key::Equal),
            '-' => Some(awase::Key::Minus),
            ',' => Some(awase::Key::Comma),
            '.' => Some(awase::Key::Period),
            _ => Option::None,
        },
        KeyCode::Escape => Some(awase::Key::Escape),
        KeyCode::Enter => Some(awase::Key::Return),
        KeyCode::Tab => Some(awase::Key::Tab),
        KeyCode::Backspace => Some(awase::Key::Backspace),
        KeyCode::Delete => Some(awase::Key::Delete),
        KeyCode::Up => Some(awase::Key::Up),
        KeyCode::Down => Some(awase::Key::Down),
        KeyCode::Left => Some(awase::Key::Left),
        KeyCode::Right => Some(awase::Key::Right),
        KeyCode::Home => Some(awase::Key::Home),
        KeyCode::End => Some(awase::Key::End),
        KeyCode::PageUp => Some(awase::Key::PageUp),
        KeyCode::PageDown => Some(awase::Key::PageDown),
        _ => Option::None,
    }
}

/// Convert madori modifiers to awase modifiers.
fn to_awase_modifiers(mods: &madori::event::Modifiers) -> awase::Modifiers {
    let mut result = awase::Modifiers::NONE;
    if mods.ctrl {
        result = result | awase::Modifiers::CTRL;
    }
    if mods.alt {
        result = result | awase::Modifiers::ALT;
    }
    if mods.shift {
        result = result | awase::Modifiers::SHIFT;
    }
    if mods.meta {
        result = result | awase::Modifiers::CMD;
    }
    result
}

/// Map a key event to an action based on current mode.
#[must_use]
pub fn map_key(event: &KeyEvent, mode: Mode) -> Action {
    if !event.pressed {
        return Action::None;
    }

    match mode {
        Mode::Filter => map_filter_key(event),
        Mode::Dashboard => map_dashboard_key(event),
        Mode::Process => map_process_key(event),
    }
}

fn map_dashboard_key(event: &KeyEvent) -> Action {
    // Build awase hotkey for key matching
    if let Some(awase_key) = to_awase_key(&event.key) {
        let awase_mods = to_awase_modifiers(&event.modifiers);
        let hotkey = awase::Hotkey::new(awase_mods, awase_key);

        // Use awase hotkey for checking key + modifier combinations
        if hotkey.modifiers.is_empty() {
            match hotkey.key {
                awase::Key::Q => return Action::Quit,
                awase::Key::P => return Action::SwitchProcess,
                awase::Key::R => return Action::ForceRefresh,
                awase::Key::Escape => return Action::Quit,
                _ => {}
            }
        }
    }

    match event.key {
        KeyCode::Tab if !event.modifiers.shift => Action::FocusNext,
        KeyCode::Tab if event.modifiers.shift => Action::FocusPrev,
        KeyCode::Char('j') | KeyCode::Down => Action::Down,
        KeyCode::Char('k') | KeyCode::Up => Action::Up,
        KeyCode::Char('h') | KeyCode::Left => Action::FocusPrev,
        KeyCode::Char('l') | KeyCode::Right => Action::FocusNext,
        _ => Action::None,
    }
}

fn map_process_key(event: &KeyEvent) -> Action {
    // Build awase hotkey for key matching
    if let Some(awase_key) = to_awase_key(&event.key) {
        let awase_mods = to_awase_modifiers(&event.modifiers);
        let hotkey = awase::Hotkey::new(awase_mods, awase_key);

        if hotkey.modifiers.is_empty() {
            match hotkey.key {
                awase::Key::Q => return Action::Quit,
                awase::Key::Escape => return Action::Back,
                awase::Key::Slash => return Action::EnterFilter,
                _ => {}
            }
        }
    }

    match event.key {
        KeyCode::Char('j') | KeyCode::Down => Action::Down,
        KeyCode::Char('k') | KeyCode::Up => Action::Up,
        KeyCode::Char('g') if !event.modifiers.shift => Action::First,
        KeyCode::Char('G') | KeyCode::Char('g') if event.modifiers.shift => Action::Last,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::Char('s') if !event.modifiers.shift => Action::CycleSort,
        KeyCode::Char('S') | KeyCode::Char('s') if event.modifiers.shift => {
            Action::ToggleSortDirection
        }
        KeyCode::Char('r') if !event.modifiers.any() => Action::ForceRefresh,
        KeyCode::Char('d') if !event.modifiers.any() => Action::SwitchDashboard,
        KeyCode::Tab => Action::FocusNext,
        _ => Action::None,
    }
}

fn map_filter_key(event: &KeyEvent) -> Action {
    match event.key {
        KeyCode::Escape => Action::Back,
        KeyCode::Enter => Action::Confirm,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Char(c) => Action::Char(c),
        _ => Action::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use madori::event::Modifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            key: code,
            pressed: true,
            modifiers: Modifiers::default(),
            text: None,
        }
    }

    #[allow(dead_code)]
    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent {
            key: code,
            pressed: true,
            modifiers: Modifiers {
                shift: true,
                ..Default::default()
            },
            text: None,
        }
    }

    #[test]
    fn dashboard_quit() {
        assert_eq!(map_key(&key(KeyCode::Char('q')), Mode::Dashboard), Action::Quit);
    }

    #[test]
    fn dashboard_switch_process() {
        assert_eq!(
            map_key(&key(KeyCode::Char('p')), Mode::Dashboard),
            Action::SwitchProcess
        );
    }

    #[test]
    fn dashboard_vim_navigation() {
        assert_eq!(map_key(&key(KeyCode::Char('j')), Mode::Dashboard), Action::Down);
        assert_eq!(map_key(&key(KeyCode::Char('k')), Mode::Dashboard), Action::Up);
        assert_eq!(map_key(&key(KeyCode::Char('h')), Mode::Dashboard), Action::FocusPrev);
        assert_eq!(map_key(&key(KeyCode::Char('l')), Mode::Dashboard), Action::FocusNext);
    }

    #[test]
    fn process_sort_cycle() {
        assert_eq!(
            map_key(&key(KeyCode::Char('s')), Mode::Process),
            Action::CycleSort
        );
    }

    #[test]
    fn process_filter_entry() {
        assert_eq!(
            map_key(&key(KeyCode::Char('/')), Mode::Process),
            Action::EnterFilter
        );
    }

    #[test]
    fn process_escape_back() {
        assert_eq!(map_key(&key(KeyCode::Escape), Mode::Process), Action::Back);
    }

    #[test]
    fn filter_char_input() {
        assert_eq!(
            map_key(&key(KeyCode::Char('a')), Mode::Filter),
            Action::Char('a')
        );
    }

    #[test]
    fn filter_backspace() {
        assert_eq!(
            map_key(&key(KeyCode::Backspace), Mode::Filter),
            Action::Backspace
        );
    }

    #[test]
    fn filter_escape_exits() {
        assert_eq!(map_key(&key(KeyCode::Escape), Mode::Filter), Action::Back);
    }

    #[test]
    fn released_keys_ignored() {
        let mut k = key(KeyCode::Char('q'));
        k.pressed = false;
        assert_eq!(map_key(&k, Mode::Dashboard), Action::None);
    }

    #[test]
    fn awase_key_conversion() {
        assert_eq!(to_awase_key(&KeyCode::Char('a')), Some(awase::Key::A));
        assert_eq!(to_awase_key(&KeyCode::Escape), Some(awase::Key::Escape));
        assert_eq!(to_awase_key(&KeyCode::Enter), Some(awase::Key::Return));
    }

    #[test]
    fn awase_modifier_conversion() {
        let mods = Modifiers {
            ctrl: true,
            shift: true,
            ..Default::default()
        };
        let awase_mods = to_awase_modifiers(&mods);
        assert!(awase_mods.contains(awase::Modifiers::CTRL));
        assert!(awase_mods.contains(awase::Modifiers::SHIFT));
        assert!(!awase_mods.contains(awase::Modifiers::CMD));
    }
}
