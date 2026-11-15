use super::*;
use crate::tree::test_support::grid_fake::{grid, grid_with};

#[test]
fn the_rectangle_between_two_corners_runs_row_by_row() {
    assert_eq!(
        rectangle((1, 2), (2, 3), 10),
        vec![(1, 2), (1, 3), (2, 2), (2, 3)]
    );
    assert_eq!(rectangle((2, 3), (1, 2), 3), vec![(1, 2), (1, 3), (2, 2)]);
}

#[test]
fn a_table_without_a_hit_at_its_corners_presents_no_view() {
    let (client, element, _calls, _server) = grid(1_000_000, 10, 6);
    let missing = AtspiElement::new(element.bus_name(), "/missing", 1);
    let view = zbus::block_on(read_table_view(&client, &missing, 4096, Liveness::Alive));
    assert!(matches!(view, Ok(None)) || view.is_err());
}

#[test]
fn the_view_of_a_table_is_its_cells_with_their_indexes() {
    let (client, element, _calls, _server) = grid(1_000_000, 10, 6);
    let cells = zbus::block_on(read_table_view(&client, &element, 4096, Liveness::Alive))
        .expect("the view reads")
        .expect("the table has a view");
    assert_eq!(cells.len(), 60);
    assert_eq!(cells[11].1, 11);
    assert_eq!(cells[11].0.1, "/cell/11");
}

#[test]
fn a_toolkit_that_answers_with_new_cell_objects_still_has_a_view() {
    let (client, element, _calls, _server) = grid_with(1_000_000, 10, 6, true);
    let cells = zbus::block_on(read_table_view(&client, &element, 4096, Liveness::Alive))
        .expect("the view reads")
        .expect("the table has a view");
    assert_eq!(cells.len(), 60);
    assert_eq!(cells[0].0.1, "/fresh/0");
}
