#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let res = winres::WindowsResource::new();
    res.compile().unwrap();

    tauri_build::build()
}

#[cfg(unix)]
fn main() {
    tauri_build::build()
}
