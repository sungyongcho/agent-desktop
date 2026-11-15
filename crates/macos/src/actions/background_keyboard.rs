//! Keyboard input for an element that already holds its application's
//! keyboard focus. The key events go to the application's process only: the
//! application is not activated, and the user's own keyboard input is not
//! touched. Nothing is sent unless the element is the focus holder, and that
//! is confirmed again before every chunk, so text cannot land elsewhere.

use agent_desktop_core::{AdapterError, Deadline, KeyCombo};

use crate::actions::physical_keyboard::{
    focused_element_matches, lost_focus, owning_process, require_same_process,
};
use crate::tree::properties::attributes;
use crate::tree::{AXElement, same_element};

/// Whether `element` is the element its application reports as its keyboard
/// focus. The application does not have to be active.
pub(crate) fn holds_focus(element: &AXElement, deadline: Deadline) -> Result<bool, AdapterError> {
    match crate::system::app::app_ops::pid_from_element(element, deadline).filter(|pid| *pid > 0) {
        Some(pid) => focused_element_matches(element, pid, deadline),
        None => Ok(false),
    }
}

/// The text element that holds the keyboard focus of `pid`'s application.
pub(crate) fn text_focus_holder(
    pid: i32,
    deadline: Deadline,
) -> Result<Option<AXElement>, AdapterError> {
    let app = crate::tree::element_for_pid(pid);
    attributes::set_messaging_timeout(&app, deadline)?;
    let holder = attributes::copy_element_attr_result(&app, "AXFocusedUIElement", deadline);
    deadline.ensure_remaining()?;
    let Ok(Some(holder)) = holder else {
        return Ok(None);
    };
    let role = attributes::copy_string_attr_result(&holder, "AXRole", deadline);
    deadline.ensure_remaining()?;
    let role = role.ok().flatten();
    Ok(crate::actions::type_text::takes_typed_text(role.as_deref()).then_some(holder))
}

/// Types `text` into `element`. The first chunk goes to `element` only. An
/// application can answer typed text by moving the edit to another text
/// element, as a formula editor that opens on the first character does; the
/// rest of the text then follows the edit there, the way a person's keys
/// would. Any other change of the focus holder stops the typing.
pub(crate) fn type_text(
    element: &AXElement,
    text: &str,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    crate::input::keyboard::preflight_text(text, deadline)?;
    let identity = owning_process(element, deadline)?;
    let mut started = false;
    crate::input::keyboard::synthesize_text(text, identity.pid(), deadline, |deadline| {
        require_same_process(identity)?;
        let holder = text_focus_holder(identity.pid(), deadline)?;
        let on_element = holder
            .as_ref()
            .is_some_and(|holder| same_element(holder, element));
        if on_element || (started && holder.is_some()) {
            started = true;
            return Ok(());
        }
        Err(lost_focus())
    })
}

/// Sends `combo` to `element`, which must hold the keyboard focus.
pub(crate) fn press(
    element: &AXElement,
    combo: &KeyCombo,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let identity = owning_process(element, deadline)?;
    require_same_process(identity)?;
    if !focused_element_matches(element, identity.pid(), deadline)? {
        return Err(lost_focus());
    }
    crate::input::keyboard::synthesize_key(combo, Some(identity.pid()), deadline)
}

/// Sends `combo` to the text element that holds the keyboard focus of the
/// application that owns `anchor`. Sends nothing when no text element does.
pub(crate) fn press_text_focus_holder(
    anchor: &AXElement,
    combo: &KeyCombo,
    deadline: Deadline,
) -> Result<bool, AdapterError> {
    let identity = owning_process(anchor, deadline)?;
    require_same_process(identity)?;
    if text_focus_holder(identity.pid(), deadline)?.is_none() {
        return Ok(false);
    }
    crate::input::keyboard::synthesize_key(combo, Some(identity.pid()), deadline)?;
    Ok(true)
}
