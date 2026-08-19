-- Это более или менее минимальный скрипт для поднятия БД с минимальными данными.
-- Потом система сама будет добавлять пользователей, их учётные записи, и так далее.
TRUNCATE TABLE
    project_platform,
    project_user,
    user_account_project,
    query_ticket_chat,
    bot,
    attachment,
    "message",
    query_ticket,
    messenger_chat,
    bot_account,
    user_account,
    "user",
    platform_mirror,
    platform,
    project,
    project_group
RESTART IDENTITY CASCADE;

-- Уничтожить псевдо-бд очереди.
DROP TABLE IF EXISTS last_operator, queued_ticket;
