use crate::config::ToolbarPosition;
use crate::domain::Choice;
use crate::screenshot::Args;
use crate::session::messages::Msg;
use cosmic::iced::keyboard::{Key, Modifiers, key::Named};

/// Route a key press to the text annotation being typed.
///
/// While a text annotation is open, keystrokes are *text*, not shortcuts —
/// otherwise typing "and" would toggle the shape tool, start a redaction and
/// re-enter region mode. This runs before every other binding and swallows
/// anything it plausibly owns.
fn handle_text_editing_key(key: &Key, modifiers: Modifiers) -> Option<Msg> {
    match key {
        // Enter commits; Shift+Enter adds a line break.
        Key::Named(Named::Enter) if modifiers.shift() => Some(Msg::text_newline()),
        Key::Named(Named::Enter) => Some(Msg::text_commit()),
        // Escape finishes the label (keeping it) rather than cancelling the whole
        // screenshot. Nothing discards a typed label implicitly.
        Key::Named(Named::Escape) => Some(Msg::text_commit()),
        Key::Named(Named::Backspace) => Some(Msg::text_backspace()),
        Key::Named(Named::Tab) => Some(Msg::text_insert("    ".to_string())),
        // Printable characters. Ctrl/Alt combos are left alone so they can't be
        // typed as literal text, but they also must not fall through to the
        // shortcut table mid-edit — swallow them instead.
        Key::Character(c) => {
            if modifiers.control() || modifiers.alt() {
                Some(Msg::text_commit())
            } else {
                Some(Msg::text_insert(c.to_string()))
            }
        }
        // Everything else (arrows, F-keys, modifiers alone) is ignored. Returning
        // None is safe here because the caller early-returns, so these keys never
        // reach the shortcut table below.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::messages::{DrawMsg, TextAction};

    fn ch(c: &str) -> Key {
        Key::Character(c.into())
    }

    /// Extract the text action a key produced, if any.
    fn text_action(key: Key, modifiers: Modifiers) -> Option<TextAction> {
        match handle_text_editing_key(&key, modifiers) {
            Some(Msg::Draw(DrawMsg::Text(action))) => Some(action),
            _ => None,
        }
    }

    #[test]
    fn letters_that_are_shortcuts_are_typed_as_text_instead() {
        // Every one of these is a mode shortcut when not editing text: typing
        // "adroqs" must not toggle tools, start a redaction or re-enter region
        // mode. This is the whole reason text editing intercepts the keyboard.
        for c in ["a", "d", "r", "o", "q", "s", "R", "D"] {
            match text_action(ch(c), Modifiers::default()) {
                Some(TextAction::Insert(s)) => assert_eq!(s, c),
                other => panic!("{c:?} should insert text, got {other:?}"),
            }
        }
    }

    #[test]
    fn space_is_typed_as_text() {
        // Space arrives as a character, not a named key.
        match text_action(ch(" "), Modifiers::default()) {
            Some(TextAction::Insert(s)) => assert_eq!(s, " "),
            other => panic!("space should insert text, got {other:?}"),
        }
    }

    #[test]
    fn enter_commits_and_shift_enter_adds_a_line() {
        assert!(matches!(
            text_action(Key::Named(Named::Enter), Modifiers::default()),
            Some(TextAction::Commit)
        ));
        assert!(matches!(
            text_action(Key::Named(Named::Enter), Modifiers::SHIFT),
            Some(TextAction::Newline)
        ));
    }

    #[test]
    fn escape_saves_the_label_rather_than_cancelling_the_screenshot() {
        // Without interception this would cancel the whole capture; and it must
        // keep the text, not discard it.
        assert!(matches!(
            text_action(Key::Named(Named::Escape), Modifiers::default()),
            Some(TextAction::Commit)
        ));
    }

    #[test]
    fn backspace_deletes_and_tab_indents() {
        assert!(matches!(
            text_action(Key::Named(Named::Backspace), Modifiers::default()),
            Some(TextAction::Backspace)
        ));
        match text_action(Key::Named(Named::Tab), Modifiers::default()) {
            Some(TextAction::Insert(s)) => assert_eq!(s, "    "),
            other => panic!("tab should indent, got {other:?}"),
        }
    }

    #[test]
    fn ctrl_combos_commit_instead_of_typing_a_letter() {
        // Ctrl+Z shouldn't insert "z" — it finishes the label so the next press
        // reaches the real undo binding.
        assert!(matches!(
            text_action(ch("z"), Modifiers::CTRL),
            Some(TextAction::Commit)
        ));
    }

    #[test]
    fn navigation_keys_are_swallowed_while_editing() {
        // Must not fall through to screen navigation / toolbar movement.
        for key in [
            Key::Named(Named::ArrowLeft),
            Key::Named(Named::ArrowRight),
            Key::Named(Named::Home),
        ] {
            assert!(
                handle_text_editing_key(&key, Modifiers::default()).is_none(),
                "{key:?} should be ignored while editing"
            );
        }
    }
}

pub fn handle_key_event(
    args: &Args,
    key: Key,
    modifiers: Modifiers,
    current_output_index: usize,
) -> Option<Msg> {
    // Text editing owns the keyboard while it's active.
    if args.annotations.text_editing.is_some() {
        return handle_text_editing_key(&key, modifiers);
    }

    // Determine if we have a complete selection for action shortcuts
    let has_selection = match &args.session.choice {
        Choice::Rectangle(r, _) => r.dimensions().is_some(),
        Choice::Output(Some(_)) => true, // Only confirmed screen counts as selection
        _ => false,
    };

    let arrow_mode = args.annotations.arrow_mode;
    let redact_mode = args.annotations.redact_mode;

    // Check if we're in a mode that supports navigation
    let in_screen_picker = matches!(&args.session.choice, Choice::Output(None)); // Picker mode only

    // Check if OCR/QR have results (pressing O/Q again should copy and close)
    let has_ocr_result = args.detection.ocr_text.is_some();
    let has_qr_result = !args.detection.qr_codes.is_empty();

    match key {
        // Ctrl+hjkl or Ctrl+arrows: move toolbar position
        Key::Character(c) if c.as_str() == "h" && modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Left))
        }
        Key::Character(c) if c.as_str() == "j" && modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Bottom))
        }
        Key::Character(c) if c.as_str() == "k" && modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Top))
        }
        Key::Character(c) if c.as_str() == "l" && modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Right))
        }
        Key::Named(Named::ArrowLeft) if modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Left))
        }
        Key::Named(Named::ArrowDown) if modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Bottom))
        }
        Key::Named(Named::ArrowUp) if modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Top))
        }
        Key::Named(Named::ArrowRight) if modifiers.control() => {
            Some(Msg::toolbar_position(ToolbarPosition::Right))
        }
        // Undo/redo shortcuts
        Key::Character(c) if c.as_str() == "z" && modifiers.control() && !modifiers.shift() => {
            Some(Msg::undo())
        }
        Key::Character(c)
            if (c.as_str() == "y" && modifiers.control())
                || (c.as_str() == "z" && modifiers.control() && modifiers.shift()) =>
        {
            Some(Msg::redo())
        }
        // Save/copy shortcuts (always available - empty selection captures all screens)
        Key::Named(Named::Enter) if modifiers.control() => Some(Msg::save_to_pictures()),
        Key::Named(Named::Escape) => Some(Msg::cancel_requested()),
        // Space/Enter to confirm selection in picker mode (screen)
        Key::Character(c) if c.as_str() == " " && in_screen_picker => Some(Msg::confirm()),
        Key::Named(Named::Enter) if in_screen_picker => Some(Msg::confirm()),
        // Enter to copy when not in picker mode
        Key::Named(Named::Enter) => Some(Msg::copy_to_clipboard()),
        // Navigation keys in screen picker: h/l and arrows navigate screens
        Key::Character(c) if c.as_str() == "h" && in_screen_picker => Some(Msg::navigate_left()),
        Key::Character(c) if c.as_str() == "l" && in_screen_picker => Some(Msg::navigate_right()),
        Key::Named(Named::ArrowLeft) if in_screen_picker => Some(Msg::navigate_left()),
        Key::Named(Named::ArrowRight) if in_screen_picker => Some(Msg::navigate_right()),
        // Mode toggle shortcuts (require selection)
        // Shift+A: cycle shape tool (arrow -> circle -> rectangle -> arrow)
        Key::Character(c)
            if c.as_str().eq_ignore_ascii_case("a") && modifiers.shift() && has_selection =>
        {
            Some(Msg::cycle_shape_tool())
        }
        // A: toggle current shape tool
        Key::Character(c) if c.as_str() == "a" && has_selection => Some(Msg::shape_mode_toggle()),
        // Shift+D: cycle to next redact tool (redact/pixelate) and activate it
        Key::Character(c) if c.as_str() == "D" && modifiers.shift() && has_selection => {
            Some(Msg::cycle_redact_tool())
        }
        // D: toggle current redact tool
        Key::Character(c) if c.as_str() == "d" && has_selection => {
            Some(Msg::redact_tool_mode_toggle())
        }
        // OCR shortcut: if result exists, copy and close; otherwise start OCR
        Key::Character(c) if c.as_str() == "o" && has_ocr_result => Some(Msg::ocr_copy_and_close()),
        Key::Character(c) if c.as_str() == "o" && has_selection => Some(Msg::ocr_requested()),
        // QR shortcut: if result exists, copy and close; otherwise start scan
        Key::Character(c) if c.as_str() == "q" && has_qr_result => Some(Msg::qr_copy_and_close()),
        Key::Character(c) if c.as_str() == "q" && has_selection => Some(Msg::qr_requested()),
        // Shift+R: trigger recording (only when region is selected)
        Key::Character(c) if c.as_str() == "R" && modifiers.shift() && has_selection => {
            Some(Msg::record_region())
        }
        // Selection mode shortcuts (always available, but not when in draw mode)
        // Use current_output_index (the screen where this key was pressed)
        Key::Character(c) if c.as_str() == "r" && !arrow_mode && !redact_mode => {
            Some(Msg::region_mode())
        }
        Key::Character(c) if c.as_str() == "s" && !arrow_mode && !redact_mode => {
            Some(Msg::screen_mode(current_output_index))
        }
        _ => None,
    }
}
