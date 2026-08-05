use crate::config::ConsulConfig;
use reqwest::Client;
use serde_json::json;

#[derive(Debug)]
pub struct ConsulClient {
    // Клиент для HTTP запросов
    client: Client,
    // Данные конфигов для Consul
    config: ConsulConfig,
    // Хост Consul
    base_url: String,
    // Пользователь Consul
    auth_user: String,
    // Пароль Consul
    auth_pass: String,
}

impl ConsulClient {
    pub fn new(config: ConsulConfig) -> Self {
        let client = Client::new();

        let parsed_url = match reqwest::Url::parse(&config.consul_host) {
            Ok(u) => u,
            Err(e) => {
                // Если не удалось распарсить конфиги, то возвращаем пустые значения,
                // тогда регистрация автоматически отвалится и выдаст соответствующую ошибку.
                tracing::error!("Failed to parse Consul URL: {}", e);
                return Self {
                    client,
                    config,
                    base_url: String::new(),
                    auth_user: String::new(),
                    auth_pass: String::new(),
                };
            }
        };

        let scheme = parsed_url.scheme();
        let host = parsed_url.host_str().unwrap_or("");
        let port = parsed_url.port().unwrap_or(8500);

        let base_url = format!("{}://{}:{}", scheme, host, port);

        let auth_user = parsed_url.username().to_string();
        let auth_pass = parsed_url.password().unwrap_or("").to_string();

        Self {
            client,
            config,
            base_url,
            auth_user,
            auth_pass,
        }
    }

    pub async fn register(&self) {
        if self.base_url.is_empty() {
            tracing::error!("Cannot register: invalid Consul URL");
            return;
        }

        let check = json!({
            "CheckID": self.config.consul_id,
            "Name": "HTTP Health Check",
            "HTTP": format!("{}/health", self.config.current_host),
            "Method": "GET",
            "Interval": "60s",
            "Timeout": "10s",
            "deregister_critical_service_after": "1m"
        });

        let payload = json!({
            "ID": self.config.consul_id,
            "Name": "uzor-aiomni-core",
            "Tags": self.config.consul_tags,
            "Address": self.config.current_host,
            "Check": check
        });

        let url = format!("{}/v1/agent/service/register", self.base_url);

        let mut req = self.client.put(&url).json(&payload);

        if !self.auth_user.is_empty() {
            req = req.basic_auth(&self.auth_user, Some(&self.auth_pass));
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("Consul successfully registered");
            }
            Ok(resp) => {
                tracing::error!("Couldn't register Consul. Status: {}", resp.status());
            }
            Err(e) => {
                tracing::error!("Request failed: {}", e);
            }
        }
    }
}
