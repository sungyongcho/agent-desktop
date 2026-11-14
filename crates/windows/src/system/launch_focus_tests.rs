use super::*;

const USER: Foreground = Foreground {
    handle: 10,
    pid: 100,
};
const LAUNCHED: u32 = 200;

#[test]
fn a_foreground_window_of_the_launched_process_is_stolen_focus() {
    let now = Foreground {
        handle: 20,
        pid: LAUNCHED,
    };
    assert_eq!(judge(USER, Some(now), LAUNCHED), Look::Stolen);
}

#[test]
fn the_unchanged_foreground_keeps_waiting() {
    assert_eq!(judge(USER, Some(USER), LAUNCHED), Look::Waiting);
}

#[test]
fn a_window_of_another_process_is_never_touched() {
    let switched = Foreground {
        handle: 30,
        pid: 300,
    };
    assert_eq!(judge(USER, Some(switched), LAUNCHED), Look::Elsewhere);
    assert_eq!(judge(USER, None, LAUNCHED), Look::Elsewhere);
}

#[test]
fn a_background_launch_asks_for_no_activation() {
    let show = startup_show(false);
    assert_eq!(show.flags, 0x1);
    assert_eq!(show.show, 4);
}

#[test]
fn an_activating_launch_keeps_the_default_startup_show() {
    assert_eq!(startup_show(true), StartupShow { flags: 0, show: 0 });
}
