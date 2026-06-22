-- Either a platform, or an instance of a platform.
CREATE TABLE platform
(
    id BIGSERIAL PRIMARY KEY,
    -- CAN BE IF THAT IS MORE EFFICIENT
    -- It is linked to a hardcoded ENUM for each api.
    -- We do not need a
    api_id SMALLINT NOT NULL,
    -- The name should be the platform.name by default, using the official
    -- messenger API name as a fallback.
    "name" VARCHAR(255),
    created_on TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
    altered_on TIMESTAMP WITHOUT TIME ZONE
);

-- The API host url is recorded in a separate table, since each instance can have multiple
-- addresses (eg proxies).
CREATE TABLE platform_mirror
(
    platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,
    "url" VARCHAR(255) NOT NULL,
    -- A possible note on the purpose of the this mirror link.
    note VARCHAR(1023),
    UNIQUE (platform_id, "url")
);

CREATE INDEX ON platform_mirror(platform_id);
CREATE INDEX ON platform_mirror("url");
-- Represents a group of projects that can share user data. Usually
-- this is an organization or a division of an organization.
-- FOR NOW I do not see a need for further nesting of the project structure.
CREATE TABLE project_group
(
    id BIGSERIAL PRIMARY KEY,
    -- Perhaps VARBINARY is better here. Not sure whether this should be obligatory.
    external_id VARCHAR(255),
    -- Not sure whether this should be obligatory.
    group_name VARCHAR(255),
    created_on TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
    altered_on TIMESTAMP WITHOUT TIME ZONE
);

CREATE TABLE project
(
    id BIGSERIAL PRIMARY KEY,
    project_group_id BIGINT NOT NULL REFERENCES project_group(id) ON DELETE RESTRICT,
    -- Perhaps VARBINARY is better here. Not sure whether this should be obligatory.
    external_id VARCHAR(255),
    -- Not sure whether this should be obligatory.
    project_name VARCHAR(255),
    created_on TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
    altered_on TIMESTAMP WITHOUT TIME ZONE
);

-- This links a project to permitted platforms.
-- We may, or may not wish to add indexes on both fields.
-- For now I have not to avoid premature optimization.
-- The platforms can also be listed in a vector field in the "project" table
-- as this can, in __certain__ cases improve DB performance.
CREATE TABLE project_platform(
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,

    UNIQUE (project_id, platform_id)
);

-- This represents a chat user as defined by the omni-ai platform.
-- It exists to keep track of all of a users accounts, chats and messages.
-- However, not all of a user's data is available to all platforms.
-- TODO: Do we need a separate table of user phones? Especially, if the user
-- represents an organisation, there is a chance of multiple phone numbers.
CREATE TABLE "user"
(
    -- Internal ID of a user.
    id BIGSERIAL PRIMARY KEY,
    -- For now we tie user accounts together by phone.
    -- It is possible that this should be associated with accounts rather than phones.
    phone VARCHAR(63) NOT NULL,
    -- User name or code.
    designation VARCHAR(255) NOT NULL
);

-- This represents basic user data by which a project can identify a user.
-- And is important to partitioning user information.
-- For now we run ONE to MANY (user to project_user).
CREATE TABLE project_user
(
    user_id BIGINT NOT NULL REFERENCES "user"(id) ON DELETE RESTRICT,
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    -- TODO: RESTORE THIS FIELD WHEN IT BECOMES RELEVANT.
    -- external_id VARCHAR(255) NOT NULL,
    created_on TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now(),
    altered_on TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT now()
);

-- Specifically a user account. We might not actually need this table,
-- but depending on the exact manner in which we handle our chats it may be useful.
CREATE TABLE user_account
(
    id BIGSERIAL PRIMARY KEY,
    platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,
    account_status SMALLINT NOT NULL DEFAULT 0,
    user_id BIGINT NOT NULL REFERENCES "user"(id) ON DELETE RESTRICT,
    -- The id that identifies the user in the platform.
    external_id VARCHAR(255) NOT NULL,
    -- Internal designation or alias that the user uses on this account.
    alias VARCHAR(255) NOT NULL,

    UNIQUE (platform_id, external_id)
);

-- We might not strictly need this for users, but we certainly need this
-- for bots.
-- We should probably decide whether bots and users can live in the same
-- table, or whether they truly require different tables.
-- A bot can belong to multiple projects. There may be problems with shared user
-- data in this case.
CREATE TABLE bot_account
(
    id BIGSERIAL PRIMARY KEY,
    platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,
    external_id VARCHAR(255) NOT NULL,
    expiry_time_hours BIGINT,
    token BYTEA NOT NULL,

    UNIQUE (platform_id, external_id)
);

-- This applies to users for now.
CREATE TABLE user_account_project
(
    account_id BIGINT NOT NULL REFERENCES user_account(id) ON DELETE RESTRICT,
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,

    UNIQUE (account_id, project_id)
);

-- This applies to users for now.
CREATE TABLE bot_account_project
(
    account_id BIGINT NOT NULL REFERENCES bot_account(id) ON DELETE RESTRICT,
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,

    UNIQUE (account_id, project_id)
);

