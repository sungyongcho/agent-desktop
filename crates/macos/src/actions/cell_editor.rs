//! Text delivery to the editor of a table cell that holds its application's
//! keyboard focus.
//!
//! A cell editor can acknowledge an accessibility value write and ignore it,
//! while its application handles key events addressed to the process without
//! being activated. The text therefore goes by key events to the process: the
//! application stays where it is, and the editor is confirmed as the focus
//! holder before every chunk.

use std::time::Duration;

use agent_desktop_core::{
    AdapterError, Deadline, DeliveryOutcome, DeliverySemantics, ErrorCode, KeyCombo,
};

use crate::actions::chain::press_selection;
use crate::actions::{ax_helpers, background_keyboard};
use crate::tree::AXElement;
use crate::tree::properties::attributes;

pub(crate) const TYPE_LABEL: &str = "PIDTargetedUnicodeText";
pub(crate) const REPLACE_LABEL: &str = "PIDTargetedReplaceText";
pub(crate) const WRITE_LABEL: &str = "CellEditorWrite";

const MAX_PRESSES: usize = 2;
const NO_EDITOR_SUGGESTION: &str = "Take a fresh snapshot before another write: the press can select the cell or open another surface.";

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

/// How a value write reaches an element through a cell's in-place editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellTarget {
    Cell,
    Editor,
    Neither,
}

/// Classifies `element` for a value write. A read that fails says `Neither`:
/// the element then gets its ordinary chain, which reports the failure.
pub(crate) fn target(element: &AXElement, deadline: Deadline) -> CellTarget {
    if element.0.is_null() {
        return CellTarget::Neither;
    }
    let role = ax_helpers::element_role(element, deadline).ok().flatten();
    if role.as_deref() == Some(CELL_ROLE) {
        CellTarget::Cell
    } else if holds_focus(element, deadline).unwrap_or(false) {
        CellTarget::Editor
    } else {
        CellTarget::Neither
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

/// Writes `value` into a cell that opens a text editor when it is pressed.
///
/// The cell is pressed, the way a click presses it, until its editor holds
/// the application's keyboard focus, at most twice, because a first press can
/// only select the cell. The
/// editor's text is then replaced and Return commits it. Text goes only to an
/// editor that sits inside this cell, so it cannot land in another cell. A
/// cell that was pressed and gave no editor fails: nothing was typed.
pub(crate) fn write(
    cell: &AXElement,
    value: &str,
    deadline: Deadline,
) -> Result<DeliveryOutcome, AdapterError> {
    if ax_helpers::element_role(cell, deadline)?.as_deref() != Some(CELL_ROLE) {
        return Ok(DeliveryOutcome::NotDelivered);
    }
    let mut pressed = false;
    let typed = match open_editor(cell, deadline, &mut pressed)? {
        Some(editor) => replace(&editor, value, deadline)?,
        None => DeliveryOutcome::NotDelivered,
    };
    if typed == DeliveryOutcome::NotDelivered {
        return nothing_typed(pressed);
    }
    let committed = background_keyboard::press_text_focus_holder(cell, &return_key(), deadline)?;
    Ok(write_outcome(typed, committed))
}

/// The end of a write that typed nothing. A press that was sent stays on
/// record as a failure, so the caller does not read it as a write.
pub(crate) fn nothing_typed(pressed: bool) -> Result<DeliveryOutcome, AdapterError> {
    if !pressed {
        return Ok(DeliveryOutcome::NotDelivered);
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        "The cell was pressed and gave no text editor to type into; nothing was typed",
    )
    .with_suggestion(NO_EDITOR_SUGGESTION)
    .with_disposition(DeliverySemantics::delivered_unverified()))
}

/// The editor text was confirmed before the commit, and the commit was sent:
/// only then is the write verified.
pub(crate) fn write_outcome(typed: DeliveryOutcome, committed: bool) -> DeliveryOutcome {
    let confirmed = matches!(
        typed,
        DeliveryOutcome::Satisfied
            | DeliveryOutcome::Delivered(agent_desktop_core::DeliveryObservation::Verified)
    );
    DeliveryOutcome::from_delivery(true, confirmed && committed)
}

fn open_editor(
    cell: &AXElement,
    deadline: Deadline,
    pressed: &mut bool,
) -> Result<Option<AXElement>, AdapterError> {
    if let Some(editor) = editor_of(cell, deadline)? {
        return Ok(Some(editor));
    }
    for _ in 0..MAX_PRESSES {
        let press = press_selection::press_or_select_container(cell, deadline)?;
        if press == DeliveryOutcome::NotDelivered {
            return Ok(None);
        }
        *pressed = true;
        let read = || editor_of(cell, deadline);
        let opened =
            deadline.poll_until(SETTLE, SETTLE_POLL, read, |read| !matches!(read, Ok(None)))?;
        if opened.is_some() {
            return Ok(opened);
        }
    }
    Ok(None)
}

/// The text element inside `cell` that holds the application's keyboard focus.
fn editor_of(cell: &AXElement, deadline: Deadline) -> Result<Option<AXElement>, AdapterError> {
    let Some(pid) =
        crate::system::app::app_ops::pid_from_element(cell, deadline).filter(|pid| *pid > 0)
    else {
        return Ok(None);
    };
    let Some(holder) = background_keyboard::text_focus_holder(pid, deadline)? else {
        return Ok(None);
    };
    let parent = attributes::copy_element_attr_result(&holder, "AXParent", deadline);
    deadline.ensure_remaining()?;
    Ok(parent
        .ok()
        .flatten()
        .is_some_and(|parent| crate::tree::same_element(&parent, cell))
        .then_some(holder))
}

fn return_key() -> KeyCombo {
    KeyCombo {
        key: "return".into(),
        modifiers: vec![],
    }
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
