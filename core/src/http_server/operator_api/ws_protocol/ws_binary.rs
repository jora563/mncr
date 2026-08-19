use crate::CoreCtx;
use crate::error::{CoreError, Result};

use actix_ws::Session;
use bytes::Bytes;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
#[repr(u8)]
pub(crate) enum WsBinItemKind {
    File = 1,
}

impl WsBinItemKind {
    pub(super) fn from_u8(u: u8) -> Result<Self> {
        match u {
            1 => Ok(Self::File),
            _ => Err(CoreError::InvalidWsBinItemKind(u)),
        }
    }

    pub(super) fn into_u8(self) -> u8 {
        unsafe { std::mem::transmute::<Self, u8>(self) }
    }
}

/// Предмет получен как бинарный объект.
/// [0] байт говорит какой тип сообщения.
/// [1..17] байт, идентификатор сообщения.
/// [17..33] байт, идентификатор оригинального запроса. Если ноль, то отсутствует.
/// [33..] данные.
#[derive(Clone, Debug)]
pub(crate) struct WsRawBinItem {
    kind: WsBinItemKind,
    message_id: u128,
    request_id: u128,
    data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum WsBinItem {
    File(WsAttachment),
}

// Предмет получен как бинарный объект.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WsAttachment {
    id: i64,
    message_id: i64,
    data: Vec<u8>,
}

impl WsRawBinItem {
    /// Protocol для сборки WS сообщений.
    #[tracing::instrument(skip_all)]
    pub(crate) fn from_bytes(b: Bytes) -> Result<Self> {
        tracing::debug!("Pre-processing WS binary item");
        if b.len() < 33 {
            return Err(CoreError::WsBytesTooShort(b.len()));
        }
        let kind = WsBinItemKind::from_u8(b[0])?;
        let message_id = u128::from_le_bytes(b[1..17].try_into()?);
        let request_id = u128::from_le_bytes(b[17..33].try_into()?);
        let data = b[33..].to_vec();

        Ok(WsRawBinItem {
            kind,
            message_id,
            request_id,
            data,
        })
    }

    /// Достать идишник сообщения
    pub(crate) fn get_id(&self) -> u128 {
        self.message_id
    }

    /// Достать идишник сообщения на которое это сообщения является ответом,
    /// Если оно является ответом.
    pub(crate) fn get_request_id(&self) -> Option<u128> {
        match self.request_id {
            0 => None,
            x => Some(x),
        }
    }

    /// Из сырого сообщения в варёное.
    #[tracing::instrument(skip_all)]
    pub(crate) fn into_item(self) -> Result<WsBinItem> {
        tracing::debug!("Processing WS binary item");
        let inner = match self.kind {
            WsBinItemKind::File => WsAttachment::from_bytes(self.data)?,
        };
        Ok(WsBinItem::File(inner))
    }

    /// Обратно в `Bytes`
    pub(crate) fn into_bytes(self) -> Bytes {
        let mut output = Vec::with_capacity(33 + self.data.len());
        output.push(self.kind.into_u8());
        output.extend(self.message_id.to_le_bytes());
        output.extend(self.request_id.to_le_bytes());
        output.extend(self.data);
        output.into()
    }
}

impl WsBinItem {
    // Обратно в сыр
    pub(crate) fn into_raw(self, message_id: u128, request_id: Option<u128>) -> WsRawBinItem {
        match self {
            Self::File(f) => WsRawBinItem {
                kind: WsBinItemKind::File,
                message_id,
                request_id: request_id.unwrap_or(0),
                data: f.into_bytes(),
            },
        }
    }

    /// Одним шагом в сырое сообщение.
    pub(crate) fn into_bytes(self, message_id: u128, request_id: Option<u128>) -> Bytes {
        self.into_raw(message_id, request_id).into_bytes()
    }
}

impl WsAttachment {
    /// Создать новый из существующих данных.
    pub(super) fn new(id: i64, message_id: i64, data: Vec<u8>) -> Self {
        Self {
            id,
            message_id,
            data,
        }
    }

    fn from_bytes(mut b: Vec<u8>) -> Result<Self> {
        tracing::debug!("Converting WS binary bytes to file attachment record.");
        if b.len() < 16 {
            return Err(CoreError::WsAttachmentTooShort(b.len()));
        }
        let id = i64::from_le_bytes(b[0..8].try_into()?);
        let message_id = i64::from_le_bytes(b[8..16].try_into()?);

        b.copy_within(16.., 0);
        b.truncate(b.len() - 16);
        let data = b;

        Ok(WsAttachment {
            id,
            message_id,
            data,
        })
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut output = Vec::with_capacity(16 + self.data.len());
        output.extend(self.id.to_le_bytes());
        output.extend(self.message_id.to_le_bytes());
        output.extend(self.data);
        output
    }

    /// Обработать сообщение. Пока ничего не делаем, абы не знаем цель.
    pub(crate) async fn handle(self, _ctx: &Arc<CoreCtx>, _session: &mut Session) -> Result<()> {
        tracing::info!("WS Attachment handled.");
        Ok(())
    }
}
