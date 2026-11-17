use crate::domain::models::Permission;
pub fn permissions() -> Vec<Permission> {
    vec![
        Permission {
            capability: "Local inference",
            state: "allowed",
            description: "Only 127.0.0.1:1234. No redirects or proxies.",
        },
        Permission {
            capability: "Conversation storage",
            state: "allowed",
            description: "SQLite in the application's data folder.",
        },
        Permission {
            capability: "Desktop control",
            state: "disabled",
            description: "Future native adapter and per-action approval required.",
        },
        Permission {
            capability: "Files & shell",
            state: "disabled",
            description: "No file tools or command execution registered.",
        },
        Permission {
            capability: "Voice input",
            state: "on-demand",
            description: "Manual dictation or explicitly enabled Hey Jarvis voice mode. Wake detection and transcription run locally; audio stays in memory.",
        },
        Permission {
            capability: "Voice playback",
            state: "on-demand",
            description: "Local Kokoro or the selected system voice. Automatic spoken replies only while Voice mode is enabled.",
        },
        Permission {
            capability: "Speech model setup",
            state: "on-demand",
            description: "Explicit downloads of fixed, checksum-verified speech and listening packs. No audio is uploaded.",
        },
        Permission {
            capability: "Web & MCP",
            state: "disabled",
            description: "No external integrations are active.",
        },
    ]
}
