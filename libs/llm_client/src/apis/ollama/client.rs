//! Defines a client for a Ollama instance.
use reqwest::{Client, IntoUrl, Method, Request, RequestBuilder, Url};

use crate::Result;
use crate::llm::{CallLlmService, Llm, LlmRequest};
use crate::ollama::OllamaRequest;

/// This is a Ollama client which is ued to send requests to
/// an instance of Ollama.
#[derive(Debug)]
pub struct OllamaClient {
    inner: Client,
    base_url: Url,
    /// This is an auth string. We can reveal it since it's out in the URL anyway.
    auth: Option<String>,
}

impl Llm for OllamaClient {
    const DEFAULT_SERVICE: &str = "http://localhost:11434/";

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
        self.auth = Some(token.as_ref().to_string());
        self
    }
}

impl CallLlmService for OllamaRequest {
    type Client = OllamaClient;
    const DEFAULT_PATH: &str = "api/chat";

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

impl OllamaClient {
    #[tracing::instrument(skip(self, req))]
    async fn post<Req: LlmRequest>(&self, req: &Req, path: &str) -> Result<Req::Response> {
        tracing::info!("Posting request to LLM");

        let client = &self;
        let url = client.base_url.join(path)?;
        let request = Request::new(Method::POST, url);

        tracing::trace!("Request: {request:?}");

        let response = RequestBuilder::from_parts(client.inner.clone(), request);
        let response = if let Some(auth) = &client.auth {
            response.bearer_auth(auth)
        } else {
            response
        }
        .json(req)
        .send()
        .await
        .inspect_err(|e| tracing::error!("Error after send: {e}"))?;

        tracing::trace!("reply: {response:?}");

        response
            .json()
            .await
            .inspect_err(|e| tracing::error!("Error after json: {e}"))
            .map_err(Into::into)
    }
}
