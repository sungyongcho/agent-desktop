//! Keyboard input for an element that already holds its application's
//! keyboard focus. The key events go to the application's process only: the
//! application is not activated, and the user's own keyboard input is not
//! touched. Nothing is sent unless the element is the focus holder, and that
//! is confirmed again before every chunk, so text cannot land elsewhere.

use agent_desktop_core::{AdapterError, Deadline, KeyCombo};

use crate::actions::physical_keyboard::{
    focused_element_matches, lost_focus, owning_process, require_same_process,
};
use crate::system::process::process_identity::ProcessIdentity;
use crate::tree::AXElement;

/// Whether `element` is the element its application reports as its keyboard
/// focus. The application does not have to be active.
pub(crate) fn holds_focus(element: &AXElement, deadline: Deadline) -> Result<bool, AdapterError> {
    match crate::system::app::app_ops::pid_from_element(element, deadline).filter(|pid| *pid > 0) {
        Some(pid) => focused_element_matches(element, pid, deadline),
        None => Ok(false),
    }
}

pub(crate) fn type_text(
    element: &AXElement,
    text: &str,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    crate::input::keyboard::preflight_text(text, deadline)?;
    let identity = owning_process(element, deadline)?;
    crate::input::keyboard::synthesize_text(text, identity.pid(), deadline, |deadline| {
        require_focus_holder(element, identity, deadline)
    })
}

pub(crate) fn press(
    element: &AXElement,
    combo: &KeyCombo,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    let identity = owning_process(element, deadline)?;
    require_focus_holder(element, identity, deadline)?;
    crate::input::keyboard::synthesize_key(combo, Some(identity.pid()), deadline)
}

fn require_focus_holder(
    expected: &AXElement,
    identity: ProcessIdentity,
    deadline: Deadline,
) -> Result<(), AdapterError> {
    require_same_process(identity)?;
    if focused_element_matches(expected, identity.pid(), deadline)? {
        return Ok(());
    }
    Err(lost_focus())
}
