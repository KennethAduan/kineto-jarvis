use crate::domain::{models::PlatformInfo, ports::NativePlatform};

// Future native tools must go through this boundary and an explicit authorization gate.
// There is deliberately no execute-command or generic file-access method.
pub struct Windows;
pub struct MacOs;
pub struct Unsupported;
impl NativePlatform for Windows {
    fn name(&self) -> &'static str {
        "Windows"
    }
}
impl NativePlatform for MacOs {
    fn name(&self) -> &'static str {
        "macOS"
    }
}
impl NativePlatform for Unsupported {
    fn name(&self) -> &'static str {
        "Unsupported platform"
    }
}

pub fn info() -> PlatformInfo {
    let platform: Box<dyn NativePlatform> = match std::env::consts::OS {
        "windows" => Box::new(Windows),
        "macos" => Box::new(MacOs),
        _ => Box::new(Unsupported),
    };
    PlatformInfo {
        name: platform.name(),
        automation_available: platform.automation_available(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_native_adapters_deny_automation() {
        for p in [&Windows as &dyn NativePlatform, &MacOs, &Unsupported] {
            assert!(!p.automation_available());
        }
        assert!(
            crate::application::policy::permissions()
                .iter()
                .filter(|p| p.state == "allowed")
                .count()
                == 2
        );
    }
}
