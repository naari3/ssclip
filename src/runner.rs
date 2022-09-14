use windows::{
    core::HSTRING,
    Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONSTOP, MB_OK},
};

use crate::{error::Result, APP_NAME};

pub trait Runner {
    fn run(&mut self) {
        match self.run_inner() {
            Ok(_) => {}
            Err(err) => {
                let msg = format!("{}", err);
                message_box(&msg);
                std::process::exit(1);
            }
        };
    }
    fn run_inner(&mut self) -> Result<()>;
}

fn message_box(text: &str) {
    unsafe {
        MessageBoxW(
            None,
            &HSTRING::from(text),
            &HSTRING::from(APP_NAME),
            MB_ICONSTOP | MB_OK,
        );
    }
}
