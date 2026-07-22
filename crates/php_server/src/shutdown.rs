use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use tokio::sync::watch;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum ShutdownPhase {
    Running = 0,
    Draining = 1,
    Forced = 2,
}

#[derive(Clone, Debug)]
pub(crate) struct ShutdownCoordinator {
    phase: Arc<AtomicU8>,
    sender: watch::Sender<ShutdownPhase>,
}

impl ShutdownCoordinator {
    pub(crate) fn new() -> Self {
        let (sender, _) = watch::channel(ShutdownPhase::Running);
        Self {
            phase: Arc::new(AtomicU8::new(ShutdownPhase::Running as u8)),
            sender,
        }
    }

    pub(crate) fn phase(&self) -> ShutdownPhase {
        match self.phase.load(Ordering::Acquire) {
            0 => ShutdownPhase::Running,
            1 => ShutdownPhase::Draining,
            _ => ShutdownPhase::Forced,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        self.phase() == ShutdownPhase::Running
    }

    pub(crate) fn begin_draining(&self) -> bool {
        if self
            .phase
            .compare_exchange(
                ShutdownPhase::Running as u8,
                ShutdownPhase::Draining as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return false;
        }
        self.sender.send_replace(ShutdownPhase::Draining);
        true
    }

    pub(crate) fn force(&self) -> bool {
        let previous = self
            .phase
            .swap(ShutdownPhase::Forced as u8, Ordering::AcqRel);
        if previous == ShutdownPhase::Forced as u8 {
            return false;
        }
        self.sender.send_replace(ShutdownPhase::Forced);
        true
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<ShutdownPhase> {
        self.sender.subscribe()
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}
