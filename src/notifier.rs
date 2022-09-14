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

    pub fn run(&mut self, reload_rx: Receiver<()>) -> Result<(), ConfyError> {
        let notify_tx2_1 = self.tx.clone();
        let notify_tx3 = self.tx.clone();
        loop {
            let config = Config::load()?;
            let paths: Vec<_> = config.path_iter().collect();
            let keys = self.watcher_map.keys().cloned().collect::<Vec<_>>();
            for k in keys {
                if !paths.contains(&k) {
                    self.watcher_map.remove(&k);
                }
            }
            let notify_tx2_2 = notify_tx2_1.clone();
            for path in paths {
                let mut watcher: notify::RecommendedWatcher = Watcher::new(
                    send_notify_handler(notify_tx2_2.clone(), NotifyKind::Copy),
                    notify::Config::default(),
                )
                .unwrap();

                watcher
                    .watch(&path.clone(), notify::RecursiveMode::Recursive)
                    .unwrap();

                self.watcher_map.insert(path.clone(), watcher);
            }

            let config_path = Config::get_config_path();
            let mut watcher: notify::RecommendedWatcher = Watcher::new(
                send_notify_handler(notify_tx3.clone(), NotifyKind::Reload),
                notify::Config::default(),
            )
            .unwrap();
            watcher
                .watch(&config_path.clone(), notify::RecursiveMode::Recursive)
                .unwrap();
            self.watcher_map.insert(config_path.clone(), watcher);

            reload_rx.recv().unwrap();
            println!("reload");
        }
    }
}
