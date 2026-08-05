# Файл для базы данных.
# Условно контейнер где содержится БД postgresql
FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt update && apt-get install -y \
    postgresql-16 postgresql-client-16 \
    git

# Create postgres user
# NB: This must be done in one step or the server shuts down
# before any command is run.
RUN service postgresql start && service postgresql status && \
su - postgres -c "psql -c\"CREATE USER aio_core \
    WITH superuser inherit replication bypassrls \
    createdb createrole password 'password'\"" && \
# Some debug lines to make sure that things are created correctly.
su - postgres -c "psql -c\"CREATE DATABASE aio_core OWNER aio_core\"" && \
su - postgres -c "psql -c\"GRANT ALL ON DATABASE aio_core TO aio_core\""

EXPOSE 5432

ENV PG_MAJOR=16
ENV PATH=$PATH:/usr/lib/postgresql/$PG_MAJOR/bin

CMD ["/usr/lib/postgresql/16/bin/postgres"]
