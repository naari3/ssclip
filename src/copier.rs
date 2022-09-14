use std::{borrow::Cow, path::PathBuf};

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
                    Clipboard::new()?.set_image(image_data)?;
                    println!("copied");
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        }
    }
}
