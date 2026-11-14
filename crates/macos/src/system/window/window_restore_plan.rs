use agent_desktop_core::{AdapterError, Rect, WindowFrameRecord};

use crate::system::window::window_postcondition::TOLERANCE;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum WindowRestorePlan {
    Unminimize,
    SetFrame(Rect),
    AlreadyNormal,
}

impl WindowRestorePlan {
    /// A window that is minimized, or whose state cannot be read, is
    /// un-minimized. A window that is not minimized has something to undo
    /// only while it still holds the frame its maximize produced, and a frame
    /// that cannot be read is an error, so the record is kept for a retry.
    pub(crate) fn decide(
        minimized: Option<bool>,
        record: Option<&WindowFrameRecord>,
        current: impl FnOnce() -> Result<Rect, AdapterError>,
    ) -> Result<Self, AdapterError> {
        if minimized != Some(false) {
            return Ok(Self::Unminimize);
        }
        let Some(record) = record else {
            return Ok(Self::AlreadyNormal);
        };
        Ok(if is_maximized_at(record, current()?) {
            Self::SetFrame(record.before)
        } else {
            Self::AlreadyNormal
        })
    }
}

pub(crate) fn is_maximized_at(record: &WindowFrameRecord, frame: Rect) -> bool {
    let held = record.maximized;
    [
        (frame.x, held.x),
        (frame.y, held.y),
        (frame.width, held.width),
        (frame.height, held.height),
    ]
    .into_iter()
    .all(|(left, right)| (left - right).abs() <= TOLERANCE)
}

#[cfg(test)]
#[path = "window_restore_plan_tests.rs"]
mod tests;
