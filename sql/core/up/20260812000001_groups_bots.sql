ALTER TABLE bot_account
    ADD COLUMN project_id BIGINT REFERENCES project(id) ON DELETE RESTRICT;

UPDATE bot_account SET project_id = (
    SELECT project_id FROM bot_account_project WHERE account_id = bot_account.id LIMIT 1
);

DROP TABLE bot_account_project;

ALTER TABLE project_group DROP COLUMN external_id;
ALTER TABLE project ADD COLUMN altered_by VARCHAR(255) NOT NULL DEFAULT 'unknown';