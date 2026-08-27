fn main() {
    println!("cargo:rustc-check-cfg=cfg(cargo_clippy, values(any()))");
    println!("cargo:rustc-check-cfg=cfg(cargo_clippy)");
    build_apple_intelligence_bridge();
    #[cfg(feature = "gui")]
    tauri_build::build();
}

fn build_apple_intelligence_bridge() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let manifest_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo provides CARGO_MANIFEST_DIR"),
    );
    let swift_source = manifest_dir.join("native/apple_intelligence.swift");
    let stub_source = manifest_dir.join("native/apple_intelligence_stub.c");
    let output =
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo provides OUT_DIR"))
            .join("apple_intelligence.o");
    let module_cache = output
        .parent()
        .expect("bridge output has a parent")
        .join("module-cache");
    std::fs::create_dir_all(&module_cache).expect("Could not create the Swift module cache");
    println!("cargo:rerun-if-changed={}", swift_source.display());
    println!("cargo:rerun-if-changed={}", stub_source.display());
    let sdk = std::process::Command::new("xcrun")
        .arg("--show-sdk-path")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok());
    let has_foundation_models = sdk.as_deref().is_some_and(|path| {
        std::path::Path::new(path.trim())
            .join("System/Library/Frameworks/FoundationModels.framework")
            .exists()
    });
    let target = std::env::var("TARGET").expect("Cargo provides TARGET");
    let architecture = target
        .split('-')
        .next()
        .map(|value| if value == "aarch64" { "arm64" } else { value })
        .expect("macOS target has an architecture");
    let status = if has_foundation_models {
        // The bridge is weak-linked and guarded at runtime, but compiling it
        // for the framework's first supported release avoids Swift
        // back-deployment libraries becoming requirements of older Macs.
        std::process::Command::new("xcrun")
            .args([
                "swiftc",
                "-parse-as-library",
                "-emit-object",
                "-O",
                "-target",
            ])
            .arg(format!("{architecture}-apple-macosx26.0"))
            .arg(&swift_source)
            .arg("-o")
            .arg(&output)
            .env("CLANG_MODULE_CACHE_PATH", &module_cache)
            .env("SWIFT_MODULECACHE_PATH", &module_cache)
            .status()
            .expect("Xcode with the Swift compiler is required for Apple Intelligence")
    } else {
        std::process::Command::new("xcrun")
            .args(["clang", "-c", "-O2", "-target"])
            .arg(format!("{architecture}-apple-macosx10.13"))
            .arg(&stub_source)
            .arg("-o")
            .arg(&output)
            .status()
            .expect("Xcode command line tools are required for macOS builds")
    };
    assert!(
        status.success(),
        "Could not compile the Apple Intelligence bridge"
    );
    println!("cargo:rustc-link-arg={}", output.display());
    if has_foundation_models {
        println!("cargo:rustc-link-arg=-Wl,-weak_framework,FoundationModels");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
