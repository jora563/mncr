# Down migrations

Откаты БД. В этой папке SQL файлы с откатами БД. Если миграция в принципе не
откатная, то создаётся пустой SQL файл с комментарием:
"Миграция с [год-месяц-день] не подлежит откату".

Формат наименования миграции по умолчанию:

`[YYYYMMDD]_[описание_миграции].sql`

На пример:

`20260829_user_table_add_fields.sql`

NB: This file exists to make sure that git creates the folder. It may need to be deleted later.
