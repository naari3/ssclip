use std::path::Path;

#[cfg(windows)]
extern crate windres;

use windres::Build;

fn main() {
    println!("cargo:rerun-if-changed=icon/icon_plain.svg");
    println!("cargo:rerun-if-changed=icon/icon.ico");
    let input = Path::new("icon/icon_plain.svg");
    let output = Path::new("icon/icon.ico");

    svg_to_ico::svg_to_ico(input, 96.0, output, &[32, 64, 256])
        .expect("failed to convert svg to ico");

    Build::new().compile("src/ssclip.rc").unwrap();
}
