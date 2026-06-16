INSERT INTO project_group(external_id, group_name) VALUES('AFHG-9999', 'AFHG group');

INSERT INTO project(project_group_id, external_id, project_name) VALUES(1, 'AFHG-9999-0001', 'Project I');

INSERT INTO platform(api_id, name) VALUES(33, 'AFHG-gram');

INSERT INTO project_platform(project_id, platform_id) VALUES(1, 1);

INSERT INTO bot_account(platform_id, user_id, external_id, token) VALUES(1, 0 , '1234-1234-1234', 0x01231231);

INSERT INTO user(phone, designation) VALUES('079991232323', 'Гоф, Александр Фантазёрович');

INSERT INTO user_account(platform_id, user_id, external_id, alias) VALUES(1,1,'4567-4567-4567', 'Фантазёр');

INSERT INTO user_account_project(account_id, project_id) VALUES(1, 1);
INSERT INTO bot_account_project(account_id, project_id) VALUES(1, 1);
INSERT INTO project_user(user_id, project_id, external_id) VALUES(1,1,'AFGH-c2-0001');

INSERT INTO messenger_chat(user_account_id, bot_account_id, project_id, platform_id, started_on, latest_post_on)
	VALUES(1, 1, 1, 1, now(), now());

INSERT INTO query_ticket(user_ticket_number, user_id, project_id, close_status, topic, started_on, latest_post_on)
    VALUES(7, 1, 1, 0, 'Disfunctional personal AFHG.', now(), now());

INSERT INTO query_ticket_chat(query_ticket_id, messenger_chat_id) VALUES(1, 1);

INSERT INTO message(user_account_id, bot_account_id, type, external_id, messenger_chat_id, query_ticket_id, content, edited, deleted)
	VALUES
		(1, null ,0, 'AFHG-gram-msg-ngre-sefg-1234-0001', 1, 1, 'Help! My AFHG has failed to AFHG 5 times in a row! I have no idea what is going on! Aaaaa!', FALSE, FALSE),
		(null, 1 ,0, 'AFHG-gram-msg-ngre-sefg-1234-0002', 1, 1, 'Sir, please describe how your AFHG has failed to AFHG.', FALSE, FALSE),
		(1, null ,0, 'AFHG-gram-msg-ngre-sefg-1234-0003', 1, 1, 'Its coming for me! I have to hide. Help me!', FALSE, FALSE);
