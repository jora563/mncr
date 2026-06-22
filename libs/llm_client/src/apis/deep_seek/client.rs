//! Defines a client for a DeepSeek instance.
use reqwest::{Client, IntoUrl, Method, Request, RequestBuilder, Url};

use crate::Result;
use crate::deep_seek::DeepSeekRequest;
use crate::llm::{CallLlmService, Llm, LlmRequest};

/// This is a DeepSeek client which is ued to send requests to
/// an instance of DeepSeek.
#[derive(Debug)]
pub struct DeepSeekClient {
    inner: Client,
    base_url: Url,
    /// This is an auth string. We can reveal it since it's out in the URL anyway.
    auth: String,
}

impl Llm for DeepSeekClient {
    const DEFAULT_SERVICE: &str = "https://api.deepseek.com/";

    fn new() -> Result<Self> {
        Ok(Self {
            inner: Client::new(),
            base_url: Url::parse(Self::DEFAULT_SERVICE)?,
            auth: Default::default(),
        })
    }

    fn set_base_uri<U: IntoUrl>(mut self, path: U) -> Result<Self> {
        self.base_url = path.into_url()?;
        Ok(self)
    }

    fn set_auth<S: AsRef<str>>(mut self, token: S) -> Self {
        self.auth = token.as_ref().to_string();
        self
    }
}

impl CallLlmService for DeepSeekRequest {
    type Client = DeepSeekClient;
    const DEFAULT_PATH: &str = "chat/completions/";

    /// The standard path is "messages/", but of course there are others possible.
    async fn post(
        &self,
        client: &Self::Client,
        path: &str,
    ) -> Result<<Self as LlmRequest>::Response> {
        client.post(self, path).await
    }
}

impl DeepSeekClient {
    async fn post<Req: LlmRequest>(&self, req: &Req, path: &str) -> Result<Req::Response> {
        let url = self.base_url.join(path)?;
        let request = Request::new(Method::POST, url);
        println!("Request: {request:?}");

        let response = RequestBuilder::from_parts(self.inner.clone(), request)
            .bearer_auth(&self.auth)
            .json(req);
        response
            .send()
            .await
            .inspect_err(|e| println!("Error after send: {e}"))?
            .json()
            .await
            .inspect_err(|e| println!("Error after json: {e}"))
            .map_err(Into::into)
    }
}
