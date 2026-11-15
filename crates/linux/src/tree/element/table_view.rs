//! The cells of a table that are in view.
//!
//! A table that manages its descendants may report millions of cells. The
//! view is the rectangle of cells between the cell at the top-left corner and
//! the cell at the bottom-right corner of the table's own screen extents.
//! Each cell is read with `Table.GetAccessibleAt` and keeps the index it has
//! in the table, read with `Table.GetIndexAt`.

use agent_desktop_core::AdapterError;
use futures_util::stream::{self, StreamExt};
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::system::ACCESSIBLE_INTERFACE;
use crate::tree::properties::interface_reads::{SCREEN_COORDINATES, fetch_extents};
use crate::tree::properties::liveness::Liveness;
use crate::tree::properties::node_read::{
    COMPONENT_INTERFACE, NULL_PATH, call, deserialize, field,
};
use crate::tree::{AtspiElement, ChildAddress, Field};

pub(super) const TABLE_INTERFACE: &str = "org.a11y.atspi.Table";
const INSET: i32 = 5;
const CELLS_IN_FLIGHT: usize = 32;

type Cell = (ChildAddress, usize);

fn address_of(entry: (String, OwnedObjectPath)) -> Option<ChildAddress> {
    let (bus, path) = entry;
    (!bus.is_empty() && path.as_str() != NULL_PATH).then(|| (bus, path.to_string()))
}

async fn read_i32(
    connection: &Connection,
    element: &AtspiElement,
    interface: &str,
    method: &str,
    index: i32,
    liveness: Liveness,
) -> Result<Option<i32>, AdapterError> {
    let reply = call(connection, element, interface, method, &(index,)).await;
    Ok(field(deserialize::<i32>(reply), liveness)?
        .present()
        .copied()
        .filter(|value| *value >= 0))
}

async fn cell_at(
    connection: &Connection,
    table: &AtspiElement,
    row: i32,
    column: i32,
    liveness: Liveness,
) -> Result<Option<Cell>, AdapterError> {
    let position = (row, column);
    let at = call(
        connection,
        table,
        TABLE_INTERFACE,
        "GetAccessibleAt",
        &position,
    );
    let index = call(connection, table, TABLE_INTERFACE, "GetIndexAt", &position);
    let (at, index) = futures_util::future::join(at, index).await;
    let address = field(deserialize::<(String, OwnedObjectPath)>(at), liveness)?
        .present()
        .cloned()
        .and_then(address_of);
    let index = field(deserialize::<i32>(index), liveness)?
        .present()
        .copied()
        .and_then(|index| usize::try_from(index).ok());
    Ok(address.zip(index))
}

/// The row and column of the cell shown at one screen point. A hit whose
/// index in its parent does not map back to the same index through the
/// table's own row and column is not a cell of the grid and yields nothing.
/// The index is compared, not the object address: a toolkit that creates its
/// cell objects on demand answers with a new object for the same cell.
async fn cell_under(
    connection: &Connection,
    table: &AtspiElement,
    point: (i32, i32),
    liveness: Liveness,
) -> Result<Option<(i32, i32)>, AdapterError> {
    let reply = call(
        connection,
        table,
        COMPONENT_INTERFACE,
        "GetAccessibleAtPoint",
        &(point.0, point.1, SCREEN_COORDINATES),
    )
    .await;
    let Some(hit) = field(deserialize::<(String, OwnedObjectPath)>(reply), liveness)?
        .present()
        .cloned()
        .and_then(address_of)
    else {
        return Ok(None);
    };
    let cell = AtspiElement::new(&hit.0, &hit.1, table.pid());
    let reply = call(
        connection,
        &cell,
        ACCESSIBLE_INTERFACE,
        "GetIndexInParent",
        &(),
    )
    .await;
    let Some(index) = field(deserialize::<i32>(reply), liveness)?
        .present()
        .copied()
        .filter(|index| *index >= 0)
    else {
        return Ok(None);
    };
    let row = read_i32(
        connection,
        table,
        TABLE_INTERFACE,
        "GetRowAtIndex",
        index,
        liveness,
    );
    let column = read_i32(
        connection,
        table,
        TABLE_INTERFACE,
        "GetColumnAtIndex",
        index,
        liveness,
    );
    let (row, column) = futures_util::future::join(row, column).await;
    let (Some(row), Some(column)) = (row?, column?) else {
        return Ok(None);
    };
    let confirmed = cell_at(connection, table, row, column, liveness).await?;
    Ok(confirmed
        .filter(|(_, confirmed_index)| usize::try_from(index).ok() == Some(*confirmed_index))
        .map(|_| (row, column)))
}

fn rectangle(first: (i32, i32), second: (i32, i32), limit: usize) -> Vec<(i32, i32)> {
    let (rows, columns) = (
        first.0.min(second.0)..=first.0.max(second.0),
        first.1.min(second.1)..=first.1.max(second.1),
    );
    rows.flat_map(|row| columns.clone().map(move |column| (row, column)))
        .take(limit)
        .collect()
}

/// The cells in view of `table`, each with its index in the table, in index
/// order and at most `limit` of them. `None` when the table does not place
/// two cells at its corners, so the caller falls back to another reading.
pub(super) async fn read_table_view(
    connection: &Connection,
    table: &AtspiElement,
    limit: usize,
    liveness: Liveness,
) -> Result<Option<Vec<Cell>>, AdapterError> {
    let Field::Present(extents) = fetch_extents(connection, table, liveness).await? else {
        return Ok(None);
    };
    if extents.width <= 0 || extents.height <= 0 {
        return Ok(None);
    }
    let (inset_x, inset_y) = (INSET.min(extents.width / 2), INSET.min(extents.height / 2));
    let top_left = (extents.x + inset_x, extents.y + inset_y);
    let bottom_right = (
        extents.x + extents.width - inset_x - 1,
        extents.y + extents.height - inset_y - 1,
    );
    let (first, last) = futures_util::future::join(
        cell_under(connection, table, top_left, liveness),
        cell_under(connection, table, bottom_right, liveness),
    )
    .await;
    let (Some(first), Some(last)) = (first?, last?) else {
        return Ok(None);
    };
    let reads: Vec<Result<Option<Cell>, AdapterError>> = stream::iter(
        rectangle(first, last, limit)
            .into_iter()
            .map(|(row, column)| cell_at(connection, table, row, column, liveness)),
    )
    .buffered(CELLS_IN_FLIGHT)
    .collect()
    .await;
    let mut cells = Vec::with_capacity(reads.len());
    for read in reads {
        cells.extend(read?);
    }
    cells.sort_by_key(|(_, index)| *index);
    cells.dedup_by_key(|(_, index)| *index);
    Ok((!cells.is_empty()).then_some(cells))
}

#[cfg(test)]
#[path = "table_view_tests.rs"]
mod tests;
