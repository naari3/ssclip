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
    pub fn new(config: Config) -> Result<Self, notify::Error> {
        let (tx, rx) = unbounded();

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

        let mut me = Self {
            config: config.clone(),
            _config_watcher: config_watcher,
            _watchers: Vec::new(),
            tx,
            rx,
        };
        me.watcher_from_config(config)?;

        Ok(me)
    }

    pub fn reset(&mut self) -> Result<(), notify::Error> {
        self.config.reload().unwrap();
        self.watcher_from_config(self.config.clone())?;
        Ok(())
    }

    fn watcher_from_config(&mut self, config: Config) -> Result<(), notify::Error> {
        self._watchers.clear();
        for path in config.paths.iter() {
            for entry in glob::glob(path.as_str()).unwrap() {
                if let Ok(path) = entry {
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
                    )?;
                    let result = watcher.watch(&path, notify::RecursiveMode::Recursive);
                    if let Err(e) = result {
                        let e = e.add_path(PathBuf::from(path));
                        return Err(e);
                    }
                    self._watchers.push(watcher);
                }
            }
        }
        Ok(())
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
