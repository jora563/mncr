//! Суб-модуль конфигурации.
use crate::error::Result;

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Настройки стыка с Uzorом
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UzorPluginConfig {
    pub(crate) asaa_home: String,
    pub(crate) keycloak: KeycloakSettings,
}

/// Настройки стыка с
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct KeycloakSettings {
    pub(crate) use_server_based_check: bool,
    pub(crate) home: String,
    pub(crate) client_id: String,
    pub(crate) client_secret: Option<String>,
    pub(crate) public_key: String,
    pub(crate) endpoint: String,
    pub(crate) realm: String,
}

impl std::fmt::Debug for KeycloakSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("KeycloakSettings")
            .field("use_server_based_check", &self.use_server_based_check)
            .field("home", &self.home)
            .field("client_id", &self.client_id)
            .field("endpoint", &self.endpoint)
            .field("realm", &self.realm)
            .field("public_key", &"[hidden]")
            .field("client_secret", &"[hidden]")
            .finish()
    }
}

impl UzorPluginConfig {
    /// Создать из файла конфигурации
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::from_str(&file).map_err(Into::into)
    }
    /// Get Keycloak realm
    pub fn realm(&self) -> &str {
        &self.keycloak.realm
    }
    /// Get Keycloak realm
    pub fn client_id(&self) -> &str {
        &self.keycloak.client_id
    }
    /// Get Keycloak realm
    pub fn client_secret(&self) -> Option<&str> {
        self.keycloak.client_secret.as_ref().map(|x| x as &str)
    }
}

#[cfg(feature = "mock_server")]
impl UzorPluginConfig {
    pub fn set_asaa_home(&mut self, new_host: &str) {
        self.asaa_home = new_host.to_string();
    }
    pub fn set_keycloak_home(&mut self, new_host: &str) {
        self.keycloak.home = new_host.to_string();
    }
}
/// Шаблон для того чтобы получить эти настройки. Она скорее всего нужна
/// для того чтобы плугин гибко работал.
pub trait GetUzorPluginConfig {
    fn get_config(&self) -> &UzorPluginConfig;
}

impl GetUzorPluginConfig for UzorPluginConfig {
    fn get_config(&self) -> &UzorPluginConfig {
        self
    }
}
