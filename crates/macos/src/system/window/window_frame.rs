//! Maximize on macOS is a frame move and resize, so the frame it replaced is
//! kept in the state root and `restore` puts it back.

use agent_desktop_core::{
    AdapterError, Deadline, OperationReport, ProcessIdentity, Rect, WindowFrameMemory,
    WindowFrameRecord, WindowInfo, WindowOp, WindowReadback,
};

use crate::system::window::{
    window_ops,
    window_postcondition::TOLERANCE,
    window_report, window_resolve,
    window_restore_plan::{WindowRestorePlan, is_maximized_at},
};

pub(crate) fn window_op(
    win: &WindowInfo,
    op: WindowOp,
    deadline: Deadline,
) -> Result<OperationReport, AdapterError> {
    match op {
        WindowOp::Maximize => maximize(win, deadline),
        WindowOp::Restore => restore(win, deadline),
        _ => dispatch(win, &op, window_report::window_readback(&op), deadline),
    }
}

fn dispatch(
    win: &WindowInfo,
    op: &WindowOp,
    readback: Option<WindowReadback>,
    deadline: Deadline,
) -> Result<OperationReport, AdapterError> {
    let dispatched = window_ops::execute(win, op.clone(), deadline);
    OperationReport::from_window_dispatch(win, op, readback, dispatched)
}

fn maximize(win: &WindowInfo, deadline: Deadline) -> Result<OperationReport, AdapterError> {
    let report = dispatch(
        win,
        &WindowOp::Maximize,
        window_report::window_readback(&WindowOp::Maximize),
        deadline,
    )?;
    if let Err(error) = remember_frame(win, deadline) {
        tracing::debug!(%error, "window frame was not recorded");
    }
    Ok(report)
}

fn remember_frame(win: &WindowInfo, deadline: Deadline) -> Result<(), AdapterError> {
    let (Some(before), Some(process)) = (win.bounds, process_of(win)) else {
        return Ok(());
    };
    let memory = WindowFrameMemory::open()?;
    let key = window_key(win);
    if memory
        .recall(&key)?
        .is_some_and(|held| is_maximized_at(&held, before))
    {
        return Ok(());
    }
    let maximized = current_frame(win, deadline)?;
    memory.remember(
        &key,
        WindowFrameRecord {
            process,
            before,
            maximized,
        },
        process_alive,
    )
}

fn restore(win: &WindowInfo, deadline: Deadline) -> Result<OperationReport, AdapterError> {
    let memory = WindowFrameMemory::open().ok();
    let key = window_key(win);
    let record = memory
        .as_ref()
        .and_then(|memory| memory.recall(&key).ok().flatten());
    let plan = WindowRestorePlan::decide(win.state.minimized, record.as_ref(), || {
        current_frame(win, deadline)
    })?;
    match plan {
        WindowRestorePlan::Unminimize => dispatch(
            win,
            &WindowOp::Restore,
            window_report::window_readback(&WindowOp::Restore),
            deadline,
        ),
        WindowRestorePlan::AlreadyNormal => {
            if let (Some(memory), Some(_)) = (&memory, &record) {
                memory.forget(&key)?;
            }
            Ok(OperationReport::window_already_satisfied(
                win,
                &WindowOp::Restore,
            ))
        }
        WindowRestorePlan::SetFrame(frame) => {
            set_frame(win, frame, deadline)?;
            if let Some(memory) = &memory {
                memory.forget(&key)?;
            }
            let size = WindowOp::Resize {
                width: frame.width,
                height: frame.height,
            };
            let readback = WindowReadback::for_window_op(&size, TOLERANCE, "points");
            OperationReport::from_window_dispatch(win, &WindowOp::Restore, Some(readback), Ok(()))
        }
    }
}

fn set_frame(win: &WindowInfo, frame: Rect, deadline: Deadline) -> Result<(), AdapterError> {
    let place = WindowOp::Move {
        x: frame.x,
        y: frame.y,
    };
    let size = WindowOp::Resize {
        width: frame.width,
        height: frame.height,
    };
    window_ops::execute(win, place, deadline)?;
    window_ops::execute(win, size, deadline)
}

fn current_frame(win: &WindowInfo, deadline: Deadline) -> Result<Rect, AdapterError> {
    let element = window_resolve::window_element_for_info(win, deadline)?;
    crate::tree::element_bounds::read_bounds_with_deadline(&element, deadline)?
        .ok_or_else(|| AdapterError::internal("window frame is unreadable"))
}

fn window_key(win: &WindowInfo) -> String {
    format!(
        "{}:{}:{}",
        win.pid,
        win.process_instance.as_deref().unwrap_or_default(),
        win.id
    )
}

fn process_of(win: &WindowInfo) -> Option<ProcessIdentity> {
    win.process_instance
        .as_deref()
        .map(|instance| ProcessIdentity::new(win.pid, instance))
}

fn process_alive(record: &WindowFrameRecord) -> bool {
    crate::system::process::process_identity::to_pid_t(record.process.pid)
        .and_then(|pid| {
            crate::system::process::process_identity::matches_instance(
                pid,
                &record.process.instance,
            )
        })
        .unwrap_or(false)
}
