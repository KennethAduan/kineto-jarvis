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

In LM Studio, load Qwen3.5-9B and start the local server at `http://127.0.0.1:1234`. This MVP expects local server authentication to be disabled; it never sends an API key. Keep the server bound to loopback. Refresh the connection in the right panel, select a discovered model and send a message. The app prefers a discovered Qwen3.5-9B ID, but can use another listed chat model. It does not download or silently load models.

`pnpm dev` provides a browser UI preview with native operations disabled. It does not save chats in browser storage or contact the inference server.

## Voice

Open **Voice settings** beside the composer microphone and choose **Download speech model**. This explicit, one-time download fetches the fixed multilingual Whisper Base model (147,951,465 bytes, about 148 MB) from Hugging Face, verifies SHA-256, and stores it under application data / `voice/ggml-base.bin`. Interrupted or invalid downloads are discarded. Speech-model setup is the only new internet request; it never includes conversation text or audio.

Click the microphone to record, then **Finish** to transcribe locally on the CPU. The transcript is appended to your existing draft for review; it is never sent automatically. Recording closes after 30 seconds, including when the WebView is suspended. Cancel or switching conversations discards the recording/transcript. Audio stays in memory and is not written to SQLite or audio files. Very quiet or shorter-than-half-second recordings are rejected.

Use **Read reply aloud** below a completed assistant reply for manual playback with the installed system voice; **Stop speaking** interrupts it. Code blocks are omitted and playback is capped at 8,000 characters. No wake word, always-on microphone, automatic speech or cloud speech service is enabled. Windows/macOS microphone privacy settings must permit the app; macOS bundles include the microphone usage description and audio-input entitlement.

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

No web search, MCP, file access or computer control is implemented. No inference-model download management, authentication token storage, attachments, token-aware context management, secure erase or signed installers are included. Voice uses the default microphone and installed system voice; device/voice selection, voice activity detection, GPU transcription and real-time dictation are not included. Windows is the development target. Windows and macOS CI checks passed for the initial MVP; macOS runtime behavior still requires verification on a Mac.

API references: [Tauri channels](https://v2.tauri.app/develop/calling-frontend/), [Tauri capabilities](https://v2.tauri.app/security/capabilities/), [LM Studio chat completions](https://lmstudio.ai/docs/developer/openai-compat/chat-completions).
