use reqwest::Client as ReqwestClient;
use serde::Serialize;

pub struct Client {
    pub client: ReqwestClient,
}

impl Client {
    pub fn new() -> Self {
        let client = ReqwestClient::builder()
            .timeout(std::time::Duration::from_secs(35))
            .build()
            .expect("Не удалось создать HTTP клиент");
        Self { client }
    }

    pub async fn get(&self, url: &str) -> Result<String, reqwest::Error> {
        self.client.get(url).send().await?.text().await
    }

    pub async fn post_json<T: Serialize + ?Sized>(&self, url: &str, body: &T) -> Result<String, reqwest::Error> {
        self.client.post(url).json(body).send().await?.text().await
    }

    pub async fn post_form<T: Serialize + ?Sized>(&self, url: &str, form: &T) -> Result<String, reqwest::Error> {
        self.client.post(url).form(form).send().await?.text().await
    }
}
