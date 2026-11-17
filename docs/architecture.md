# Architecture and extension guide

Dependencies point toward the domain. Frameworks and external I/O are replaceable adapters.

```text
React presentation → useAssistant → AssistantGateway port
                                     ↑
                              Tauri IPC adapter
                                     ↓
Tauri commands → AssistantService → domain ports
                                     ↑
                       SQLite / LM Studio / native OS adapters
```

## Frontend

- `src/domain/`: data contracts, the assistant gateway port and model preference rules. No React, Tauri or browser-storage imports.
- `src/features/assistant/useAssistant.ts`: application controller; receives an `AssistantGateway`. Owns connection refresh, selection, generation lifecycle, cancellation and state reconciliation. It does not call Tauri directly.
- `src/features/assistant/`: sidebar, chat workspace, runtime inspector, task/permission pages, generation status and confirmation dialog. Presentation consumes controller state/actions.
- `useConversationSearch` queries the gateway after a short debounce, ignores obsolete responses, and restores history when cleared. Rust validates the query; SQLite searches titles and message content using parameters and caps results at 100. No messages are uploaded or copied into browser storage.
- `MessageContent` is loaded only when an assistant answer needs formatting. It memoizes unchanged messages and defers Markdown parsing during streaming. An explicit element allowlist, skipped raw HTML, inert links and image placeholders keep untrusted output from executing or fetching content. Code blocks and tables scroll within their message.
- `src/components/ui/`: shadcn/ui source components generated from the official registry. The local `cn` helper is in `src/lib/utils.ts`. Small local customizations include scroll viewport props, destructive button contrast, and 44px minimum targets.
- `src/components/feedback/ActivityOrb.tsx`: shared Libraries.dev Thinking Orb adapter. Uses the separately tuned 64px avatar and 20px inline sizes; surrounding text carries the accessible status. Honors system reduced motion. The library suspends animation when hidden/offscreen.
- `src/infrastructure/tauri/`: frontend adapters importing the Tauri SDK. The assistant adapter converts streaming callbacks to a typed IPC channel; the voice adapter implements `VoiceGateway`.
- `src/App.tsx`: composition and shell. Wires the concrete adapter into the controller.
- `src/ui.css`: Tailwind 4 and shadcn theme; `src/style.css`: application-specific layout and theme. All fonts/assets are local. Motion uses `LazyMotion`, with system reduced-motion preferences honored.

