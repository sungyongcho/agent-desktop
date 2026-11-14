//! A table cell's position and a table's size, read from the table interface
//! of the accessibility API. Only elements whose role is a cell or a table pay
//! for the extra reads.

use agent_desktop_core::{ElementIdentifier, NodeDescriptor};

const CELL_ROLE: &str = "AXCell";
const TABLE_ROLES: [&str; 3] = ["AXTable", "AXGrid", "AXOutline"];

pub(crate) fn is_cell_role(role: Option<&str>) -> bool {
    role == Some(CELL_ROLE)
}

pub(crate) fn is_table_role(role: Option<&str>) -> bool {
    role.is_some_and(|role| TABLE_ROLES.contains(&role))
}

pub(crate) fn cell_identifier(row: Option<i64>, column: Option<i64>) -> Option<ElementIdentifier> {
    let row = u32::try_from(row?).ok()?;
    let column = u32::try_from(column?).ok()?;
    Some(ElementIdentifier::grid_cell(row, column))
}

pub(crate) fn table_size(rows: Option<i64>, columns: Option<i64>) -> NodeDescriptor {
    NodeDescriptor {
        row_count: rows.and_then(|count| u32::try_from(count).ok()),
        column_count: columns.and_then(|count| u32::try_from(count).ok()),
        ..NodeDescriptor::default()
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use accessibility_sys::{
        AXValueGetTypeID, AXValueGetValue, kAXErrorSuccess, kAXValueTypeCFRange,
    };
    use agent_desktop_core::{Deadline, ElementIdentifier, NodeDescriptor};
    use core_foundation::{
        base::{CFType, TCFType},
        number::CFNumber,
        string::CFString,
    };
    use core_foundation_sys::base::CFRange;

    use crate::tree::AXElement;

    fn copy_value(element: &AXElement, attribute: &str, deadline: Deadline) -> Option<CFType> {
        let name = CFString::new(attribute);
        let (error, raw) = crate::tree::properties::ax_ipc::copy_attribute_value(
            element,
            name.as_concrete_TypeRef(),
            deadline,
        );
        if raw.is_null() {
            return None;
        }
        let value = unsafe { CFType::wrap_under_create_rule(raw) };
        (error == kAXErrorSuccess).then_some(value)
    }

    fn read_count(element: &AXElement, attribute: &str, deadline: Deadline) -> Option<i64> {
        copy_value(element, attribute, deadline)?
            .downcast::<CFNumber>()?
            .to_i64()
    }

    fn read_range_start(element: &AXElement, attribute: &str, deadline: Deadline) -> Option<i64> {
        let value = copy_value(element, attribute, deadline)?;
        if value.type_of() != unsafe { AXValueGetTypeID() } {
            return None;
        }
        let mut range = CFRange {
            location: 0,
            length: 0,
        };
        let decoded = unsafe {
            AXValueGetValue(
                value.as_CFTypeRef() as _,
                kAXValueTypeCFRange,
                (&mut range as *mut CFRange).cast(),
            )
        };
        decoded.then_some(i64::try_from(range.location).ok()?)
    }

    pub(crate) fn read_cell_identifier(
        element: &AXElement,
        deadline: Deadline,
    ) -> Option<ElementIdentifier> {
        super::cell_identifier(
            read_range_start(element, "AXRowIndexRange", deadline),
            read_range_start(element, "AXColumnIndexRange", deadline),
        )
    }

    pub(crate) fn read_table_size(element: &AXElement, deadline: Deadline) -> NodeDescriptor {
        super::table_size(
            read_count(element, "AXRowCount", deadline),
            read_count(element, "AXColumnCount", deadline),
        )
    }
}

#[cfg(target_os = "macos")]
pub(crate) use imp::{read_cell_identifier, read_table_size};

#[cfg(test)]
#[path = "grid_position_tests.rs"]
mod tests;
