//! Тесты WS методов. Для начала тесты сериализации, так
//! Как есть строгие требования по АПИ.

use super::ws_binary as wsb;
use super::ws_event as wse;
use super::ws_req as wsr;
use crate::error::CoreError;

#[test]
fn test_ws_bin_item_kind() {
    let kind0 = wsb::WsBinItemKind::from_u8(0).unwrap_err();
    assert!(matches!(kind0, CoreError::InvalidWsBinItemKind(0)));

    let kind1 = wsb::WsBinItemKind::from_u8(1).unwrap();
    assert_eq!(kind1, wsb::WsBinItemKind::File);
    assert_eq!(kind1.into_u8(), 1);

    for n in 2..=255 {
        let kind0 = wsb::WsBinItemKind::from_u8(n).unwrap_err();
        assert!(matches!(kind0, CoreError::InvalidWsBinItemKind(_)));
    }
}

#[test]
fn test_roundtrip_ws_bin_item() {
    let bin_item = wsb::WsBinItem::File(wsb::WsAttachment::new(5, 2324, b"I am a teapot".to_vec()));

    let bytes = bin_item.clone().into_bytes(30, None);
    let mut expected = Vec::new();

    expected.push(1);
    expected.extend(30u128.to_le_bytes());
    expected.extend(0u128.to_le_bytes());
    expected.extend(5i64.to_le_bytes());
    expected.extend(2324i64.to_le_bytes());
    expected.extend(b"I am a teapot");

    assert_eq!(&bytes, &expected);

    let output = wsb::WsRawBinItem::from_bytes(bytes).unwrap();

    assert_eq!(output.get_id(), 30);
    assert_eq!(output.get_request_id(), None);

    let output = output.into_item().unwrap();

    assert_eq!(output, bin_item);
}

