fn main() {
    #[cfg(target_os = "windows")]
    stage_speech_runtime();
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
            "speech_status",
            "speech_save_preferences",
            "speech_preview",
            "speech_download_model",
            "voice_mode_status",
            "voice_mode_download",
            "voice_mode_start",
            "voice_mode_heartbeat",
            "voice_mode_resume",
            "voice_mode_stop",
        ]),
    ))
    .expect("Tauri build failed");
}

#[cfg(target_os = "windows")]
fn stage_speech_runtime() {
    // The direct sherpa-onnx-sys `links` dependency stages the matching runtime first.
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let profile = std::env::var("PROFILE").expect("PROFILE");
    let directory = out
        .ancestors()
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name == profile.as_str())
        })
        .expect("Cargo profile directory");
    // Windows searches the executable directory before System32. Keep the matching ONNX
    // DLL beside both Cargo test layouts, preventing use of Windows' older system ONNX DLL.
    let bundle_runtime = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"),
    )
    .join("native-runtime")
    .join("windows");
    let mut destinations = vec![out.clone(), directory.join("deps"), bundle_runtime];
    // Cargo 1.100 may isolate linked test artifacts in a separate package unit's out/.
    // Those unit directories are allocated before this build script is executed.
    let package_units = directory.join("build").join("kineto-jarvis");
    if let Ok(entries) = std::fs::read_dir(package_units) {
        destinations.extend(
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path().join("out"))
                .filter(|path| path.is_dir()),
        );
    }
    for destination in destinations {
        std::fs::create_dir_all(&destination).expect("create runtime directory");
        for name in [
            "onnxruntime.dll",
            "onnxruntime_providers_shared.dll",
            "sherpa-onnx-c-api.dll",
            "sherpa-onnx-cxx-api.dll",
        ] {
            std::fs::copy(directory.join(name), destination.join(name))
                .expect("stage matching speech runtime");
        }
    }
}
