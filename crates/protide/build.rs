use std::path::PathBuf;

fn main() {
    // Generate .desktop file for Linux dock integration
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let icon_src = PathBuf::from("assets/protide-logo.png");
    let icon_dst = out_dir.join("protide-logo.png");

    if icon_src.exists() {
        std::fs::copy(&icon_src, &icon_dst).ok();
    }

    // Write desktop file metadata so the app can be pinned in Ubuntu dock
    let desktop_content = r#"[Desktop Entry]
Name=Protide
GenericName=API Testing Tool
Comment=Native API client for HTTP, GraphQL, WebSocket, and gRPC
Exec=protide
Icon=protide
Terminal=false
Type=Application
Categories=Development;WebDevelopment;Network;
Keywords=api;rest;graphql;websocket;grpc;http;testing;
StartupNotify=true
StartupWMClass=Protide
"#;
    std::fs::write(out_dir.join("protide.desktop"), desktop_content).ok();

    // Instruct cargo to rerun this script if the icon changes
    println!("cargo::rerun-if-changed=assets/protide-logo.png");
}