-- This refers to a single messenger chat between a customer and a bot in a single
-- messenger
CREATE TABLE messenger_chat
(
    id BIGSERIAL PRIMARY KEY,
    -- ID in the system of the messenger/chat platform.
    external_id VARCHAR(64) NOT NULL,
    -- Do we also need a bot account id, or does each project only have a single bot?
    user_account_id BIGINT NOT NULL REFERENCES user_account(id) ON DELETE RESTRICT,
    bot_account_id BIGINT NOT NULL REFERENCES bot_account(id) ON DELETE RESTRICT,
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    platform_id BIGINT NOT NULL REFERENCES platform(id) ON DELETE RESTRICT,
    started_on TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    -- We do not need a "closed" field, since we can check "closed_on IS NULL".
    closed_on TIMESTAMP WITHOUT TIME ZONE,
    latest_post_on TIMESTAMP WITHOUT TIME ZONE,

    UNIQUE (external_id, platform_id),
    UNIQUE (user_account_id, project_id, platform_id)
);

-- This refers to a single topic of discussion/query ("обращение"), and covers a single
-- appeal/request/query by a customer to the bot. It can span several different messengers
-- but "expires"/"closes" once the handling is complete.
CREATE TABLE query_ticket
(
    id BIGSERIAL PRIMARY KEY,
    -- generated by somehow.
    user_ticket_number INTEGER NOT NULL GENERATED ALWAYS AS IDENTITY,
    user_id BIGINT NOT NULL REFERENCES "user"(id) ON DELETE RESTRICT,
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,
    -- currently proposed schema (for now):
    -- 0=ongoing,
    -- 1=escalation-ongoing, (we might not need this one)
    -- 2=closed-ok,
    -- 3=closed-no-resolution,
    -- 4=closed-after-escalation
    close_status SMALLINT NOT NULL,
    started_on TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    -- We do not need a "closed" field, since we can check "closed_on IS NULL".
    closed_on TIMESTAMP WITHOUT TIME ZONE,
    latest_post_on TIMESTAMP WITHOUT TIME ZONE,
    -- A summary of the topic of the ticket. This is useful if the ticket is
    -- passed to a human operator, or when the ticket is loaded up initially.
    topic VARCHAR(1023),

    UNIQUE (user_ticket_number, user_id)
);

-- A single messenger chat can be dealing with multiple query tickets,
-- A single ticket can be spread over multiple chats.
CREATE TABLE query_ticket_chat
(
    query_ticket_id BIGINT REFERENCES query_ticket(id) ON DELETE RESTRICT,
    messenger_chat_id BIGINT REFERENCES messenger_chat(id) ON DELETE RESTRICT,

    UNIQUE (query_ticket_id, messenger_chat_id)
);

CREATE TABLE message
(
    id BIGSERIAL PRIMARY KEY,
    user_account_id BIGINT REFERENCES user_account(id) ON DELETE RESTRICT,
    bot_account_id BIGINT REFERENCES bot_account(id) ON DELETE RESTRICT,
    -- let's not enum yet at the DB level.
    -- direction ENUM('a', 'b') NOT NULL,
    "type" SMALLINT NOT NULL,
    -- ID of the message in the messenger's own system
    external_id VARCHAR(64) NOT NULL,
    -- In theory messenger_chat_id and query_ticket_id can be placed
    -- into a separate table with (message_id, messenger_chat_id, query_ticket_id)
    -- but this is probably excessive normalization.
    messenger_chat_id BIGINT NOT NULL REFERENCES messenger_chat(id) ON DELETE RESTRICT,
    query_ticket_id BIGINT NOT NULL REFERENCES query_ticket(id) ON DELETE RESTRICT,
    -- Probably should be limited, but with a very big limit.
    content TEXT,
    edited BOOLEAN NOT NULL,
    deleted BOOLEAN NOT NULL,
    created_on TIMESTAMP WITHOUT TIME ZONE DEFAULT now(),
    -- Possibly bot and user accounts should be equivalent.
    UNIQUE (user_account_id, bot_account_id, external_id)
    -- TODO: Think about history for later.
);

CREATE TABLE attachment
(
    id BIGSERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES message(id) ON DELETE RESTRICT,
    -- Let's not assign values to the enum YET.
    -- type ENUM('photo', 'file') NOT NULL,
    "type" SMALLINT NOT NULL,
    external_id VARCHAR(64) NOT NULL,
    file_url VARCHAR(1024),
    file_size BIGINT,
    created_on TIMESTAMP WITHOUT TIME ZONE DEFAULT now()
);

CREATE INDEX ON attachment(message_id);

-- Like user, but for bots. Fundamentally compatible with user,
-- but has different table links.
CREATE TABLE bot
(
    -- Internal ID of a user.
    id BIGSERIAL PRIMARY KEY,
    -- User name or code.
    designation VARCHAR(255) NOT NULL,
    -- A bot may be linked with a bot account, or can be disconnected.
    bot_account_id BIGINT REFERENCES bot_account(id) ON DELETE RESTRICT
);
