//! Embeds Windows executable resources for packaged builds.
//!
//! Version metadata is populated from Cargo package metadata; the icon and a
//! per-monitor DPI-awareness manifest are attached explicitly. Other platforms
//! build the same sources without executable resources.

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
                .set_manifest(MANIFEST)
                .compile()
                .expect("failed to embed Windows resources");
        }
    }
}

#[cfg(windows)]
const MANIFEST: &str = r#"<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </windowsSettings>
  </application>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
</assembly>"#;
