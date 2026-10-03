//! Embeds Windows executable resources for packaged builds.
//!
//! Version metadata is populated from Cargo package metadata and the icon is
//! attached explicitly. GPUI already embeds an application manifest, so this
//! build script must not add a second one (the resource compiler rejects a
//! duplicate MANIFEST resource). Other platforms build the same sources without
//! executable resources.

fn main() {
    println!("cargo:rerun-if-changed=../../packaging/icons/app-icon.ico");

    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
            let icon = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packaging/icons/app-icon.ico");
            winresource::WindowsResource::new()
                .set_icon(&icon.to_string_lossy())
                .set("ProductName", "PacketSmith")
                .set("FileDescription", "PacketSmith API development platform")
                .set("LegalCopyright", "Copyright (c) PacketSmith Contributors")
                .set("OriginalFilename", "PacketSmith.exe")
                .compile()
                .expect("failed to embed Windows resources");
        }
    }
}
