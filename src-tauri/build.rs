fn main() {
    println!("cargo:rustc-check-cfg=cfg(cargo_clippy, values(any()))");
    println!("cargo:rustc-check-cfg=cfg(cargo_clippy)");
    tauri_build::build();
}
