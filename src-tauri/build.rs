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
            "search_conversations",
            "voice_status",
            "voice_download_model",
            "voice_start_recording",
            "voice_finish_recording",
            "voice_cancel",
            "voice_speak_message",
            "voice_stop_speaking",
        ]),
    ))
    .expect("Tauri build failed");
}
