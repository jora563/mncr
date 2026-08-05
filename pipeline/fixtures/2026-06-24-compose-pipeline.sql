-- Это более или менее минимальный скрипт для поднятия БД с минимальными данными.
-- Потом система сама будет добавлять пользователей, их учётные записи, и так далее.

INSERT INTO public.project_group (external_id,group_name,created_on,altered_on) VALUES
	 ('GG-1','Good Group','2026-06-11 16:20:40.218781','2026-06-11 16:20:40.218781'),
	 ('GG-2','Bad Group','2026-06-11 16:21:36.503544','2026-06-11 16:21:36.503544')
	 ON CONFLICT(id) DO NOTHING;
INSERT INTO public.project (project_group_id,external_id,project_name,created_on,altered_on) VALUES
	 (1,'P-1','Good Project I','2026-06-11 16:24:33.498022','2026-06-11 16:24:33.498022'),
	 (1,'P-2','Good Project II','2026-06-11 16:24:50.651012','2026-06-11 16:24:50.651012'),
	 (1,'P-3','Good Project III','2026-06-11 16:25:10.34503','2026-06-11 16:25:10.34503'),
	 (2,'BP-1','Bad Project','2026-06-11 16:25:35.47118','2026-06-11 16:25:35.47118')
	 ON CONFLICT(id) DO NOTHING;

INSERT INTO public.platform (api_id,"name",created_on,altered_on) VALUES
	 (1,'Вкак','2026-06-11 16:34:24.84347','2026-06-11 16:34:24.84347'),
	 (2,'Вагон','2026-06-11 16:34:24.846495','2026-06-11 16:34:24.846495'),
	 (2,'Тележка','2026-06-11 16:34:24.848411','2026-06-11 16:34:24.848411')
	 ON CONFLICT(id) DO NOTHING;
INSERT INTO public.platform_mirror (platform_id,url,note) VALUES
	 (1,'vk.kk','yes'),
	 (1,'wagon.com','no'),
	 (2,'wag.on','no'),
	 (2,'cart.com','no'),
	 (3,'ca.rt','maybe')
	 ON CONFLICT(platform_id,"url") DO NOTHING;


INSERT INTO public."user" (phone,designation) VALUES
	 ('79451100022','Bob'),
	 ('+79452200022','Alice'),
	 ('+79453300022','Rick'),
	 ('+78001100022','Morty')
	 ON CONFLICT(id) DO NOTHING;

INSERT INTO public.user_account (platform_id,user_id,external_id,alias) VALUES
	 (1,2,'TG-999','Alice'),
	 (3,2,'TG-999','Alice'),
	 (2,2,'TG-999','Alice')
	 ON CONFLICT(platform_id,external_id) DO NOTHING;

-- НБ: Тут надо добавить токены автторизации ботов в правильным формате до запуска.
INSERT INTO public.bot_account (platform_id,external_id,expiry_time_hours,"token") VALUES
	 (2,'WGB-1',6,'5197913983:AAF7vSQpinqjvjQkMYwiG7TiDv_pxk4XjCE'),
	 (1,'VKB-1',24,'239313626::vk1.a.jwA-jvAvRFcBKquRzTP3gcVClxBVU-8sg4lnObaVV2M-SK4QH1wrAtJMR9aStekMbNetpgp-qQJAsnBqT7nokW2Kox8JLUV0kqrBoD6xjBo67mxIUfcB4mnqcFk7wTbJmfDU51-Crcq2brwbiCqnqoCIwvVgrqbc5_N5eUtkGXwztQUI0KXl4skbMis4usgpAOh3A0-4slvbvOpSbpgc0w')
	 ON CONFLICT(platform_id,external_id) DO NOTHING;


INSERT INTO public.user_account_project (account_id,project_id) VALUES
	 (1,2),
	 (3,1),
	 (2,1),
	 (3,2)
	 ON CONFLICT(account_id,project_id) DO NOTHING;
INSERT INTO public.bot_account_project (account_id,project_id) VALUES
	 (1,2),
	 (2,2)
	 ON CONFLICT(account_id,project_id) DO NOTHING;
INSERT INTO public.project_platform (project_id,platform_id) VALUES
	 (1,1),
	 (2,1),
	 (3,1),
	 (1,2),
	 (2,2),
	 (3,3)
	 ON CONFLICT(project_id,platform_id) DO NOTHING;
