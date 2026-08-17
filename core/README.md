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
## HTTP(S)/WS API [⚠️Будет дорабатываться⚠️]

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
