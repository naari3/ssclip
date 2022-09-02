use std::path::PathBuf;

use crossbeam_channel::{unbounded, Receiver, Sender};
use notify::Watcher;

use crate::config::Config;

pub enum Event {
    Watch(notify::Event),
    Reload,
}

pub struct DirectoryWatcher {
    config: Config,
    _watchers: Vec<notify::RecommendedWatcher>,
    _config_watcher: notify::RecommendedWatcher,
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl DirectoryWatcher {
    pub fn new(config: Config) -> Self {
        let (tx, rx) = unbounded();
        let mut watchers = Vec::new();

        for path in config.paths.iter() {
            let tx = tx.clone();
            let mut watcher: notify::RecommendedWatcher = Watcher::new(
                move |event: Result<notify::Event, notify::Error>| match event {
                    Ok(event) => {
                        tx.send(Event::Watch(event)).unwrap();
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                },
                notify::Config::default(),
            )
            .unwrap();
            watcher
                .watch(path.as_ref(), notify::RecursiveMode::Recursive)
                .unwrap();
            watchers.push(watcher);
        }

        let tx2 = tx.clone();
        let config_watcher = config.get_watcher(
            move |event: Result<notify::Event, notify::Error>| match event {
                Ok(event) => {
                    match event.kind {
                        notify::EventKind::Modify(_) => {
                            for path in event.paths {
                                // check paths file sizes
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    if metadata.len() > 0 {
                                        tx2.send(Event::Reload).unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(_) => {}
            },
        );

        Self {
            config,
            _config_watcher: config_watcher,
            _watchers: watchers,
            tx,
            rx,
        }
    }

    pub fn reset(&mut self) {
        self.config.reload().unwrap();
        self._watchers.clear();
        for path in self.config.paths.iter() {
            let tx = self.tx.clone();
            let mut watcher: notify::RecommendedWatcher = Watcher::new(
                move |event: Result<notify::Event, notify::Error>| match event {
                    Ok(event) => {
                        tx.send(Event::Watch(event)).unwrap();
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                },
                notify::Config::default(),
            )
            .unwrap();
            watcher
                .watch(path.as_ref(), notify::RecursiveMode::Recursive)
                .unwrap();
            self._watchers.push(watcher);
        }
    }

    pub fn run(&mut self, tx: Sender<PathBuf>) {
        loop {
            match self.rx.recv() {
                Ok(event) => match event {
                    Event::Watch(event) => match event.kind {
                        notify::EventKind::Remove(_) => {}
                        _ => {
                            for path in event.paths {
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    if metadata.len() > 0 {
                                        tx.send(path).unwrap();
                                        break;
                                    }
                                }
                            }
                        }
                    },
                    Event::Reload => {
                        break;
                    }
                },
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
    }
}
