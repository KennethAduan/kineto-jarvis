fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "bootstrap",
            "discover_models",
            "create_conversation",
            "get_messages",
            "delete_conversation",
            "send_message",
            "cancel_generation",
            "get_activity",
        ]),
    ))
    .expect("Tauri build failed");
}
