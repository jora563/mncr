SELECT * FROM project_group;
SELECT * FROM project;
SELECT * FROM platform;
SELECT * FROM project_platform;
SELECT * FROM bot_account;
SELECT * FROM user;
SELECT * FROM user_account;
SELECT * FROM user_account_project;
SELECT * FROM bot_account_project;
SELECT * FROM project_user;
SELECT * FROM messenger_chat;
SELECT * FROM query_ticket;
SELECT * FROM query_ticket_chat;
select * from message;

SELECT * FROM message
	WHERE
		query_ticket_id=(SELECT id FROM query_ticket WHERE user_ticket_number=7 AND project_id=1)
		AND user_account_id IS NOT NULL
	ORDER BY message.created_on ASC;
