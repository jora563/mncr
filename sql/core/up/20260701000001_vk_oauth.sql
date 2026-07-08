-- OAuth данные для standalone приложения VK.
-- Содержит идентификатор приложения, секретный ключ и сервисный токен.
CREATE TABLE vk_oauth
(
 id BIGSERIAL PRIMARY KEY,
 -- Ссылка на платформу (VK инстанцию).
 platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,
 -- Идентификатор standalone приложения VK.
 app_id BIGINT NOT NULL,
 -- Секретный ключ приложения (protected key).
 secure_key BYTEA NOT NULL,
 -- Сервисный токен для API вызовов.
 service_token BYTEA NOT NULL,

 -- На одну платформу приходится один набор OAuth данных standalone приложения.
 UNIQUE (platform_id)
);

-- Состояние OAuth запроса для получения номера телефона.
-- Используется для защиты от CSRF и связывания запроса с пользователем.
CREATE TABLE vk_oauth_state
(
 id BIGSERIAL PRIMARY KEY,
 -- Уникальный state для OAuth.
 state TEXT NOT NULL UNIQUE,
 -- Внешний ID пользователя в платформе, чтобы найти его после авторизации.
 user_ext_id VARCHAR(255) NOT NULL,
 -- Платформа (VK инстанция), через которую идет авторизация.
 platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,
 -- Время создания state.
 created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
