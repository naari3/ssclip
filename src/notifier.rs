use std::{collections::HashMap, path::PathBuf};

use confy::ConfyError;
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
    pub watcher_map: HashMap<PathBuf, RecommendedWatcher>,
    pub tx: Sender<(PathBuf, NotifyKind)>,
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
    pub fn new(tx: Sender<(PathBuf, NotifyKind)>) -> Self {
        Self {
            watcher_map: HashMap::new(),
            tx,
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

    pub fn push_watcher(&mut self, path: &PathBuf, kind: NotifyKind) {
        if self.watcher_map.contains_key(path) {
            return;
        }

        let mut watcher: notify::RecommendedWatcher = Watcher::new(
            send_notify_handler(self.tx.clone(), kind),
            notify::Config::default(),
        )
        .unwrap();

        watcher
            .watch(&path.clone(), notify::RecursiveMode::Recursive)
            .unwrap();

        self.watcher_map.insert(path.clone(), watcher);
    }

    pub fn run(&mut self, reload_rx: Receiver<()>) -> Result<(), ConfyError> {
        loop {
            let config = Config::load()?;
            let paths: Vec<_> = config.path_iter().collect();
            self.remove_diff(&paths);

            for path in paths {
                self.push_watcher(&path, NotifyKind::Copy);
            }
            self.push_watcher(&Config::get_config_path(), NotifyKind::Reload);

            reload_rx.recv().unwrap();
            println!("reload");
        }
    }
}
