//! Lets Tauri generate what the app needs at compile time.

fn main() {
    // The icon is embedded into the executable. Without this line a changed
    // icon is not picked up until something else triggers a rebuild.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build();
}
