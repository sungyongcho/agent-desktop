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
    let plan = WindowRestorePlan::decide(Some(true), Some(frame(0.0, 1496.0)), Some(&record()));
    assert_eq!(plan, WindowRestorePlan::Unminimize);
}

#[test]
fn a_window_still_at_its_maximized_frame_goes_back_to_the_recorded_frame() {
    let plan = WindowRestorePlan::decide(Some(false), Some(frame(1.0, 1495.0)), Some(&record()));
    assert_eq!(plan, WindowRestorePlan::SetFrame(frame(100.0, 400.0)));
}

#[test]
fn a_window_moved_since_or_without_a_record_has_nothing_to_undo() {
    let moved = WindowRestorePlan::decide(Some(false), Some(frame(300.0, 1496.0)), Some(&record()));
    let unrecorded = WindowRestorePlan::decide(Some(false), Some(frame(0.0, 1496.0)), None);
    assert_eq!(moved, WindowRestorePlan::AlreadyNormal);
    assert_eq!(unrecorded, WindowRestorePlan::AlreadyNormal);
}
