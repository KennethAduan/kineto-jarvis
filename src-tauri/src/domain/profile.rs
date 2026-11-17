use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub display_name: String,
    pub content: String,
    pub enabled: bool,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if self.display_name.chars().count() > 120
            || self.content.chars().count() > 20_000
            || self.display_name.contains('\0')
            || self.content.contains('\0')
        {
            return Err(
                "Use a name up to 120 characters and profile up to 20,000 characters.".into(),
            );
        }
        Ok(())
    }
}
pub trait ProfileRepository: Send {
    fn profile(&self) -> Result<Profile, String>;
    fn save_profile(&self, profile: &Profile) -> Result<(), String>;
}
