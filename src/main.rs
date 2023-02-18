#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod copier;
mod error;
mod notifier;
mod runner;
mod transfer;

use std::path::Path;

use crossbeam_channel::unbounded;
use trayicon::*;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage, MSG},
};
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use crate::copier::Copier;
use crate::notifier::Notifier;
use crate::runner::Runner;
use crate::{config::Config, transfer::Transfer};

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
    let is_exists = key.get_value::<String, &str>(APP_NAME).is_ok();
    dbg!(is_exists);

    let (s, r) = std::sync::mpsc::channel::<TrayEvents>();
    let icon = include_bytes!("../icon/icon.ico");

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

    let (reload_tx, reload_rx) = unbounded();

    let reload_tx2 = reload_tx.clone();
    std::thread::spawn(move || {
        r.iter().for_each(|m| match m {
            TrayEvents::DoubleClickTrayIcon => {
                println!("Double click");
            }
            TrayEvents::ClickTrayIcon => {
                reload_tx2.send(()).unwrap();
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

    let (notify_tx, notify_rx) = unbounded();
    let (watch_tx, watch_rx) = unbounded();

    std::thread::spawn(|| Notifier::new(notify_tx, reload_rx).run());
    std::thread::spawn(|| Transfer::new(notify_rx, watch_tx, reload_tx).run());
    std::thread::spawn(|| Copier::new(watch_rx).run());

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
