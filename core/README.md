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
    - `AI_OMNI_CONFIG_PATH=.test-settings/ai_omni_core_settings.toml cargo run --bin core`
    - `AI_OMNI_CONFIG_PATH=.test-settings/ai_omni_core_settings.toml target/debug/ai-omni-core`

___
## Дополнительный материал

- Чтобы провести юнит тесты БД см. [README для БД библиотеки](../libs/db/README.md)
- Чтобы попробовать REPL ЛЛМ на llm-client см. [README для ЛЛМ клиент библиотеки](../libs/llm_client/README.md)