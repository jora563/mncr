# AIOMNI Core

AIOMNI Core это центральный сервис системы AIOMNI. Он написан на Rust.

___
## Сборка

Для сборки требуется чтобы был установлен Rust с необходимыми компонентами. Для тестирования требуется postgresql и правильно настроенные БД.

1. Зайти в корневую директорию проекта.
2. Запустить: `cargo fmt --all` для проверки форматирования и токенизации.
3. Запустить: `cargo clippy --all-targets` для проверки сборки. `clippy` также даёт советы на счёт стилистики и "не очень хорошего кода". `--all-targets` обеспечивает проверку всех бинарных артефактов, включая тестов, библиотек и примеров.
4. Запустить `cargo test` чтобы запустить все тесты.
___
## Запуск

Чтобы запустить AIOMNI Core нужно:

0. Установить Rust, postgresql, сервер ЛЛМ.
1. Поднять postgresql.
2. Создать пользователя и БД через любой клиент.
```sql
CREATE USER aio_core WITH SUPERUSER PASSWORD 'password';
CREATE DATABASE aio_core;
ALTER DATABASE aio_core OWNER TO aio_core;
GRANT ALL ON DATABASE aio_core TO aio_core;
```
3. Зайти пользователям `aio_core` в клиент postgresql и создать БД `ai_omni_test_db_1`.
4. Запустить AIOMNI Core `AI_OMNI_CONFIG_PATH=.test-settings/ai_omni_core_settings.toml cargo run --bin core` из корневой директории проекта. Приложение проработает несколько секунд, и проведёт изначальные миграции БД.
5. Через клиент postgresql залить базовые данные `.test-settings/2026-06-19-OMNIAI-6-core-min.sql` в базу данных. При этом надо проставить правильные креды для бота ТГ и ВК (иначе запросы будут падать).
6. Запустить сервер localai, проверить что правильный ЛЛМ есть в наличии и что в `.test-settings/ai_omni_core_settings.toml` правильно прописаны адрес запроса и наименование модели.
7. Теперь можно полноценно запускать AIOMNI Core из корневой папки через любую из этих команд:
    - `AI_OMNI_CONFIG_PATH=.test-settings/ai_omni_core_settings.toml cargo run --bin ai-omni-core`
    - `AI_OMNI_CONFIG_PATH=.test-settings/ai_omni_core_settings.toml target/debug/ai-omni-core`

___
## Дополнительный материал

- Чтобы провести юнит тесты БД см. [README для БД библиотеки](../libs/db/README.md)
- Чтобы попробовать REPL ЛЛМ на llm-client см. [README для ЛЛМ клиент библиотеки](../libs/llm_client/README.md)


___
## HTTP(S) API [⚠️Будет дорабатываться⚠️]

У AIOMNI Core будет несколько API которые могут вызывать посторонние клиенты.

