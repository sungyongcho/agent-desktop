use super::*;
use agent_desktop_core::ProcessIdentity;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn record() -> WindowFrameRecord {
    WindowFrameRecord {
        process: ProcessIdentity::new(7u32, "boot"),
        before: rect(100.0, 30.0, 400.0, 300.0),
        maximized: rect(0.0, 30.0, 1496.0, 900.0),
    }
}

#[test]
fn a_full_maximize_is_ours() {
    assert_eq!(
        FramePlace::of(&record(), rect(0.0, 30.0, 1496.0, 900.0)),
        FramePlace::Ours
    );
}

#[test]
fn a_move_whose_resize_did_not_land_is_ours() {
    assert_eq!(
        FramePlace::of(&record(), rect(0.0, 30.0, 400.0, 300.0)),
        FramePlace::Ours
    );
}

#[test]
fn a_resize_whose_move_did_not_land_is_ours() {
    assert_eq!(
        FramePlace::of(&record(), rect(100.0, 30.0, 1496.0, 900.0)),
        FramePlace::Ours
    );
}

#[test]
fn the_recorded_frame_is_before() {
    assert_eq!(
        FramePlace::of(&record(), rect(100.0, 30.0, 400.0, 300.0)),
        FramePlace::Before
    );
}

#[test]
fn a_frame_moved_somewhere_unrelated_is_elsewhere() {
    assert_eq!(
        FramePlace::of(&record(), rect(300.0, 80.0, 700.0, 500.0)),
        FramePlace::Elsewhere
    );
}

#[test]
fn drift_within_tolerance_still_matches() {
    assert_eq!(
        FramePlace::of(&record(), rect(1.0, 31.0, 1495.0, 899.0)),
        FramePlace::Ours
    );
}
