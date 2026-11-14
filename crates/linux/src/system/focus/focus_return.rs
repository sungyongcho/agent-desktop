//! Gives the focus back after a launch that was not asked to take it. The
//! window system answers, not the accessibility bus, so an application that
//! never reaches the bus still has its focus return.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ProcessIdentity};

use crate::system::launch::launch_observe::POLL_INTERVAL;
use crate::system::process::process_facts;
use crate::system::x11::control::activate::{ACTIVATION_GRACE, active_window_within};
use crate::system::x11::control::control_send::activate_window;
use crate::system::x11::stacking::stacking_source::StackingSource;
use crate::system::x11::{DisplayReader, X11State};

const FOCUS_WAIT: Duration = Duration::from_millis(3_000);
const APPEARED_GRACE: Duration = Duration::from_millis(1_500);
const CONFIRM_WAIT: Duration = Duration::from_millis(700);
const RESTORE_ATTEMPTS: usize = 3;
const ACTIVE_READ: Duration = Duration::from_millis(500);

/// The window that had the focus before the spawn, and every window open then.
pub(crate) struct Prior {
    client: u32,
    owner: ProcessIdentity,
    known: HashSet<u32>,
}

#[derive(Debug, PartialEq, Eq)]
enum Look {
    Stolen,
    Appeared,
    Nothing,
}

pub(crate) fn read_prior(deadline: Deadline) -> Option<Prior> {
    let X11State::Ready {
        active: Some(active),
        clients,
    } = DisplayReader.clients_within(deadline)
    else {
        return None;
    };
    let pid = clients.iter().find(|c| c.id == active)?.pid?;
    let facts = process_facts::read(pid)?;
    Some(Prior {
        client: active,
        owner: ProcessIdentity::new(pid, facts.instance),
        known: clients.iter().map(|c| c.id).collect(),
    })
}

fn judge(prior: &Prior, state: &X11State, belongs: &impl Fn(u32) -> bool) -> Look {
    let X11State::Ready { active, clients } = state else {
        return Look::Nothing;
    };
    let launched = |id: u32| {
        clients
            .iter()
            .any(|c| c.id == id && !prior.known.contains(&id) && c.pid.is_some_and(belongs))
    };
    let prior_open = clients.iter().any(|c| c.id == prior.client);
    if prior_open && active.is_some_and(launched) {
        Look::Stolen
    } else if clients.iter().any(|c| launched(c.id)) {
        Look::Appeared
    } else {
        Look::Nothing
    }
}

/// Process group of `pid`, read from `/proc/<pid>/stat`.
pub(crate) fn process_group(pid: u32) -> Option<u32> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = &text[text.rfind(')')? + 1..];
    rest.split_whitespace().nth(2)?.parse().ok()
}

/// Whether a window owner belongs to a launch: the spawned process, a process
/// of its group, or a resident process that may have taken the request.
pub(crate) fn belongs_to(spawned: u32, residents: HashSet<u32>) -> impl Fn(u32) -> bool {
    move |pid| pid == spawned || residents.contains(&pid) || process_group(pid) == Some(spawned)
}

/// Waits for the launched application's window and, when it took the focus,
/// asks the window manager to return it to the window that had it. True only
/// when the display server names that window active afterwards. The wait ends
/// early when `keeps_waiting` says the launch cannot show a window.
pub(crate) fn give_back(
    prior: &Prior,
    belongs: impl Fn(u32) -> bool,
    mut keeps_waiting: impl FnMut() -> bool,
    deadline: Deadline,
) -> bool {
    let wait = deadline.capped(FOCUS_WAIT);
    let mut appeared: Option<Instant> = None;
    loop {
        match judge(prior, &DisplayReader.clients_within(wait), &belongs) {
            Look::Stolen => return restore(prior, &belongs, deadline),
            Look::Appeared => {
                if appeared.get_or_insert_with(Instant::now).elapsed() >= APPEARED_GRACE {
                    return false;
                }
            }
            Look::Nothing => {}
        }
        if wait.is_expired() || !keeps_waiting() {
            return false;
        }
        std::thread::sleep(POLL_INTERVAL.min(wait.remaining()));
    }
}

/// Asks again while the launched window keeps the focus: an application can
/// request its own activation after the first return and win the race.
fn restore(prior: &Prior, belongs: &impl Fn(u32) -> bool, deadline: Deadline) -> bool {
    for _ in 0..RESTORE_ATTEMPTS {
        let sent = deadline.capped(ACTIVATION_GRACE);
        if activate_window(prior.client, &prior.owner, sent, true).is_err() {
            return false;
        }
        let confirm = deadline.capped(CONFIRM_WAIT);
        while !confirm.is_expired() {
            if active_window_within(ACTIVE_READ) == Some(prior.client) {
                return true;
            }
            std::thread::sleep(POLL_INTERVAL.min(confirm.remaining()));
        }
        if judge(prior, &DisplayReader.clients_within(deadline), belongs) != Look::Stolen {
            return false;
        }
    }
    false
}

#[cfg(test)]
#[path = "focus_return_tests.rs"]
mod tests;
