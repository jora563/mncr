//! Defines a client for a Claude instance.
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, IntoUrl, Method, Request, RequestBuilder, Url};

use crate::Result;
use crate::claude::ClaudeRequest;
use crate::llm::{CallLlmService, Llm, LlmRequest};

/// This is a Claude client which is ued to send requests to
/// an instance of Claude.
#[derive(Debug)]
pub struct ClaudeClient {
    inner: Client,
    base_url: Url,
    /// This is an auth string. We can reveal it since it's out in the URL anyway.
    auth: String,
}

impl Llm for ClaudeClient {
    const DEFAULT_SERVICE: &str = "https://api.anthropic.com/v1/";

    fn new() -> Result<Self> {
        Ok(Self {
            inner: Client::new(),
            base_url: Url::parse(Self::DEFAULT_SERVICE)?,
            auth: Default::default(),
        })
    }

    #[tracing::instrument(skip_all)]
    fn set_base_uri<U: IntoUrl>(mut self, path: U) -> Result<Self> {
        self.base_url = path.into_url()?;
        Ok(self)
    }

    fn set_auth<S: AsRef<str>>(mut self, token: S) -> Self {
        self.auth = token.as_ref().to_string();
        self
    }
}

impl CallLlmService for ClaudeRequest {
    type Client = ClaudeClient;
    const DEFAULT_PATH: &str = "messages/";

    /// The standard path is "messages/", but of course there are others possible.
    #[tracing::instrument(skip(self, client))]
    async fn post(
        &self,
        client: &Self::Client,
        path: &str,
    ) -> Result<<Self as LlmRequest>::Response> {
        client.post(self, path).await
    }
}

impl ClaudeClient {
    #[tracing::instrument(skip(self, req))]
    async fn post<Req: LlmRequest>(&self, req: &Req, path: &str) -> Result<Req::Response> {
        tracing::info!("Posting request to LLM");

        let url = self.base_url.join(path)?;
        let request = Request::new(Method::POST, url);

        tracing::trace!("Request: {request:?}");

        let mut map = HeaderMap::new();
        map.insert("X-Api-Key", HeaderValue::from_str(&self.auth)?);
        map.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let response = RequestBuilder::from_parts(self.inner.clone(), request)
            .headers(map)
            .json(req)
            .bearer_auth(&self.auth)
            .json(req)
            .send()
            .await;
        tracing::trace!("reply: {response:?}");

        response
            .inspect_err(|e| println!("Error after send: {e}"))?
            .json()
            .await
            .inspect_err(|e| println!("Error after json: {e}"))
            .map_err(Into::into)
    }
}