#[test]
fn test_serialize_ws_requests() {
    let message_send = wsr::WsRequestMsg {
        id: 999,
        inner: wsr::WsRequest::MessageSend(wsr::MessageSendRequest {
            ticket_id: 5555,
            message: "I am all the teapots".to_owned(),
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "MessageSend",
  "data": {
    "chatId": 5555,
    "message": "I am all the teapots"
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wsr::WsRequestMsg {
        id: 999,
        inner: wsr::WsRequest::MessageSend(wsr::MessageSendRequest {
            ticket_id: 5555,
            message: "I am all the teapots".to_owned(),
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "MessageSend",
  "data": {
    "chatId": 5555,
    "message": "I am all the teapots"
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::MessageHistoryGet(wsr::MessageHistoryGetRequest {
            ticket_id: 101,
            last_message_id: Some(32),
            count: Some(4),
        }),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "MessageHistoryGet",
  "data": {
    "chatId": 101,
    "messageId": 32,
    "size": 4
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::FileGet(wsr::FileGetRequest { message_id: 101 }),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "FileGet",
  "data": {
    "messageId": 101
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::GetQueuedChat(wsr::GetQueuedChatRequest {
            tags: vec![
                "banking".to_string(),
                "accounting".to_string(),
                "taxes".to_string(),
            ],
        }),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "GetQueuedChat",
  "data": {
    "tags": [
      "banking",
      "accounting",
      "taxes"
    ]
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::ConnectionStatusChange(wsr::ConnectionStatusChangeRequest {
            conn_status: 1,
        }),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "ConnectionStatusChange",
  "data": {
    "status": 1
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::ChatStatusChange(wsr::ChatStatusChangeRequest {
            ticket_id: 777,
            ticket_status: 1,
        }),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "ChatStatusChange",
  "data": {
    "chatId": 777,
    "status": 1
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::ChatRestore(wsr::ChatRestoreRequest),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "ChatRestore",
  "data": null
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::ChatByIdJoin(wsr::ChatByIdJoinRequest { ticket_id: 12345 }),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "ChatByIdJoin",
  "data": {
    "chatId": 12345
  }
}"#;
    assert_eq!(&res, exp);

    let message_history_get = wsr::WsRequestMsg {
        id: 9898,
        inner: wsr::WsRequest::IFrameGet(wsr::IFrameGetRequest),
    };
    let res = serde_json::to_string_pretty(&message_history_get).unwrap();
    let exp = r#"{
  "id": 9898,
  "type": "IFrameGet",
  "data": null
}"#;
    assert_eq!(&res, exp);
}

#[test]
fn test_serialize_ws_event() {
    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: Some(1),
        inner: wse::WsEvent::QueuedChatGot(wse::QueuedChatGotEvent {
            ticket_id: Some(5555),
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "request_id": 1,
  "type": "QueuedChatGot",
  "data": {
    "chatId": 5555
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::QueuedChatGot(wse::QueuedChatGotEvent {
            ticket_id: Some(5555),
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "QueuedChatGot",
  "data": {
    "chatId": 5555
  }
}"#;
    assert_eq!(&res, exp);

    let error_text = "Help! My cabbage is insane!".to_string();

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::Error(wse::ErrorEvent { error_text }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "Error",
  "data": {
    "error_text": "Help! My cabbage is insane!"
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::IFrameGot(wse::IFrameGotEvent {
            code: r#"<!DOCTYPE html><html lang="en"></html>"#.to_string(),
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "IFrameGot",
  "data": {
    "code": "<!DOCTYPE html><html lang=\"en\"></html>"
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::ChatByIdJoined(wse::ChatByIdJoinedEvent {
            ticket_id: 4,
            ticket_status: 1,
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "ChatByIdJoined",
  "data": {
    "chatId": 4,
    "status": 1
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::ChatRestored(wse::ChatRestoredEvent { ticket_id: Some(4) }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "ChatRestored",
  "data": {
    "chatId": 4
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::ConnectionStatusChanged(wse::ConnectionStatusChangedEvent {
            conn_status: 2,
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "ConnectionStatusChanged",
  "data": {
    "status": 2
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::ChatStatusChanged(wse::ChatStatusChangedEvent {
            ticket_id: 123,
            ticket_status: 1,
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "ChatStatusChanged",
  "data": {
    "chatId": 123,
    "status": 1
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::QueuedChatGot(wse::QueuedChatGotEvent {
            ticket_id: Some(1234),
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "QueuedChatGot",
  "data": {
    "chatId": 1234
  }
}"#;
    assert_eq!(&res, exp);

    let created = db::PrimitiveDateTime::new(
        db::Date::from_ordinal_date(1999, 365).unwrap(),
        db::Time::from_hms_milli(23, 59, 59, 999).unwrap(),
    );
    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::IncomingMessage(wse::IncomingMessageEvent {
            ticket_id: 1234,
            message: wse::Message {
                id: 865,
                message: "My samovar has escaped.".into(),
                has_file: false,
                file_path: vec![],
                created,
            },
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "IncomingMessage",
  "data": {
    "chatId": 1234,
    "id": 865,
    "message": "My samovar has escaped.",
    "hasFile": false,
    "dateTime": [
      1999,
      365,
      23,
      59,
      59,
      999000000
    ]
  }
}"#;
    assert_eq!(&res, exp);

    let message_send = wse::WsEventMsg {
        id: 999,
        request_id: None,
        inner: wse::WsEvent::MessageHistoryGot(wse::MessageHistoryGotEvent {
            ticket_id: 1234,
            messages: vec![
                wse::Message {
                    id: 865,
                    message: "My samovar has escaped.".into(),
                    has_file: false,
                    file_path: vec![],
                    created,
                },
                wse::Message {
                    id: 866,
                    message: "I caught it as it was trying to climb the garden fence.".into(),
                    has_file: false,
                    file_path: vec![],
                    created,
                },
            ],
        }),
    };
    let res = serde_json::to_string_pretty(&message_send).unwrap();
    let exp = r#"{
  "id": 999,
  "type": "MessageHistoryGot",
  "data": {
    "chatId": 1234,
    "messages": [
      {
        "id": 865,
        "message": "My samovar has escaped.",
        "hasFile": false,
        "dateTime": [
          1999,
          365,
          23,
          59,
          59,
          999000000
        ]
      },
      {
        "id": 866,
        "message": "I caught it as it was trying to climb the garden fence.",
        "hasFile": false,
        "dateTime": [
          1999,
          365,
          23,
          59,
          59,
          999000000
        ]
      }
    ]
  }
}"#;
    assert_eq!(&res, exp);
}
