#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod watcher;

use std::borrow::Cow;

use arboard::{Clipboard, ImageData};
use crossbeam_channel::unbounded;
use trayicon::*;
use watcher::DirectoryWatcher;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage, MSG},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum TrayEvents {
    ClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    OpenSettings,
    CheckItem1,
}

fn main() {
    let (s, r) = std::sync::mpsc::channel::<TrayEvents>();
    let icon = include_bytes!("./icon1.ico");

    // Needlessly complicated tray icon with all the whistles and bells
    let _tray_icon = TrayIconBuilder::new()
        .sender(s)
        .icon_from_buffer(icon)
        .tooltip("SSClip")
        .on_click(TrayEvents::ClickTrayIcon)
        .on_double_click(TrayEvents::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("R&eload Config", TrayEvents::OpenSettings)
                .separator()
                .checkable("This is checkable", true, TrayEvents::CheckItem1)
                .separator()
                .item("E&xit", TrayEvents::Exit),
        )
        .build()
        .unwrap();

    std::thread::spawn(move || {
        r.iter().for_each(|m| match m {
            TrayEvents::DoubleClickTrayIcon => {
                println!("Double click");
            }
            TrayEvents::ClickTrayIcon => {
                println!("Single click");
            }
            TrayEvents::Exit => {
                std::process::exit(0);
            }
            TrayEvents::OpenSettings => {
                println!("TODO: Open Settings GUI");
            }
            e => {
                println!("{:?}", e);
            }
        })
    });

    let (tx, rx) = unbounded();

    std::thread::spawn(move || {
        let config = config::Config::load().unwrap();
        let mut watcher = DirectoryWatcher::new(config);
        loop {
            watcher.run(tx.clone());
            println!("Reloading config");
            watcher.reset();
        }
    });

    std::thread::spawn(move || loop {
        let path = rx.recv().unwrap();
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
                Clipboard::new().unwrap().set_image(image_data).unwrap();
                println!("copied");
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    });

    // Your applications message loop. Because all applications require an
    // application loop, you are best served using an `winit` crate.
    unsafe {
        let mut message = MSG::default();

        while GetMessageA(&mut message, HWND(0), 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageA(&message);
        }
    }
}
