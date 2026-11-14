use super::*;
use agent_desktop_core::ProcessIdentity;

fn frame(x: f64, width: f64) -> Rect {
    Rect {
        x,
        y: 30.0,
        width,
        height: 500.0,
    }
}

fn record() -> WindowFrameRecord {
    WindowFrameRecord {
        process: ProcessIdentity::new(7u32, "boot"),
        before: frame(100.0, 400.0),
        maximized: frame(0.0, 1496.0),
    }
}

#[test]
fn a_minimized_window_is_unminimized_whatever_was_recorded() {
    let plan = WindowRestorePlan::decide(Some(true), Some(&record()), || Ok(frame(0.0, 1496.0)));
    assert_eq!(plan.ok(), Some(WindowRestorePlan::Unminimize));
}

#[test]
fn a_window_still_at_its_maximized_frame_goes_back_to_the_recorded_frame() {
    let plan = WindowRestorePlan::decide(Some(false), Some(&record()), || Ok(frame(1.0, 1495.0)));
    assert_eq!(
        plan.ok(),
        Some(WindowRestorePlan::SetFrame(frame(100.0, 400.0)))
    );
}

#[test]
fn a_window_moved_since_or_without_a_record_has_nothing_to_undo() {
    let moved =
        WindowRestorePlan::decide(Some(false), Some(&record()), || Ok(frame(300.0, 1496.0)));
    let unrecorded = WindowRestorePlan::decide(Some(false), None, || Ok(frame(0.0, 1496.0)));
    assert_eq!(moved.ok(), Some(WindowRestorePlan::AlreadyNormal));
    assert_eq!(unrecorded.ok(), Some(WindowRestorePlan::AlreadyNormal));
}

#[test]
fn an_unreadable_frame_is_an_error_so_the_record_is_kept() {
    let plan = WindowRestorePlan::decide(Some(false), Some(&record()), || {
        Err(AdapterError::internal("window frame is unreadable"))
    });
    assert!(plan.is_err());
}

#[test]
fn a_minimized_window_is_unminimized_without_reading_its_frame() {
    let plan = WindowRestorePlan::decide(Some(true), Some(&record()), || {
        panic!("frame must not be read")
    });
    assert_eq!(plan.ok(), Some(WindowRestorePlan::Unminimize));
}

#[test]
fn an_unminimized_window_without_a_record_is_not_read() {
    let plan = WindowRestorePlan::decide(Some(false), None, || panic!("frame must not be read"));
    assert_eq!(plan.ok(), Some(WindowRestorePlan::AlreadyNormal));
}
