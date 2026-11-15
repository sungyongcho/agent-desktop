use super::*;

/// Code review found the `IsPassword` divergence `withholds_content` guards
/// against - a provider answering a documented-`VT_BOOL` property as
/// `VT_I4(1)` - reaching every other gate too, because `gate_open` read only
/// `flag()`. Both halves of the fix are pinned in one test: a nonzero
/// `Known` number opens the gate `withholds_content` already accepts, and a
/// gate nothing read still does not open - without the second assertion a
/// fix that opened every unread gate unconditionally would also pass.
#[test]
fn a_nonzero_known_number_opens_the_gate_but_an_unread_gate_still_does_not() {
    let opened_by_number = ElementProperties::from_reads(vec![
        (
            TreeProperty::ToggleAvailable,
            PropertyOutcome::Known(PropertyValue::Number(1)),
        ),
        (
            TreeProperty::ToggleState,
            PropertyOutcome::Known(PropertyValue::Number(1)),
        ),
    ]);
    assert_eq!(
        opened_by_number.gated_number(TreeProperty::ToggleState),
        Some(1),
        "a provider answering IsTogglePatternAvailable as VT_I4(1) must open the gate, \
         the same way withholds_content already accepts a nonzero VT_I4 IsPassword"
    );

    let gate_never_read = ElementProperties::from_reads(vec![(
        TreeProperty::ToggleState,
        PropertyOutcome::Known(PropertyValue::Number(1)),
    )]);
    assert_eq!(
        gate_never_read.gated_number(TreeProperty::ToggleState),
        None,
        "a gate nothing read must stay closed - reading an unread gate as open \
         would be a worse bug than the one being fixed"
    );
}

/// The same divergence reaches an `…Available` property read directly
/// through `is_true`, not only a state property read through its gate: the
/// `…Available` properties are themselves ungated, so `gated_flag`'s own
/// terminal read - not `gate_open` - is what a caller like
/// `resolve_actions`/`resolve_role` depends on here.
#[test]
fn is_true_reads_a_nonzero_known_number_on_an_ungated_available_property() {
    let properties = ElementProperties::from_reads(vec![(
        TreeProperty::ToggleAvailable,
        PropertyOutcome::Known(PropertyValue::Number(1)),
    )]);

    assert!(
        properties.is_true(TreeProperty::ToggleAvailable),
        "IsTogglePatternAvailable answered as VT_I4(1) must be true, the same way \
         withholds_content already accepts a nonzero VT_I4 IsPassword"
    );
}

fn value_read(class: &str, value: &str) -> LocatorField<String> {
    ElementProperties::from_reads(vec![
        (
            TreeProperty::ClassName,
            PropertyOutcome::Known(PropertyValue::Text(class.into())),
        ),
        (
            TreeProperty::Value,
            PropertyOutcome::Known(PropertyValue::Text(value.into())),
        ),
    ])
    .locator_evidence(ResolvedVocabulary::unknown())
    .value
}

/// Character Map's emptied RichEdit read back `"\r"`, so `get` reported a
/// cleared field as non-empty. Only the one final mark of a RichEdit goes;
/// a plain edit's value and a RichEdit's inner breaks stay verbatim.
#[test]
fn a_rich_edit_value_drops_only_its_final_paragraph_mark() {
    assert_eq!(
        value_read("RICHEDIT50W", "\r"),
        LocatorField::Known(String::new())
    );
    assert_eq!(
        value_read("RichEdit50W", "a\rb\r"),
        LocatorField::Known("a\rb".into())
    );
    assert_eq!(
        value_read("Edit", "hi\r"),
        LocatorField::Known("hi\r".into())
    );
    assert_eq!(
        value_read("RichEdit20W", "hi"),
        LocatorField::Known("hi".into())
    );
}

fn number(value: i32) -> PropertyOutcome {
    PropertyOutcome::Known(PropertyValue::Number(value))
}

fn flag(value: bool) -> PropertyOutcome {
    PropertyOutcome::Known(PropertyValue::Flag(value))
}

fn identifiers_of(reads: Vec<(TreeProperty, PropertyOutcome)>) -> Vec<(IdentifierKind, String)> {
    ElementProperties::from_reads(reads)
        .locator_evidence(ResolvedVocabulary::unknown())
        .identifiers
        .identifiers()
        .iter()
        .map(|identifier| (identifier.kind, identifier.value.clone()))
        .collect()
}

/// A grid item answers its zero-based position; that address leads the
/// evidence and the automation id stays behind it.
#[test]
fn a_grid_item_puts_its_cell_address_before_its_automation_id() {
    let reads = vec![
        (TreeProperty::GridItemAvailable, flag(true)),
        (TreeProperty::GridItemRow, number(6)),
        (TreeProperty::GridItemColumn, number(1)),
        (
            TreeProperty::AutomationId,
            PropertyOutcome::Known(PropertyValue::Text("cell".into())),
        ),
    ];
    assert_eq!(
        identifiers_of(reads),
        vec![
            (IdentifierKind::GridCell, "B7".to_string()),
            (IdentifierKind::AutomationId, "cell".to_string()),
        ]
    );
}

/// UI Automation answers a default 0 for a pattern the provider lacks, so an
/// element without the grid item pattern must not become cell A1.
#[test]
fn an_element_without_the_grid_item_pattern_gets_no_cell_address() {
    let reads = vec![
        (TreeProperty::GridItemAvailable, flag(false)),
        (TreeProperty::GridItemRow, number(0)),
        (TreeProperty::GridItemColumn, number(0)),
    ];
    assert!(identifiers_of(reads).is_empty());
}

/// A position that cannot be read keeps today's identity instead of failing.
#[test]
fn an_unreadable_position_keeps_the_automation_id_alone() {
    let reads = vec![
        (TreeProperty::GridItemAvailable, flag(true)),
        (TreeProperty::GridItemRow, PropertyOutcome::Unknown),
        (TreeProperty::GridItemColumn, number(2)),
        (
            TreeProperty::AutomationId,
            PropertyOutcome::Known(PropertyValue::Text("cell".into())),
        ),
    ];
    assert_eq!(
        identifiers_of(reads),
        vec![(IdentifierKind::AutomationId, "cell".to_string())]
    );
}
