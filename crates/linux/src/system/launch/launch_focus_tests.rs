use super::*;
use crate::system::x11::X11Client;

fn client(id: u32, pid: u32) -> X11Client {
    X11Client {
        id,
        pid: Some(pid),
        title: String::new(),
        extents: None,
        bounds: None,
        hidden: false,
        shadow: (0, 0, 0, 0),
    }
}

fn prior() -> Prior {
    Prior {
        client: 1,
        owner: ProcessIdentity::new(10, "i"),
        known: HashSet::from([1]),
    }
}

fn state(active: u32, clients: Vec<X11Client>) -> X11State {
    X11State::Ready {
        active: Some(active),
        clients,
    }
}

fn launched(pid: u32) -> bool {
    pid == 20
}

#[test]
fn a_new_window_of_the_launch_that_is_active_took_the_focus() {
    let now = state(2, vec![client(1, 10), client(2, 20)]);
    assert_eq!(judge(&prior(), &now, &launched), Look::Stolen);
}

#[test]
fn a_window_the_user_switched_to_is_never_taken_back() {
    let now = state(3, vec![client(1, 10), client(2, 20), client(3, 30)]);
    assert_eq!(judge(&prior(), &now, &launched), Look::Appeared);
}

#[test]
fn a_prior_window_that_closed_has_no_focus_to_return() {
    let now = state(2, vec![client(2, 20)]);
    assert_eq!(judge(&prior(), &now, &launched), Look::Appeared);
}

#[test]
fn an_old_window_of_the_application_is_not_the_launch() {
    let mut old = prior();
    old.known.insert(2);
    let now = state(2, vec![client(1, 10), client(2, 20)]);
    assert_eq!(judge(&old, &now, &launched), Look::Nothing);
}
