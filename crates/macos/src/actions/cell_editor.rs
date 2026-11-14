//! Text delivery to the editor of a table cell that holds its application's
//! keyboard focus.
//!
//! A cell editor can acknowledge an accessibility value write and ignore it,
//! while its application handles key events addressed to the process without
//! being activated. The text therefore goes by key events to the process: the
//! application stays where it is, and the editor is confirmed as the focus
//! holder before every chunk.

use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliveryOutcome, KeyCombo};

use crate::actions::{ax_helpers, background_keyboard};
use crate::tree::AXElement;
use crate::tree::properties::attributes;

pub(crate) const TYPE_LABEL: &str = "PIDTargetedUnicodeText";
pub(crate) const REPLACE_LABEL: &str = "PIDTargetedReplaceText";

const CELL_ROLE: &str = "cell";
const SETTLE: Duration = Duration::from_millis(600);
const SETTLE_POLL: Duration = Duration::from_millis(10);

/// What replacing an editor's text takes, given the text it holds now.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Replacement {
    AlreadyHeld,
    Type,
    SelectThenType(usize),
    SelectThenDelete(usize),
}

pub(crate) fn replacement(current: &str, wanted: &str) -> Replacement {
    let held = current.encode_utf16().count();
    match (held, wanted.is_empty()) {
        _ if current == wanted => Replacement::AlreadyHeld,
        (0, _) => Replacement::Type,
        (held, true) => Replacement::SelectThenDelete(held),
        (held, false) => Replacement::SelectThenType(held),
    }
}

/// Whether `element` sits directly inside a table cell and holds its
/// application's keyboard focus.
pub(crate) fn holds_focus(element: &AXElement, deadline: Deadline) -> Result<bool, AdapterError> {
    attributes::set_messaging_timeout(element, deadline)?;
    let parent = attributes::copy_element_attr_result(element, "AXParent", deadline);
    deadline.ensure_remaining()?;
    let Ok(Some(parent)) = parent else {
        return Ok(false);
    };
    if ax_helpers::element_role(&parent, deadline)?.as_deref() != Some(CELL_ROLE) {
        return Ok(false);
    }
    background_keyboard::holds_focus(element, deadline)
}

/// Replaces the whole text of a focused cell editor with `value`. Sends
/// nothing when `element` is not such an editor, or when the text it holds
/// cannot be selected first: typing would then add to it.
pub(crate) fn replace(
    element: &AXElement,
    value: &str,
    deadline: Deadline,
) -> Result<DeliveryOutcome, AdapterError> {
    if !holds_focus(element, deadline)? {
        return Ok(DeliveryOutcome::NotDelivered);
    }
    let current = crate::tree::copy_value_typed(element, deadline).unwrap_or_default();
    let selected = match replacement(&current, value) {
        Replacement::AlreadyHeld => return Ok(DeliveryOutcome::Satisfied),
        Replacement::Type => true,
        Replacement::SelectThenType(held) | Replacement::SelectThenDelete(held) => {
            select_all(element, held, deadline)
        }
    };
    if !selected {
        return Ok(DeliveryOutcome::NotDelivered);
    }
    if value.is_empty() {
        background_keyboard::press(element, &delete_key(), deadline)?;
    } else {
        background_keyboard::type_text(element, value, deadline)?;
    }
    Ok(DeliveryOutcome::from_delivery(
        true,
        settles_on(element, value, deadline),
    ))
}

fn select_all(element: &AXElement, held: usize, deadline: Deadline) -> bool {
    attributes::set_selected_text_range(element, 0..held, deadline)
        && attributes::selected_text_range(element, deadline) == Some(0..held)
}

fn settles_on(element: &AXElement, value: &str, deadline: Deadline) -> bool {
    let read = || crate::tree::copy_value_typed(element, deadline).unwrap_or_default() == value;
    deadline.poll_until(SETTLE, SETTLE_POLL, read, |held| *held)
}

fn delete_key() -> KeyCombo {
    KeyCombo {
        key: "delete".into(),
        modifiers: vec![],
    }
}

#[cfg(test)]
#[path = "cell_editor_tests.rs"]
mod tests;
