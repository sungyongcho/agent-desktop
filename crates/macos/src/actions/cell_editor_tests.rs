use super::{Replacement, replacement};

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