- [HTTP API Администратора](#api-администратора)
- WS API Оператора
- HTTP API веб-хуков чатов
- HTTP API LLM службы

⚠️ НБ: API routes are provisional!

___
#### GET health

Достать учётную запись бота по его идентификатору.

- Method: GET
- Route: /health
- Headers: To be decided.
- Auth: None
- Response: OK 200 "AIOMNI Core is healthy"

___
### API Администратора

Функционал, в основном, менеджмента проектами и учётными записями ботов. Также будет включать функционал под-грузки данных для LORA адаптеров.


- [DELETE /v1/admin_api/bot/{n}](#delete-bot)
- [DELETE /v1/admin_api/project/{n}](#delete-project)
- [DELETE /v1/admin_api/project_group/{n}](#delete-project-group)
- [GET /v1/admin_api/bot/{n}](#get-bot)
- [GET /v1/admin_api/frontend](#get-frontend)
- [GET /v1/admin_api/platforms](#get-platforms)
- [GET /v1/admin_api/project/{n}/bots](#get-project-bots)
- [GET /v1/admin_api/projects](#get-permitted-projects)
- [GET /v1/admin_api/project_group/{n}/projects](#get-projects)
- [GET /v1/admin_api/project_groups](#get-project_groups)
- [POST /v1/admin_api/bot_account](#post-bot_account)
- [PUT /v1/admin_api/bot_account](#put-bot_account)
- [POST /v1/admin_api/project](#post-project)
- [PUT /v1/admin_api/project](#put-project)
- [POST /v1/admin_api/project_group](#post-project_group)
- [PUT /v1/admin_api/project_group](#put-project_group)

⚠️ Когда будет доработан функционал доступа, SSO и.т.д. будет

**⚠️ Предварительно:** Аутентификация на методы этого АПИ потребует хедер "Authorization: Bearer {token}", где `{token}` это токен авторизации. Внутренний процесс авторизации зависит от системы конкретного заказчика с которой проходят взаимодействия.

___
### DELETE bot

Удалить бот по его идентификатору.

- Method: DELETE
- Route: /v1/admin_api/bot/{bot_id}
- Headers: To be decided.
- Auth: BEARER
- URL suffix: {bot_id} заменить идентификатором бота.
- Response: OK 200

___
### DELETE project

Удалить проект по его идентификатору

- Method: DELETE
- Route: /v1/admin_api/project/{project_id}
- Headers: To be decided.
- Auth: BEARER
- URL suffix: {project_id} заменить идентификатором проекта.
- Response: OK 200

___
### DELETE project group

Удалить проект по его идентификатору

- Method: DELETE
- Route: /v1/admin_api/project_group/{group_id}
- Headers: To be decided.
- Auth: BEARER
- URL suffix: {group_id} заменить идентификатором группы.
- Response: OK 200

___
#### GET bot

Достать учётную запись бота по его идентификатору.

- Method: GET
- Route: /v1/admin_api/bot/{bot_id}
- Headers: To be decided.
- Auth: BEARER
- URL suffix: {bot_id} заменить идентификатором бота.
- Response: OK 200
  ```json
    {
        "id": integer,
        "platform_id": integer,
        "external_id": String,
        "expiry_time_hours": Optional<integer>,
        "token": ByteArray
    }
  ```

___
#### GET frontend

Достать все платформы. Работает как справочник.

- Method: GET
- Route: /v1/admin_api/frontend
- Headers: To be decided.
- Auth: BEARER
- URL suffix: -
- Response: OK 200 + a byte-stream of UTF-8 encoded frontend files.


___
#### GET platforms

Достать все платформы. Работает как справочник.

- Method: GET
- Route: /v1/admin_api/platforms
- Headers: To be decided.
- Auth: BEARER
- URL suffix: -
- Response: OK 200
  ```json
    [{
        "platform": {
            "id": integer,
            "api_id": integer,
            "name": String,
            "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
            "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
        },
        "mirrors": {
            "platform_id": integer,
            "url": String,
            "note": String
        }
    }]
  ```

___
#### GET project bots

Достать все боты по идентификатору их проекта. Боты приходят с полными метаданными.

- Method: GET
- Route: /v1/admin_api/project/{project_id}/bots
- Headers: To be decided.
- Auth: BEARER
- URL suffix: {project_id} заменить идентификатором проекта.
- Response: OK 200
  ```json
    [{
        "account": {
            "id": integer,
            "platform_id": integer,
            "external_id": String,
            "expiry_time_hours": Optional<integer>,
            "token": ByteArray
        },
        "platform": {
            "platform": {
                "id": integer,
                "api_id": String,
                "name": String,
                "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
                "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
            },
            "mirrors": [{
                "platform_id": integer,
                "url": String,
                "note": String
            }]
        },
        "project": {
            "id": integer,
            "project_group_id": integer,
            "external_id": String,
            "project_name": String,
            "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
            "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
        }
    }]
  ```

___
#### GET permitted projects

Достать все проекты которые дозволены пользователю.

- Method: GET
- Route: /v1/admin_api/projects
- Headers: To be decided.
- Auth: BEARER
- URL suffix: -
- Response: OK 200
  ```json
    [{
        "id": integer,
        "project_group_id": integer,
        "external_id": String,
        "project_name": String,
        "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
        "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
    }]
  ```


___
#### GET projects

Достать все проекты по идентификатору их проектной группы.

- Method: GET
- Route: /v1/admin_api/project_group/{group_id}/projects
- Headers: To be decided.
- Auth: BEARER
- URL suffix: {group_id} заменить идентификатором группы.
- Response: OK 200
  ```json
    {
        "group": {
            "id": integer,
            "external_id": String,
            "group_name": String,
            "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
            "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
        },
        "projects": [{
            "id": integer,
            "project_group_id": integer,
            "external_id": String,
            "project_name": String,
            "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
            "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
        }]
    }
  ```

___
#### GET project_groups

Достать все проектные группы.

- Method: GET
- Route: /v1/admin_api/project_groups
- Headers: To be decided.
- Auth: BEARER
- Response: OK 200
  ```json
    [{
        "id": integer,
        "external_id": String,
        "group_name": String,
        "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
        "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
    }]
  ```

___
#### POST bot_account

Добавить новую учётную запись бота.

- Method: POST
- Route: /v1/admin_api/bot
- Headers: To be decided.
- Auth: BEARER
- Request:
  ```json
    {
        "platform_id": integer,
        "external_id": String,
        "token": ByteArray,
        "expiry_h": Optional<integer>
    }
  ```
- Response: OK 201
  ```json
    {
        "id": integer,
        "platform_id": integer,
        "external_id": String,
        "expiry_time_hours": Optional<integer>,
        "token": ByteArray
    }
  ```

___
#### PUT bot_account

Обновить учётную запись бота.

- Method: PUT
- Route: /v1/admin_api/bot
- Headers: To be decided.
- Auth: BEARER
- Request:
  ```json
    {
        "id": integer,
        "platform_id": integer,
        "external_id": String,
        "expiry_time_hours": Optional<integer>,
        "token": ByteArray
    }
  ```
- Response: OK 200

___
#### POST project

Добавить новый проект.

- Method: POST
- Route: /v1/admin_api/project
- Headers: To be decided.
- Auth: BEARER
- Request:
  ```json
    {
        "group_id": integer,
        "external_id": String,
        "name": String
    }
  ```
- Response: ОК 201
  ```json
    {
        "id": integer,
        "project_group_id": integer,
        "external_id": String,
        "project_name": String,
        "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
        "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
    }
  ```

___
#### PUT project

Обновить данные проекта.

- Method: PUT
- Route: /v1/admin_api/project
- Headers: To be decided.
- Auth: BEARER
- Request:
  ```json
    {
        "id": integer,
        "project_group_id": integer,
        "external_id": String,
        "project_name": String,
        "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
        "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
    }
  ```
- Response: OK 200

___
#### POST project_group

Добавить новую группу проектов.

- Method: POST
- Route: /v1/admin_api/project_group
- Headers: To be decided.
- Auth: BEARER
- Request:
  ```json
    {
        "external_id": String,
        "name": String
    }
  ```
- Response: OК 201
  ```json
    {
        "id": integer,
        "external_id": String,
        "group_name": String,
        "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
        "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
    }
  ```

___
#### PUT project_group

Обновить данные

- Method: PUT
- Route: /v1/admin_api/project_group
- Headers: To be decided.
- Auth: BEARER
- Request:
  ```json
    {
        "id": integer,
        "external_id": String,
        "group_name": String,
        "created_on": [Year,Day,Hour,Minute,Second,NanoSecond],
        "altered_on": Option<[Year,Day,Hour,Minute,Second,NanoSecond]>
    }
  ```
- Response: OK 200


___
## WS API [⚠️Будет дорабатываться⚠️]

WS API для оператора вызывается одним методом. После первоначального вызова и перехода на WS протокол, сообщения передаются по созданному каналу.

___
### Методы

#### Начать чат / подсоединится к серверу CORE

Для входа, оператор должен иметь:
- Валидный токен авторизации Keycloak по которому в ASAA есть доступ к тем проектам к которым оператор имеет отношение.
- Headers:
  - Connection: upgrade
  - Upgrade: websocket
  - Sec-WebSocket-Version: 13
  - Sec-Websocket-Key: ???
- Method: GET
- Protocol: ws
- Route: /v1/operator_api/chat
- Auth: BEARER
- Request: - 
- Response: Switching Protocols 101


___
### Сущности

- [MessageSendRequest](#messagesendrequest)
- [MessageHistoryGetRequest](#messagehistorygetrequest)
- [FileGetRequest](#filegetrequest)
- [GetQueuedChatRequest](#getqueuedchatrequest)
- [ConnectionStatusChangeRequest](#connectionstatuschangerequest)
- [ChatStatusChangeRequest](#chatstatuschangerequest)
- [ChatRestoreRequest](#chatrestorerequest)
- [ChatByIdJoinRequest](#chatbyidjoinrequest)
- [IFrameGetRequest](#iframegetrequest)
- [MessageSentEvent](#messagesentevent)
- [MessageHistoryGotEvent](#messagehistorygotevent)
- [IncomingMessageEvent](#incomingmessageevent)
- [QueuedChatGotEvent](#queuedchatgotevent)
- [ConnectionStatusChangedEvent](#connectionstatuschangedevent)
- [ChatStatusChangedEvent](#chatstatuschangedevent)
- [ChatRestoredEvent](#chatrestoredevent)
- [ChatByIdJoinedEvent](#chatbyidjoinedevent)
- [IFrameGotEvent](#iframegotevent)
- [ErrorEvent](#errorevent)


___
#### MessageSendRequest

Послать текст нового сообщения от оператору на сервер (и дальше пользователю).

Сервер не должен посылать это сообщение (Сморите [MessageSentEvent](#messagesentevent) и [MessageHistoryGotEvent](#messagehistorygotevent)).

Интерфейс оператора посылает это сообщение чтобы послать сообщение в тикет на который у него есть разрешение.

```json
{
  // Идентификатор тикета.
  "chatId": integer,
  // Текст сообщение.
  "message": String,
}
```

___
#### MessageHistoryGetRequest

Запрос на историю сообщений для этого тикета от оператора. (Ответ: [MessageHistoryGotEvent](#messagehistorygotevent)).

Интерфейс оператора посылает это сообщение чтобы запросить сообщения из определённой темы.

```json
{
  // Идентификатор
  "chatId": integer,
  // Идентификатор последнего сообщения который уже есть.
  "messageId": Option<integer>,
  // Число сообщений которое надо достать.
  "size": Option<integer>
}
```

___
#### FileGetRequest

Достать файл для сообщения. Посылает оператор. НБ: ИНТЕРФЕЙС ПОКА НЕ РАБОТАЕТ.

Интерфейс оператора посылает это сообщение чтобы запросить файл который закреплен к сообщению. 

```json
{
  // Идентификатор сообщения к которому относится файл.
  "messageId": integer
}
```

___
#### GetQueuedChatRequest

Достать новый тикет из очереди. Посылает только оператор. (Ответ: [QueuedChatGotEvent](#queuedchatgotevent)).

Интерфейс оператора посылает это сообщение чтобы получить доступную тему к обработке. Если нужна конкретная тема предпочитайте [ChatByIdJoinRequest](#chatbyidjoinrequest). Если нужно восстановить последнею тему, пользуйтесь [ChatRestoreRequest](#chatrestorerequest).

```json
{
  // Тэги которые могут быть у тикета. (Пока не задействовано)
  "tags": Array<String>
}
```

___
#### ChatRestoreRequest

Восстановить тикет для оператора если он был. Посылает только оператор. (Ответ: [ChatRestoredEvent](#chatrestoredevent)).

Интерфейс оператора посылает это сообщение чтобы восстановить последнею тему.

```json
{}
```

___
#### ConnectionStatusChangeRequest

Изменить статус соединения (пока не задействовано).

```json
{
  // Статус соединения
  "status": integer
}
```

___
#### ChatStatusChangeRequest

Изменить статус тикета. Посылает оператор. (Статус: [ChatStatusChangedEvent](#chatstatuschangedevent))

Интерфейс оператора посылает это сообщение чтобы изменить статус темы, на пример закрыть её или вернуть ЛЛМ-ке.

Валидные статусы:

- 0: В обработки (ЛЛМ)
- 1: В обработки оператора
- 2: Закрыто благополучно
- 3: Закрыто без разрешения
- 4: Закрыто после обработки оператора

```json
{
  // Идентификатор тикета
  "chatId": integer,
  // Желаемый статус тикета
  "status": integer
}
```

___
#### ChatByIdJoinRequest

Запрос оператора присоединится к чату на который у оператора уже есть право. (Ответ: [ChatByIdJoinedEvent](#chatbyidjoinedevent)).

Интерфейс оператора посылает это сообщение чтобы достать определённую тему из очереди. Обычно это должна быть уже начатая тема с которой оператор уже работал. Если нужно достать новую тему предпочитайте [GetQueuedChatRequest](#getqueuedchatrequest). Если нужно восстановить последнею тему предпочитайте [ChatRestoreRequest](#chatrestorerequest).

```json
{
  // Идентификатор тикета
  "chatId": integer
}
```

___
#### IFrameGetRequest

Запрос от оператора на сервер, достать код ФЕ чата. Запрос должен быть первым, или одним из первых. (Ответ: [IFrameGotEvent](#iframegotevent))

Интерфейс оператора посылает это сообщение чтобы достать код окна оператора. Этот запрос должен быть послан до того как оператор может начать работать, ибо иначе интерфейса нет, и работать не с чем! 

```json
{}
```

___
#### MessageSentEvent

Сообщение от сервера что подтверждает что сообщение послано операторам.

Сервер посылает это сообщение оператору чтобы подтвердить что сообщение послано клиенту.

```json
{
  // Идентификатор сообщения
  "id": integer,
  // Текст сообщения
  "message": String,
  // Имеет ли сообщения прикрепленные файлы
  "hasFile": boolean,
  // Пути файлов которые прикреплены к сообщения (не задействовано) 
  "filePath": Array<String>,
  // Время послания сообщения
  "dateTime": [Year,Day,Hour,Minute,Second,NanoSecond]
}
```

___
#### MessageHistoryGotEvent

Ответ на запрос [MessageHistoryGetRequest](#messagehistorygetrequest).

Сервер посылает это сообщение оператору.

```json
{
  // Идентификатор тикета
  "chatId": integer,
  // Массив сообщений.
  "messages": [{
  // Идентификатор сообщения
    "id": integer,
  // Текст сообщения
    "message": String,
  // Имеет ли сообщения прикрепленные файлы
    "hasFile": boolean,
  // Пути файлов которые прикреплены к сообщения (не задействовано) 
    "filePath": Array<String>,
  // Время послания сообщения
    "dateTime": [Year,Day,Hour,Minute,Second,NanoSecond]
  }]
}
```

___
#### IncomingMessageEvent

Сообщение от сервера что пришло сообщение от клиента.

Сервер посылает это сообщение оператору.

```json
{
  // Идентификатор тикета
  "chatId": integer,
  // Идентификатор сообщения
  "id": integer,
  // Текст сообщения
  "message": String,
  // Имеет ли сообщения прикрепленные файлы
  "hasFile": boolean,
  // Пути файлов которые прикреплены к сообщения (не задействовано) 
  "filePath": Array<String>,
  // Время послания сообщения
  "dateTime": [Year,Day,Hour,Minute,Second,NanoSecond]
}
```

___
#### QueuedChatGotEvent

Ответ от сервера на запрос [GetQueuedChatRequest](#getqueuedchatrequest). Если нет свободных тем, то ChatId = null.

Сервер посылает это сообщение оператору.

```json
{
  // Идентификатор тикета
  "chatId": Option<Integer>
}
```

___
#### ConnectionStatusChangedEvent

Не задействовано.

Сервер посылает это сообщение оператору.

```json
{
  // Статус на который соединение переведено
  "status": integer
}
```

___
#### ChatStatusChangedEvent

Ответ от сервера на запрос [ChatStatusChangeRequest](#chatstatuschangerequest).

Сервер посылает это сообщение оператору.

```json
{
  // Идентификатор тикета
  "chatId": integer,
  // Статус на который переведен тикет
  "status": integer
}
```

___
#### ChatRestoredEvent

Ответ от сервера на запрос [ChatRestoreRequest](#chatrestorerequest).

Сервер посылает это сообщение оператору.

```json
{
  // Идентификатор тикета
  "chatId": Option<integer>
}
```

___
#### ChatByIdJoinedEvent

Ответ от сервера на [ChatByIdJoinedEvent](#chatbyidjoinedevent).

Сервер посылает это сообщение оператору.

```json
{
  // Идентификатор тикета
  "chatId": integer,
  //Статус тикета
  "status": integer
}
```

___
#### IFrameGotEvent

Ответ от сервера на [IFrameGotEvent](#iframegotevent). Содержит код "окошка" оператора.

Сервер посылает это сообщение оператору.

```json
{
  // Код для отправки на ФЕ оператора
  "code": String
}
```

___
#### ErrorEvent

Сообщения от любой стороны об ошибки. Обычно посылается сервером.

И сервер, и интерфейс оператора может посылать это сообщение.

```json
{ // Текст ошибки
  "error_text": String
}
```

