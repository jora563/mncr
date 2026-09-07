//! АПИ для взаимодействий с ЛЛМ сервисом.
use reqwest::{Client, IntoUrl, Method, Request, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::error::{LlmError, Result};
use crate::messages::ErrorResponse;

/// This is a Claude client which is ued to send requests to
/// an instance of Claude.
#[derive(Clone, Debug)]
pub struct AiomniLlmClient {
    inner: Client,
    base_url: Url,
    /// This is an auth string. We can reveal it since it's out in the URL anyway.
    auth: String,
}

impl AiomniLlmClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Client::new(),
            base_url: Url::parse("localhost:8000").map_err(LlmError::url)?,
            auth: Default::default(),
        })
    }

    #[tracing::instrument(skip_all)]
    pub fn set_base_uri<U: IntoUrl>(mut self, path: U) -> Result<Self> {
        self.base_url = path.into_url()?;
        Ok(self)
    }

    pub fn set_auth<S: AsRef<str>>(mut self, token: S) -> Self {
        self.auth = token.as_ref().to_string();
        self
    }

    /// Послать сообщение с методом DELETE
    #[tracing::instrument(skip_all)]
    pub async fn delete<S, D>(&self, req: S, path: &str) -> Result<D>
    where
        S: Serialize,
        D: for<'d> Deserialize<'d>,
    {
        self.send(req, path, Method::DELETE).await
    }

    /// Послать сообщение с методом GET
    #[tracing::instrument(skip_all)]
    pub async fn get<S, D>(&self, req: S, path: &str) -> Result<D>
    where
        S: Serialize,
        D: for<'d> Deserialize<'d>,
    {
        self.send(req, path, Method::GET).await
    }

    /// Постлать сообщение с методом POST
    #[tracing::instrument(skip_all)]
    pub async fn post<S, D>(&self, req: S, path: &str) -> Result<D>
    where
        S: Serialize,
        D: for<'d> Deserialize<'d>,
    {
        self.send(req, path, Method::POST).await
    }

    /// Послать сообщение с методом PUT
    #[tracing::instrument(skip_all)]
    pub async fn put<S, D>(&self, req: S, path: &str) -> Result<D>
    where
        S: Serialize,
        D: for<'d> Deserialize<'d>,
    {
        self.send(req, path, Method::PUT).await
    }

    /// Метод посылает запрос на ЛЛМ и пытается дождаться ответа. Если приходит ошибка
    /// то она её преобразовывает и отдаёт.
    #[tracing::instrument(skip_all)]
    pub async fn send<S, D>(&self, req: S, path: &str, method: Method) -> Result<D>
    where
        S: Serialize,
        D: for<'d> Deserialize<'d>,
    {
        tracing::info!("Sending ({method}) request to LLM");

        let url = self.base_url.join(path).map_err(LlmError::url)?;
        let request = Request::new(method, url);

        tracing::trace!("Request: {request:?}");

        let response = RequestBuilder::from_parts(self.inner.clone(), request)
            .json(&req)
            .bearer_auth(&self.auth)
            .send()
            .await
            .inspect_err(|e| tracing::error!("Error after send: {e}"))?;

        Self::process_response(response).await
    }

    /// Метод посылает запрос на ЛЛМ и пытается дождаться ответа. Если приходит ошибка
    /// то она её преобразовывает и отдаёт.
    #[tracing::instrument(skip_all)]
    pub async fn send_payload<D>(
        &self,
        form: reqwest::multipart::Form,
        path: &str,
        method: Method,
    ) -> Result<D>
    where
        D: for<'d> Deserialize<'d>,
    {
        tracing::info!("Sending ({method}) binary request to LLM");

        let url = self.base_url.join(path).map_err(LlmError::url)?;
        let request = Request::new(method, url);

        tracing::trace!("Request: {request:?}");

        let r = RequestBuilder::from_parts(self.inner.clone(), request)
            .multipart(form)
            .bearer_auth(&self.auth);

        tracing::info!("Request to send: {r:?}");
        let response = r
            .send()
            .await
            .inspect_err(|e| tracing::error!("Error after send: {e}"))?;

        Self::process_response(response).await
    }

    async fn process_response<D: for<'d> Deserialize<'d>>(res: reqwest::Response) -> Result<D> {
        tracing::info!("Processing response from LLM");
        tracing::trace!("response raw: {res:?}");

        let text = res.text().await?;
        tracing::trace!("response text: {text:?}");
        println!("response text: {text:?}");

        // Try to get normal response.
        let res: Result<D> = serde_json::from_str(&text).map_err(Into::into);

        // If this fails, try to get error details.
        let error_response: ErrorResponse = match res {
            Ok(d) => return Ok(d),
            Err(e) => {
                tracing::warn!("Cannot deserialize response from AIOMNI LLM: {e}");
                println!("Cannot deserialize response from AIOMNI LLM: {e}");
                serde_json::from_str(&text)?
            }
        };
        Err(LlmError::from_error_response(error_response))
    }
}
