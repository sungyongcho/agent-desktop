use super::baseline::Baseline;
use super::*;

fn empty_baseline() -> Baseline {
    Baseline {
        processes: HashSet::new(),
        residents: Vec::new(),
    }
}

fn launch_with<'a>(
    bus: &'a SharedBus,
    options: &'a LaunchOptions,
    deadline: Deadline,
) -> Launch<'a> {
    Launch {
        bus,
        id: "fixture",
        options,
        lease: Some(InteractionLease::guarded(deadline, ()).unwrap()),
        deadline,
    }
}

#[test]
fn a_background_launch_releases_the_lease_once_the_focus_step_ends() {
    let bus = SharedBus::default();
    let options = LaunchOptions::default();
    let mut launch = launch_with(&bus, &options, Deadline::after(1_000).unwrap());
    assert!(!launch.return_focus(None, 1, &empty_baseline()));
    assert!(launch.lease.is_none());
}

#[test]
fn an_activating_launch_keeps_the_lease_for_its_focus_request() {
    let bus = SharedBus::default();
    let options = LaunchOptions {
        activate: true,
        ..LaunchOptions::default()
    };
    let mut launch = launch_with(&bus, &options, Deadline::after(1_000).unwrap());
    assert!(!launch.return_focus(None, 1, &empty_baseline()));
    assert!(launch.lease.is_some());
}
