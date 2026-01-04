use std::collections::HashMap;
use zbus::fdo;
use zbus::zvariant::OwnedValue;

#[zbus::proxy(
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles",
    interface = "org.freedesktop.UPower.PowerProfiles"
)]
pub trait UPowerProfiles {
    #[zbus(property)]
    fn active_profile(&self) -> fdo::Result<String>;

    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> fdo::Result<()>;

    #[zbus(property)]
    fn profiles(&self) -> fdo::Result<Vec<HashMap<String, OwnedValue>>>;
}

impl UPowerProfilesProxy<'_> {
    pub async fn available_profiles(&self) -> fdo::Result<Vec<String>> {
        let profiles = self.profiles().await?;
        let result = profiles
            .iter()
            .filter_map(|p| {
                p.get("Profile")
                    .and_then(|v| v.downcast_ref::<String>().ok())
            })
            .collect();
        Ok(result)
    }
}
