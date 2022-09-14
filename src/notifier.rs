use std::{collections::HashMap, path::PathBuf};

use crate::{error::Result, runner::Runner};
use crossbeam_channel::{Receiver, Sender};
use notify::{RecommendedWatcher, Watcher};

use crate::config::Config;

#[derive(Clone, Debug)]
pub enum NotifyKind {
    Copy,
    Reload,
}

#[derive(Debug)]
pub struct Notifier {
    watcher_map: HashMap<PathBuf, RecommendedWatcher>,
    tx: Sender<(PathBuf, NotifyKind)>,
    reload_rx: Receiver<()>,
}

fn send_notify_handler(
    tx: Sender<(PathBuf, NotifyKind)>,
    kind: NotifyKind,
) -> impl FnMut(notify::Result<notify::Event>) + Send + 'static {
    move |res| {
        if let Ok(e) = res {
            let e = match e.kind {
                notify::EventKind::Create(_) => e,
                notify::EventKind::Modify(_) => e,
                _ => return,
            };
            for path in e.paths {
                // check paths file sizes
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.len() > 0 {
                        tx.send((path, kind.clone())).unwrap();
                        break;
                    }
                }
            }
        }
    }
}

impl Notifier {
    pub fn new(tx: Sender<(PathBuf, NotifyKind)>, reload_rx: Receiver<()>) -> Self {
        Self {
            watcher_map: HashMap::new(),
            tx,
            reload_rx,
        }
    }

    pub fn remove_diff(&mut self, paths: &[PathBuf]) {
        let keys = self.watcher_map.keys().cloned().collect::<Vec<_>>();
        for k in keys {
            if !paths.contains(&k) {
                self.watcher_map.remove(&k);
            }
        }
    }

    pub fn push_watcher(&mut self, path: &PathBuf, kind: NotifyKind) -> Result<()> {
        if self.watcher_map.contains_key(path) {
            return Ok(());
        }

        let mut watcher: notify::RecommendedWatcher = Watcher::new(
            send_notify_handler(self.tx.clone(), kind),
            notify::Config::default(),
        )?;

        watcher.watch(&path.clone(), notify::RecursiveMode::Recursive)?;

        self.watcher_map.insert(path.clone(), watcher);

        Ok(())
    }
}

impl Runner for Notifier {
    fn run_inner(&mut self) -> Result<()> {
        loop {
            let config = Config::load()?;
            let paths: Vec<_> = config.path_iter().collect();
            self.remove_diff(&paths);

            for path in paths {
                self.push_watcher(&path, NotifyKind::Copy)?;
            }
            self.push_watcher(&Config::get_config_path(), NotifyKind::Reload)?;

            self.reload_rx.recv()?;
            println!("reload");
        }
    }
}
