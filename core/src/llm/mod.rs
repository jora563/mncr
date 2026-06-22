//! Модуль для основных функций взаимодействия с ЛЛМ. Пока что макет.
use db::core_schema::DbFullTicket;
use llm_client::config::{LlmClientCfg, LlmRequestCfg};
use llm_client::llm;
use llm_client::llm::{Llm, LlmMessage, LlmResponse};

use crate::config::Config;
use crate::error::Result;

/// Заглушка
#[derive(Debug, Clone)]
pub(super) struct LlmDriver<T: llm::CallLlmService> {
    client: T::Client,
    client_cfg: LlmClientCfg,
    request_cfg: LlmRequestCfg,
}

#[derive(Clone, Debug)]
pub(crate) struct LlmRequest<T: llm::LlmRequest>(T);

/// Заглушка
#[derive(Debug, Clone)]
pub(super) struct LlmReply<T: llm::LlmRequest>(T::Response);

impl<T: llm::LlmRequest> LlmReply<T> {
    /// TODO: Extract more than just a string.
    pub(crate) fn extract_answer(self) -> String {
        self.0
            .take_messages()
            .pop()
            .map(|x| x.content().to_owned())
            .unwrap_or_default()
    }
}

impl<T: llm::CallLlmService> LlmDriver<T> {
    /// Create the initial client.
    pub(crate) fn new(cfg: &Config) -> Result<Self> {
        let base_path = &cfg.llm_client().host;
        Ok(Self {
            client: T::Client::new()?.set_base_uri(base_path)?,
            client_cfg: cfg.llm_client().to_owned(),
            request_cfg: cfg.llm_req().to_owned(),
        })
    }

    /// При создания сообщения, мы обязанны склеить все сообщения которые непрочитанные в одно.
    /// Иначе увеличивается вероятность что ЛЛМка будет давать странные ответы.
    pub(crate) fn new_request(
        &self,
        ticket: &DbFullTicket,
        last: String,
        count: usize,
    ) -> LlmRequest<T> {
        let mut request = <T as llm::LlmRequest>::new();
        request = self.request_cfg.configure(request);

        // Последние несколько сообщений это то что на этот раз прислал пользователь, но по отдельности.
        let l = ticket.messages.len() - count;
        for t in ticket.messages.iter().take(l) {
            // NB: It is important to distinguish user and assistant messages.
            match (
                t.message.content.as_ref(),
                t.message.user_account_id.is_some(),
            ) {
                (Some(c), true) => request.add_message(T::Message::new_user(c)),
                (Some(c), false) => request.add_message(T::Message::new_assistant(c)),
                (None, _) => {}
            }
        }
        // Это последний набор сообщений, но склеяный.
        request.add_message(T::Message::new_user(last));
        LlmRequest(request)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn query_llm_with_ticket(
        &self,
        ticket: &DbFullTicket,
        last: String,
        count: usize,
    ) -> Result<LlmReply<T>> {
        let request = self.new_request(ticket, last, count);
        tracing::trace!("LLM Request: {request:#?}");

        let path = &self.client_cfg.chat_path;
        let response = request.0.post(&self.client, path).await?;

        Ok(LlmReply(response))
    }
}
