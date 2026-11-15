use super::{Replacement, nothing_typed, replacement, write_outcome};
use agent_desktop_core::{DeliveryObservation, DeliveryOutcome, DeliverySemantics, ErrorCode};

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
fn a_write_is_verified_only_when_the_text_was_confirmed_and_the_commit_sent() {
    let verified = DeliveryOutcome::Delivered(DeliveryObservation::Verified);
    let unverified = DeliveryOutcome::Delivered(DeliveryObservation::Unverified);

    assert_eq!(write_outcome(verified, true), verified);
    assert_eq!(write_outcome(DeliveryOutcome::Satisfied, true), verified);
    assert_eq!(write_outcome(verified, false), unverified);
    assert_eq!(write_outcome(unverified, true), unverified);
}

#[test]
fn a_cell_that_was_pressed_and_gave_no_editor_fails_as_delivered() {
    assert_eq!(nothing_typed(false).unwrap(), DeliveryOutcome::NotDelivered);

    let error = nothing_typed(true).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}
