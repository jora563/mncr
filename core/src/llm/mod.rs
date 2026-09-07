//! Модуль для основных функций взаимодействия с ЛЛМ. Пока что макет.
use db::core_schema::DbFullTicket;
use llm::config::AiomniLlmConfig;
use llm::messages::{ChatRequest, ChatResponse};
use llm::methods::AiomniLlmClient;

use crate::config::Config;
use crate::error::Result;

/// Заглушка
#[derive(Debug, Clone)]
pub(super) struct LlmDriver {
    client: AiomniLlmClient,
    config: AiomniLlmConfig,
}

#[derive(Clone, Debug)]
pub(crate) struct LlmRequest(ChatRequest);

/// Заглушка
#[derive(Debug, Clone)]
pub(super) struct LlmReply(pub(super) ChatResponse);

impl LlmReply {
    /// TODO: Extract more than just a string.
    pub(crate) fn extract_answer(self) -> String {
        self.0.reply.to_string()
    }
}

impl LlmDriver {
    /// Create the initial client.
    pub(crate) fn new(cfg: &Config) -> Result<Self> {
        let client = cfg.llm_client().get_client()?;
        let config = cfg.llm_client().to_owned();
        Ok(Self { client, config })
    }

    /// Достать голый клиент для других запросов (кроме чатов)
    pub(crate) fn raw(&self) -> &AiomniLlmClient {
        &self.client
    }

    /// При создания сообщения, мы обязанны склеить все сообщения которые непрочитанные в одно.
    /// Иначе увеличивается вероятность что ЛЛМка будет давать странные ответы.
    pub(crate) fn new_request(
        &self,
        ticket: &DbFullTicket,
        last: String,
        count: usize,
    ) -> LlmRequest {
        let mut request = ChatRequest::with_proj(ticket.ticket.project_id);

        // Последние несколько сообщений это то что на этот раз прислал пользователь, но по отдельности.
        let l = ticket.messages.len() - count;
        for t in ticket.messages.iter().take(l) {
            // NB: It is important to distinguish user and assistant messages.
            match (
                t.message.content.as_ref(),
                t.message.user_account_id.is_some(),
            ) {
                (Some(c), true) => request.add_user_history(c),
                (Some(c), false) => request.add_assistant_history(c),
                (None, _) => {}
            }
        }
        // Это последний набор сообщений, но склеяный.
        request.message = last;
        LlmRequest(request)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn query_llm_with_ticket(
        &self,
        ticket: &DbFullTicket,
        last: String,
        count: usize,
    ) -> Result<LlmReply> {
        let request = self.new_request(ticket, last, count);
        tracing::trace!("LLM Request: {request:#?}");

        let chat = self.config.chat_path();
        let response = self.client.post(request.0, chat).await?;

        Ok(LlmReply(response))
    }
}