The shadcn Sidebar is configured as an always-visible desktop sidebar. Chat follows the [official shadcn AI Elements Chat composition](https://ui.shadcn.com/docs/ai-elements/radix/chat): Message Scroller, Message, Bubble, and Input Group, using the Radix variant to match the rest of the app. The components live in `src/components/ui/`; scrolling is owned by `@shadcn/react/message-scroller`. User turns are scroll anchors, assistant replies stream underneath, and the built-in scroll button returns to the live edge. There is no parallel manual scrolling implementation in the application controller. Pairing optimistic user turns with the persistent assistant ID keeps anchor IDs stable when SQLite history is refreshed. Thinking Orbs and the restricted Markdown renderer remain presentation children of these primitives. The confirmation uses Radix focus management through shadcn AlertDialog.

## Native core

- `src-tauri/src/domain/models.rs`: conversation/message/activity/model types and typed streaming events.
- `src-tauri/src/domain/ports.rs`: `ConversationRepository`, `ModelGateway`, `EventSink`, and `NativePlatform`. No Tauri, SQLite, reqwest or filesystem dependency.
- `src-tauri/src/application/assistant.rs`: input validation, one-active-generation rule, cancellation, context selection, output limits, checkpointing and finalization. Receives repository/gateway implementations; streams to an abstract event sink.
- `src-tauri/src/application/policy.rs`: read-only capability descriptors. Tool capabilities remain disabled.
- `src-tauri/src/infrastructure/sqlite.rs`: schema, parameterized queries, transactions, restart recovery and persistence.
- `src-tauri/src/infrastructure/lm_studio.rs`: fixed local endpoint, HTTP security policy, timeouts, SSE decoding and model discovery. Tests can inject ephemeral loopback fixtures; IPC cannot change the endpoint.
- `src-tauri/src/infrastructure/platform.rs`: native Windows/macOS adapter selection. Automation is unavailable on every adapter.
- `src-tauri/src/commands.rs`: thin Tauri handlers and channel-to-EventSink translation. `DesktopState` carries the application service and runtime metadata.
- `src-tauri/src/lib.rs`: composition root. Creates concrete adapters, opens app data, registers one-instance behavior and wires handlers.

## Voice boundaries

`domain/voice.rs` defines recorder, recognizer, model and speech-output ports. `application/voice.rs` coordinates mutually exclusive recording/transcription/setup, cancellation, audio validation/resampling and bounded Markdown-to-speech text. A scope guard resets the voice stage after completion or errors. The Tauri handlers accept no arbitrary audio path, model URL or speech text: read-aloud looks up a saved, completed assistant message by ID.

`infrastructure/voice/` implements CPAL capture/output, CPU Whisper and Kokoro inference, British-English phonemization, wake detection, speech endpointing, verified model setup, and optional system speech. Manual capture is bounded to 30 seconds with a native watchdog. Cancellation drops the stream; decoding observes cancellation and a two-minute deadline. Samples remain in memory. `assets.rs` stages complete packs from fixed HTTPS URLs and checks every file's size and SHA-256 before publishing. The loopback-only inference client is unchanged.

`useVoice` receives `VoiceGateway` for manual dictation/read-aloud. Successful manual dictation appends to the draft without sending. `useSpeech` receives `SpeechGateway`; previews and saved preferences are separate operations. The shadcn Select/Slider voice picker is lazy-loaded.

`domain/speech.rs` defines synthesis, audio-output and preference-store ports. `application/speech.rs` snapshots preferences, bounds readable text at word boundaries, generates and plays chunks on a worker, and exposes cancellation and asynchronous errors. Preview uses transient validated settings; reply playback uses saved preferences. The Kokoro adapter caches the verified model and feeds phoneme codepoints to native generation. Rust Misaki, a British lexicon, and a spelling fallback handle pronunciation without espeak or a subprocess. The system voice is an explicit selection, never a silent fallback.

`domain/voice_mode.rs` defines capture, wake, endpointing and event ports. `application/voice_mode.rs` runs waiting → listening → transcribing → paused for generation/playback → explicit resume. CPAL sends bounded 512-sample mono frames; overflow aborts rather than silently dropping speech. Silero bounds utterances and capture is dropped before Whisper starts. Session IDs, cancellation and an eight-second UI-heartbeat deadline stop stale listeners. The composition root shares one Whisper instance. Command adapters serialize microphone activation and reject overlapping manual capture.

`useVoiceMode` orchestrates the opt-in conversation through injected gateways. It sends a transcript through the existing assistant workflow, speaks a saved completed reply, waits for playback, then resumes input. Epochs and conversation/page checks discard late events and prevent speech after stop. Stop cancels model generation/playback only if an active voice session owns them. Closing/minimizing stops voice mode. Wake-only background speech and microphone audio never enter SQLite; only submitted requests/replies use normal history.

Windows stages/bundles matching native DLLs beside executables and both Cargo test layouts. macOS links the inference runtime statically. These are in-process libraries. Fake-port native tests cover lifecycle/ownership, while opt-in generated/public-audio tests exercise actual Kokoro, keyword spotting, Silero, and Whisper. Browser tests check the hands-free UI flow without opening a microphone.

The macOS bundle declares microphone purpose and audio-input entitlement. Microphone access remains subject to OS privacy controls. No shell/Node/Python service or model-callable voice tool is added.

## Adding capabilities

Add a domain proposal type describing one concrete action and its exact scope. Implement an application authorization use case backed by a durable audit record and explicit user approval. Only then implement platform adapters. Keep arbitrary commands, model-generated approvals and unrestricted paths out of the IPC surface. Register the smallest new command permission and add boundary tests before exposing a UI control.

To add another local model provider, implement `ModelGateway` and wire it in the composition root; the conversation workflow and UI should stay unchanged. To change persistence, implement `ConversationRepository` and migrate existing data explicitly.

## Verification boundaries

The UI tests mock the gateway adapters. Native tests use real SQLite (including reopening an on-disk database), HTTP/SSE fixtures on ephemeral loopback ports and abstract event sinks. Voice checks include verified model downloads, CPU Whisper recognition, Kokoro synthesis/speed, actual detection of a synthesized wake phrase, Silero endpointing, transcription of a generated public question, and playback through the default output device. Live microphone acoustics and system-voice fallback need hands-on checking. macOS runtime, installer installation and signing are not claimed as verified.
