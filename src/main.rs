#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod watcher;

use std::{borrow::Cow, path::Path};

use arboard::{Clipboard, ImageData};
use crossbeam_channel::unbounded;
use trayicon::*;
use watcher::DirectoryWatcher;
use windows::core::{HSTRING, PCWSTR};
use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage, MB_OK, MSG},
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::config::Config;

const APP_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum TrayEvents {
    ClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    OpenSettings,
    ToggleAutorun,
}

fn main() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = Path::new("Software")
        .join("Microsoft")
        .join("Windows")
        .join("CurrentVersion")
        .join("Run");
    let (key, disp) = hkcu.create_subkey(&path).unwrap();
    dbg!(&disp);
    let is_exists = match key.get_value::<String, &str>(APP_NAME) {
        Ok(_) => true,
        Err(_) => false,
    };
    dbg!(is_exists);

    let (s, r) = std::sync::mpsc::channel::<TrayEvents>();
    let icon = include_bytes!("./icon1.ico");

    // Needlessly complicated tray icon with all the whistles and bells
    let mut tray_icon = TrayIconBuilder::new()
        .sender(s)
        .icon_from_buffer(icon)
        .tooltip(APP_NAME)
        .on_click(TrayEvents::ClickTrayIcon)
        .on_double_click(TrayEvents::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("O&pen Settings", TrayEvents::OpenSettings)
                .separator()
                .checkable(
                    "Start ssclip on system startup",
                    is_exists,
                    TrayEvents::ToggleAutorun,
                )
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
                // TODO: Open Settings GUI
                open::that(Config::get_config_path()).unwrap();
            }
            TrayEvents::ToggleAutorun => {
                let state = tray_icon
                    .get_menu_item_checkable(TrayEvents::ToggleAutorun)
                    .unwrap();
                let new_state = !state;
                tray_icon
                    .set_menu_item_checkable(TrayEvents::ToggleAutorun, new_state)
                    .unwrap();
                if new_state {
                    let current_exe = std::env::current_exe()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace("\\\\?\\", "");
                    key.set_value(APP_NAME, &current_exe).unwrap();
                } else {
                    key.delete_value(APP_NAME).unwrap();
                }
            }
        })
    });

    let (tx, rx) = unbounded();

    std::thread::spawn(move || {
        let config = config::Config::load().unwrap();
        let watcher = DirectoryWatcher::new(config);
        match watcher {
            Ok(mut watcher) => loop {
                watcher.run(tx.clone());
                match watcher.reset() {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Error: {}", e);
                        message_box(format!("Error: {e}").as_str());
                        std::process::exit(1);
                    }
                }
            },
            Err(e) => {
                println!("Error: {}", e);
                message_box(format!("Error: {e}").as_str());
                std::process::exit(1);
            }
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

fn to_pcwstr(s: &str) -> PCWSTR {
    PCWSTR::from(&HSTRING::from(s))
}

fn message_box(text: &str) {
    unsafe {
        MessageBoxW(None, to_pcwstr(text), to_pcwstr(APP_NAME), MB_OK);
    }
}
