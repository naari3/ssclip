use std::path::PathBuf;

use crossbeam_channel::{Receiver, Sender};

use crate::{error::Result, notifier::NotifyKind, runner::Runner};

#[derive(Debug)]
pub struct Transfer {
    notify_rx: Receiver<(PathBuf, NotifyKind)>,
    watch_tx: Sender<PathBuf>,
    reload_tx: Sender<()>,
}

impl Transfer {
    pub fn new(
        notify_rx: Receiver<(PathBuf, NotifyKind)>,
        watch_tx: Sender<PathBuf>,
        reload_tx: Sender<()>,
    ) -> Self {
        Self {
            notify_rx,
            watch_tx,
            reload_tx,
        }
    }
}

impl Runner for Transfer {
    fn run_inner(&mut self) -> Result<()> {
        loop {
            let (path, kind) = self.notify_rx.clone().recv()?;

            match kind {
                NotifyKind::Copy => {
                    self.watch_tx.send(path).unwrap();
                }
                NotifyKind::Reload => {
                    self.reload_tx.send(()).unwrap();
                }
            };
        }
    }
}
