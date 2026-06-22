use crate::error::Result;

use reqwest::Client as ReqwestClient;
use serde::Serialize;

#[derive(Clone, Debug, Default)]
pub struct Client {
    pub client: ReqwestClient,
}

impl Client {
    #[tracing::instrument(skip_all)]
    pub fn new() -> Self {
        let client = ReqwestClient::builder()
            .timeout(std::time::Duration::from_secs(35))
            .build()
            .expect("Не удалось создать HTTP клиент");
        Self { client }
    }

    #[tracing::instrument(skip_all)]
    pub async fn get(&self, url: &str) -> Result<String> {
        self.client
            .get(url)
            .send()
            .await?
            .text()
            .await
            .map_err(Into::into)
    }

    #[tracing::instrument(skip_all)]
    pub async fn post_json<T: Serialize + ?Sized>(&self, url: &str, body: &T) -> Result<String> {
        self.client
            .post(url)
            .json(body)
            .send()
            .await?
            .text()
            .await
            .map_err(Into::into)
    }

    #[tracing::instrument(skip_all)]
    pub async fn post_form<T: Serialize + ?Sized>(&self, url: &str, form: &T) -> Result<String> {
        self.client
            .post(url)
            .form(form)
            .send()
            .await?
            .text()
            .await
            .map_err(Into::into)
    }
}
