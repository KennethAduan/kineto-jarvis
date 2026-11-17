# Kineto / JARVIS

A local-first Tauri 2 desktop assistant. React + Vite + TypeScript, shadcn/ui, Tailwind 4 and Motion handle the interface; Rust owns inference, SQLite and the native permission boundary. No Python or Node sidecar runs with the desktop app. See [the architecture guide](docs/architecture.md) for the domain, application and adapter boundaries.

## Run

Requirements: Node 22.12+ (Node 24 recommended), pnpm 11, stable Rust, and the [Tauri desktop prerequisites](https://v2.tauri.app/start/prerequisites/). Windows needs MSVC C++ Build Tools, the Windows SDK and WebView2. macOS needs Xcode Command Line Tools and a native Mac build.

Voice compilation also needs CMake and LLVM's `libclang` shared library for Whisper's build-time bindings. Put CMake on PATH (or set `CMAKE` to its executable), and set `LIBCLANG_PATH` to the folder containing `libclang.dll` / `libclang.dylib`. CI obtains that build library through the pinned `libclang` package; Python is only a development setup option and is never an app runtime or sidecar.

```sh
pnpm install --frozen-lockfile
pnpm desktop
```

If Rust was just installed, restart the terminal or add the user Cargo `bin` directory to PATH. On this development machine Rust was installed without changing PATH; in PowerShell:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
pnpm desktop
```

In LM Studio, load `qwen/qwen3-4b-2507` and start the local server at `http://127.0.0.1:1234`. This MVP expects local server authentication to be disabled; it never sends an API key. Keep the server bound to loopback. Refresh the connection in the right panel, select a discovered model and send a message. The app prefers the exact discovered `qwen/qwen3-4b-2507` ID, but can use another listed chat model. It does not download or silently load models.

`pnpm dev` provides a browser UI preview with native operations disabled. It does not save chats in browser storage or contact the inference server.

## Voice

Open **Voice settings** beside the composer microphone and choose **Download speech model**. This explicit, one-time download fetches the fixed multilingual Whisper Base model (147,951,465 bytes, about 148 MB) from Hugging Face, verifies SHA-256, and stores it under application data / `voice/ggml-base.bin`. Interrupted or invalid downloads are discarded. Speech-model setup is the only new internet request; it never includes conversation text or audio.

Click the microphone to record, then **Finish** to transcribe locally on the CPU. The transcript is appended to your existing draft for review; it is never sent automatically. Recording closes after 30 seconds, including when the WebView is suspended. Cancel or switching conversations discards the recording/transcript. Audio stays in memory and is not written to SQLite or audio files. Very quiet or shorter-than-half-second recordings are rejected.

For spoken replies, select **Supertonic 3 · Male 2** (the default), download its **145 MB** pack in Voice settings, preview and save. Supertonic runs locally through the existing native sherpa-onnx runtime and produces 44.1 kHz speech. Male 1 and the previous Kokoro British voices remain available. Speaking speed ranges from 0.75× to 1.50×, defaulting to 1.15×. Preview does not change the saved voice. Existing saved voice choices and speeds are preserved. These are synthetic assistant voices, not a reproduction of a film actor. An explicit **System default voice** option uses the installed OS voice at its default speed. Preferences live in `voice/preferences.json` in application data.

Kokoro uses local CPU inference through sherpa-onnx and a Rust British-English pronunciation pipeline. Decorative emoji are skipped when speaking; JARVIS and Ultron have explicit pronunciation overrides, including all-caps spellings. Other unknown ASCII names may still be spelled out by Kokoro, and unsupported language content can require the system voice. Quiet synthesized speech is normalized with bounded gain and peak headroom to avoid digital clipping. Adjust **Output volume** from 0–150% in Voice settings, preview, then save; system voices use OS volume settings. Existing saved preferences retain their voice and speed, with volume defaulting to 100%. The pinned, SHA-256-verified voice pack is 148,782,633 bytes and lives under `voice/kokoro-int8-v1/`. Supertonic has its own text frontend and a separately pinned 145,311,845-byte pack under `voice/supertonic-3-int8-v1/`. Its upstream OpenRAIL-M model license is included alongside the runtime license. Both engines trim excess outer silence while preserving speech-edge margins. No Python or Node service is started. **Read reply aloud** below a completed reply plays it manually; **Stop speaking** interrupts it. Code blocks are omitted and playback is capped at 8,000 characters.

### Hands-free conversation

Download the **7 MB listening pack**, and make sure Whisper, the selected spoken-reply voice, and LM Studio are ready. Click **Voice mode** in the workspace. When the screen says **Say “Hey Jarvis”**, say the wake phrase, then speak your question after **I’m listening** appears. A short pause ends your turn. JARVIS sends the transcript to the local model and starts speaking short phrases as words arrive, while generation continues. Speech starts after roughly 32 characters at a word boundary, then queues phrases of roughly 72 characters or an earlier sentence boundary. It does not wait for a full sentence or completed answer. Individual LLM tokens are not separate utterances; short buffers preserve pronunciation and avoid spelling fragments. The next neural speech buffer is prepared during playback, with a bounded queue to reduce gaps. Local speech synthesis adds a short initial delay; the model's reasoning phase still has to finish before answer text arrives. The microphone remains open during generation and playback to detect user interruptions; it closes during transcription. After 30 seconds without a question it waits for the wake phrase again.

Live speech uses only answer deltas from the native chat pipeline during an active voice session. Reasoning is not spoken; Markdown code blocks and decorative emoji are omitted. The sentence queue is bounded to 256 entries, 8,000 speakable characters and ten minutes of playback; a pending Markdown segment is limited to 32 KiB. Cancelling or failing generation discards queued speech. Speech already heard before a stream failure cannot be retracted. Regular typed chat remains silent until you choose read-aloud.

Voice mode is an explicit opt-in: spoken requests send automatically and are saved in local chat history. Manual dictation continues to require review and a Send action. Voice mode starts off on every launch. Stop with the button, Escape, or say “stop listening”/“goodbye” during a listening turn. Switching conversations/pages, closing or minimizing the app also stops it; loss of the UI heartbeat expires the native session after eight seconds. Speaking during a reply cancels its audio queue and model generation while keeping the session open for the next user turn. Silero speech detection uses a 0.45-second silence endpoint. Sonora (a pure Rust WebRTC audio-processing implementation) reduces speaker echo and noise using a bounded in-memory reference of actual neural speech output. This is not speaker identification: other nearby speech can interrupt too. Echo handling depends on device latency and room acoustics; headphones are recommended for the first test. The OS system voice does not supply the native playback reference.

The **Microphone input level** meter reacts to actual captured audio while waiting, listening, and monitoring for interruption during replies. The AudioCN console displays measured RMS activity history and levels in dBFS; it does not fabricate an output waveform. Speak and check for **Audio detected** before trying the wake phrase. This confirms sound is reaching the app, not that speech was recognized. A flat meter means you should check the default input device, mute switch, and microphone permission in OS sound settings. It resets while the microphone is paused; audio is not recorded just to test the meter.

Wake-word detection uses a small Zipformer keyword model; Silero detects speech and silence. Their pinned 6,682,588-byte pack lives under `voice/hey-jarvis-v1/`. The payload hashes are checked against the original k2-fsa releases. All voice models run locally after setup. Model setup only downloads fixed public assets; no conversation text or audio is uploaded. Recordings are not saved as audio files. Windows/macOS microphone privacy settings must permit the app; macOS bundles declare both dictation and voice-mode microphone use.

## Personal memory

Open **Personal memory** from the sidebar to edit your name and background, enable/disable use in future answers, or forget the active profile. The app always identifies itself as JARVIS. Profile data lives in a separate singleton row of local SQLite and survives new chats and restarts. Public builds contain no preloaded personal information.

For this small profile, local retrieval ranks sections by matching query words, includes the overview and name, and supplies at most four 900-character sections to the model as labelled reference data. There is no embedding service, model training or cloud upload. Missing facts should not be invented, and contact details are not retrieved unless the query matches their section. This is bounded lexical retrieval, not semantic search over files. Treat retrieved profile text as untrusted data, not instructions or tool authorization.

Disabling memory omits it from new model requests. Forgetting removes the active profile row; prior chat messages and forensic SQLite/WAL traces are not erased. The database is not encrypted. Profile changes apply to future turns; changing memory is blocked during generation.

The audio interface uses official [AudioCN](https://www.audiocn.dev/) Level Meter and Live Waveform components, fed from the Rust engine. See `THIRD_PARTY_NOTICES.md` for their MIT license.

## Verify and build

```sh
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
pnpm tauri build --no-bundle
```

For browser layout and Orb rendering regressions, run `pnpm exec playwright install chromium`, then `pnpm test:e2e`. These use an isolated mock adapter on port 1421 and do not read or modify real conversation history.

An opt-in offline recognition test uses the public [Whisper JFK sample](https://github.com/ggml-org/whisper.cpp/blob/master/samples/jfk.wav). Set `WHISPER_TEST_MODEL` to the verified Base model and `WHISPER_TEST_WAV` to that 16 kHz mono WAV, then run `cargo test --manifest-path src-tauri/Cargo.toml recognizes_public_sample_offline -- --ignored`. The separate opt-in `downloads_and_verifies_public_model` test downloads 148 MB into a temporary directory and checks the real downloader. Normal tests never download models, capture your microphone or play audio.

`pnpm tauri build` also creates native installers. macOS packaging/signing needs a Mac and appropriate signing credentials. CI checks Windows and macOS; a workflow being present does not mean those remote checks have run.

The pinned sherpa-onnx dependency downloads its native libraries at build time. Windows builds bundle four native DLLs; keep them beside the executable when using the unbundled build. `build.rs` stages matching DLLs for Cargo's test layouts so Windows cannot substitute its older system ONNX runtime. macOS links the native inference runtime statically. These libraries are not sidecar processes.

Additional opt-in checks: set `KOKORO_TEST_DIR` to a scratch directory and run `cargo test --manifest-path src-tauri/Cargo.toml kokoro_download_and_synthesize_british_voices -- --ignored` to verify real voice synthesis and speed changes. Set `WAKE_TEST_DIR` too and run `detects_synthesized_hey_jarvis_and_speech_endpoint -- --ignored` to verify real wake detection and speech endpointing; providing `WHISPER_TEST_MODEL` also checks transcription of the generated question. `plays_generated_preview_through_default_output -- --ignored` requires `VOICE_PLAYBACK_TEST=1` and plays the public preview WAV. Normal tests never open the microphone or play audio.

## Current behavior

- Discover exact LM Studio model IDs via `GET /v1/models`.
- Stream `POST /v1/chat/completions` through Rust and a typed Tauri channel. Reasoning deltas produce a thinking indicator; they are not saved as the final answer.
- Libraries.dev Thinking Orbs show the current thinking/composing state in the assistant avatar and inline activity row. Animation pauses for reduced motion and when the component is not visible.
- Assistant replies support Markdown headings, lists, tables, task lists and code blocks, including unfinished code blocks during streaming. Raw HTML is skipped; links are displayed as text and remote images as placeholders, so a model cannot cause navigation or remote requests.
- Chat uses the official shadcn AI Elements components: Message Scroller, Message, Bubble and Input Group. User messages align right; assistant replies align left. Streaming follows the live edge while scrolling back preserves your reading position, with a built-in button to return to the latest reply.
- Search conversation titles and saved user/assistant messages locally from the sidebar. Queries are debounced, limited to 200 characters and return the 100 most recently updated matching conversations. Clearing search restores the complete list.
- Stop an active generation; save the partial answer and final task state. Truncated/malformed streams become failed tasks with partial text retained.
- SQLite conversation list, messages, deletion and the latest 100 generation activities. Restart recovery marks unfinished messages/tasks interrupted. Streamed text is checkpointed approximately once per second; a crash may lose the last second of text.
- Native platform interface with Windows and macOS adapters. Neither implements automation yet.
- Live task state, local connection status and capability boundaries. Connection refresh runs every 30 seconds while the window is visible and idle, or manually.

## Architecture and security

```text
React UI → allowlisted Tauri commands → Rust
                                      ├─ reqwest → 127.0.0.1:1234/v1
                                      ├─ SQLite → application data / jarvis.db
                                      ├─ CPAL → bounded in-memory microphone capture → Whisper
                                      ├─ System speech → manual reply playback
                                      └─ NativePlatform → Windows / macOS (tools disabled)
```

The frontend has no shell, filesystem, HTTP or external-link plugins. Explicit app command permissions are restricted to the bundled main window; remote origins receive no capability. A production CSP blocks external scripts, frames and browser network access. Rust uses a fixed loopback URL, ignores proxy environment variables and refuses redirects. Assistant output uses an allowlisted Markdown renderer with raw HTML, navigation and remote media disabled; user messages remain literal text. No model tools are advertised; any tool-call delta fails closed. There is no cloud fallback, analytics or telemetry.

Only a single app instance can run. SQLite writes are transactional; identifiers are generated in Rust and database queries are parameterized. User prompts are limited to 16,000 characters; inference uses the newest 40 completed messages up to 48,000 characters, plus the system prompt. This character bound is not a model-specific token budget: reduce long prompts if your configured context is smaller. Requests have connection/idle timeouts, a 15-minute total generation limit, a 4,096-token generation limit and a 2 MiB response limit.

The permission screen describes current boundaries and cannot enable future desktop, file, shell, web or MCP tools. Voice recording, speech-model setup and playback are separate, user-invoked IPC operations; they are never model tools. Implement scoped action proposals, native authorization, audit records and per-platform adapters before registering agent tools. Model-generated text must never count as user approval.

## Local data and limitations

The SQLite file is `jarvis.db` in Tauri's application data directory for `com.kineto.jarvis` (normally `%APPDATA%/com.kineto.jarvis` on Windows and `~/Library/Application Support/com.kineto.jarvis` on macOS). It is **not encrypted**. OS account permissions and disk encryption protect it. A local process can impersonate LM Studio on the fixed port; this MVP assumes the user's OS account and local inference service are trusted. Deletion removes rows and associated task history, not a forensic secure erase of SQLite/WAL/disk blocks.

No web search, MCP, file access or computer control is implemented. No inference-model download management, authentication token storage, attachments, token-aware context management, secure erase or signed installers are included. Voice uses default OS input/output devices, selected when each capture/playback begins. In-app device selection, speaker identification, GPU transcription and real-time dictation are not included. Wake detection may need acoustic tuning for a particular microphone or room. Windows is the development target; macOS runtime behavior requires verification on a Mac.

API references: [Tauri channels](https://v2.tauri.app/develop/calling-frontend/), [Tauri capabilities](https://v2.tauri.app/security/capabilities/), [LM Studio chat completions](https://lmstudio.ai/docs/developer/openai-compat/chat-completions).
