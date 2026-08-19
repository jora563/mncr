//! Модуль конфигурации сервиса.
//! Интегрируем конфигурации остальных библиотек с доп. настройками самого сервиса (их пока нет).
use crate::error::Result;

use chat::models::Platform;
use db::connect::CoreDbSettings;
use db::core_schema::ApiId;
use llm_client::config::{LlmClientCfg, LlmRequestCfg};
use queue::config::QueueConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;
use uzor_plugin::config::GetUzorPluginConfig;
use uzor_plugin::config::UzorPluginConfig;

/// Уровень логгирования
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TracingLevel {
    #[default]
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl std::fmt::Display for TracingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let output = match self {
            Self::Off => "off",
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        };
        write!(f, "{output}")
    }
}

#[allow(dead_code)]
pub(crate) trait IntoChatApi {
    fn into_chat_api(self) -> Platform;
    fn from_chat_api(a: Platform) -> Self;
}

impl IntoChatApi for ApiId {
    fn into_chat_api(self) -> Platform {
        match self {
            Self::Vk => Platform::VK,
            Self::Telegram => Platform::Telegram,
            Self::Max => Platform::Max,
        }
    }
    fn from_chat_api(a: Platform) -> Self {
        match a {
            Platform::VK => Self::Vk,
            Platform::Telegram => Self::Telegram,
            Platform::Max => Self::Max,
        }
    }
}

impl IntoChatApi for Platform {
    fn into_chat_api(self) -> Platform {
        self
    }
    fn from_chat_api(a: Platform) -> Self {
        a
    }
}

// Настройки чат адаптера
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ChatConfig {
    apis: Vec<Platform>,
}

impl ChatConfig {
    #[allow(dead_code)]
    pub(super) fn contains<I: IntoChatApi>(&self, api: I) -> bool {
        self.apis.contains(&api.into_chat_api())
    }
}

/// Структура конфигурации ЛЛМ.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct LlmConfig {
    request: LlmRequestCfg,
    client: LlmClientCfg,
}

/// Настройки Consul.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ConsulConfig {
    pub consul_host: String,
    pub consul_id: String,
    pub consul_tags: Vec<String>,
    pub current_host: String,
}

/// Общая сущность настроек.
/// ТОДО: Настройки модулей будут подключатся по ходу их исполнения.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Config {
    auth: UzorPluginConfig,
    chat: ChatConfig,
    consul: ConsulConfig,
    core: CoreSettings,
    db: CoreDbSettings,
    llm: LlmConfig,
    queue: QueueConfig,
}

/// Настройки центрального приложения.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CoreSettings {
    pub(crate) threads: u16,
    pub(crate) blocking_threads: u16,
    /// Максимальный уровень логов.
    pub(crate) log_level: TracingLevel,
    /// Записывать ли логи?
    pub(crate) record_logs: bool,
    /// Папка куда складываются логи, если мы их записываем
    /// Если не указана, то логи не записываются.
    pub(crate) log_dir: Option<String>,
    /// Настройка порта сервера
    pub(crate) server_port: u16,
    /// Максимальное число рабочих нитей которые отданы серверу.
    /// см. <https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.workers>
    pub(crate) server_worker_count: u8,
    /// Максимальное число блокирующих нитей которые отданы на каждую
    /// рабочею нить сервера.
    /// см. <https://docs.rs/actix-web/latest/actix_web/struct.HttpServer.html#method.worker_max_blocking_threads>
    pub(crate) server_max_blocking_threads: u8,
    /// Время содержания соединения через WebSocket с операторами, если соединение молчит, до
    /// его отключение.
    pub(crate) operator_idle_timeout_s: u16,
    /// Redirect URI для VK OAuth callback.
    /// Должен быть зарегистрирован в настройках VK приложения.
    pub(crate) vk_redirect_uri: String,
    /// Ссылка на файлы Фронтэнда.
    pub(crate) fe_dir: String,
}

impl Default for CoreSettings {
    fn default() -> Self {
        Self {
            threads: 6,
            blocking_threads: 2,
            log_level: TracingLevel::Info,
            record_logs: false,
            log_dir: None,
            server_port: 8081,
            server_worker_count: 2,
            server_max_blocking_threads: 2,
            operator_idle_timeout_s: 300,
            vk_redirect_uri: "https://localhost:8081/vk/callback".to_string(),
            fe_dir: ".test-settings/test-fe".to_string(),
        }
    }
}

impl GetUzorPluginConfig for Config {
    fn get_config(&self) -> &UzorPluginConfig {
        &self.auth
    }
}

impl Config {
    pub(super) fn from_file<T: AsRef<Path>>(path: T) -> Result<Self> {
        let file = std::fs::read_to_string(path)?;
        toml::from_str(&file).map_err(Into::into)
    }
    #[allow(dead_code)]
    pub(super) fn auth(&self) -> &UzorPluginConfig {
        &self.auth
    }
    pub(super) fn queue(&self) -> &QueueConfig {
        &self.queue
    }
    #[allow(dead_code)]
    pub(super) fn chat(&self) -> &ChatConfig {
        &self.chat
    }
    pub(super) fn consul(&self) -> &ConsulConfig {
        &self.consul
    }
    pub(super) fn core(&self) -> &CoreSettings {
        &self.core
    }
    pub(super) fn db(&self) -> &CoreDbSettings {
        &self.db
    }
    pub(super) fn llm_client(&self) -> &LlmClientCfg {
        &self.llm.client
    }
    pub(super) fn llm_req(&self) -> &LlmRequestCfg {
        &self.llm.request
    }
}

#[cfg(test)]
impl Config {
    pub(super) fn auth_mut(&mut self) -> &mut UzorPluginConfig {
        &mut self.auth
    }
}
