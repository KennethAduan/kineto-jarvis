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
            capability: "Web, voice & MCP",
            state: "disabled",
            description: "No external integrations are active.",
        },
    ]
}
