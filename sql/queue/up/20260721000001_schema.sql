CREATE TABLE queued_ticket(
    ticket_id BIGINT NOT NULL PRIMARY KEY,
    project_name TEXT NOT NULL,
    last_operator TEXT,
    added_to_queue TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    vip_level BIGINT NOT NULL,
    ticket_status SMALLINT NOT NULL
);

CREATE TABLE last_operator(
    id BIGSERIAL PRIMARY KEY,
    ext_id TEXT NOT NULL,
    last_ticket_id BIGINT NOT NULL REFERENCES queued_ticket(ticket_id),
    work_started TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    last_check_in TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    in_work BOOLEAN NOT NULL
);
