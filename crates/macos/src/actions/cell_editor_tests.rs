use super::{Replacement, left_open, nothing_typed, replacement};
use agent_desktop_core::{DeliveryOutcome, DeliverySemantics, ErrorCode};

#[test]
fn an_empty_editor_is_typed_into_without_a_selection() {
    assert_eq!(replacement("", "42"), Replacement::Type);
}

#[test]
fn held_text_is_selected_before_the_new_text_is_typed() {
    assert_eq!(replacement("41", "42"), Replacement::SelectThenType(2));
}

#[test]
fn held_text_is_selected_and_deleted_for_an_empty_value() {
    assert_eq!(replacement("41", ""), Replacement::SelectThenDelete(2));
}

#[test]
fn an_editor_that_already_holds_the_value_gets_nothing() {
    assert_eq!(replacement("42", "42"), Replacement::AlreadyHeld);
    assert_eq!(replacement("", ""), Replacement::AlreadyHeld);
}

#[test]
fn the_selection_length_counts_utf16_units() {
    assert_eq!(replacement("é😀", "x"), Replacement::SelectThenType(3));
}

#[test]
fn a_cell_that_was_pressed_and_gave_no_editor_fails_as_delivered() {
    assert_eq!(nothing_typed(false).unwrap(), DeliveryOutcome::NotDelivered);

    let error = nothing_typed(true).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn text_left_in_an_editor_the_application_moved_fails_as_delivered() {
    assert_eq!(left_open(false).unwrap(), DeliveryOutcome::NotDelivered);

    let error = left_open(true).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}
