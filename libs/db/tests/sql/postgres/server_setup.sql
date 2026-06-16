-- This is the script for setting up the initial user.
-- It should be run once when setting up the instance or container
-- it does not need to be run after that.
CREATE USER aio_core WITH SUPERUSER PASSWORD 'password';
CREATE DATABASE aio_core;
ALTER DATABASE aio_core OWNER TO aio_core;
GRANT ALL ON DATABASE aio_core WITH ENCODING 'UTF8' LC_COLLATE='pg_unicode_fast' TO aio_core;
