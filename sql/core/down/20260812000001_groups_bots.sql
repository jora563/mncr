-- This applies to users for now.
CREATE TABLE bot_account_project
(
    account_id BIGINT NOT NULL REFERENCES bot_account(id) ON DELETE RESTRICT,
    project_id BIGINT NOT NULL REFERENCES project(id) ON DELETE RESTRICT,

    UNIQUE (account_id, project_id)
);

INSERT INTO bot_account_project(account_id, project_id)
    SELECT id, project_id FROM bot_account WHERE project_id IS NOT NULL;

ALTER TABLE bot_account DROP COLUMN project_id;

ALTER TABLE project_group ADD COLUMN external_id VARCHAR(255);
ALTER TABLE project DROP COLUMN altered_by;