use std::{borrow::Cow, path::PathBuf, thread::sleep, time::Duration};

use arboard::{Clipboard, ImageData};
use crossbeam_channel::Receiver;

use crate::{error::Result, runner::Runner};

#[derive(Debug)]
pub struct Copier {
    watch_rx: Receiver<PathBuf>,
}

impl Copier {
    pub fn new(watch_rx: Receiver<PathBuf>) -> Self {
        Self { watch_rx }
    }
}

impl Runner for Copier {
    fn run_inner(&mut self) -> Result<()> {
        loop {
            let path = self.watch_rx.recv()?;
            match image::open(path) {
                Ok(image) => {
                    let width = image.width() as usize;
                    let height = image.height() as usize;
                    let bytes = image.into_rgba8().into_vec();
                    let image_data = ImageData {
                        width,
                        height,
                        bytes: Cow::from(&bytes[..]),
                    };
                    let task = || Clipboard::new()?.set_image(image_data.clone());
                    loop {
                        let result = task();
                        match result {
                            Ok(_) => {
                                break;
                            }
                            Err(e) => match e {
                                arboard::Error::ClipboardOccupied => {
                                    println!("Clipboard occupied, retrying...");
                                    sleep(Duration::from_millis(100));
                                    continue;
                                }
                                // others, just return error
                                _ => return Err(e.into()),
                            },
                        }
                    }
                    println!("copied");
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
    }
}
