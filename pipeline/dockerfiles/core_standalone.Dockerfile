# A container for running the build and test scripts of this thing.
# build with `docker build --tag=aio-core-standalone -f=pipeline/core_standalone.Dockerfile . `
# The container can be run manually with:
#     `docker run -i -v $PWD:/app  aio-core-standalone:latest /bin/bash`
# TODO: Determine whether we need to localise to MSK
FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

# Set the rust update server to fast server
ENV RUSTUP_UPDATE_ROOT=https://fastly-static.rust-lang.org/rustup
ENV RUSTUP_DIST_SERVER=https://fastly-static.rust-lang.org

# We can probably live without setting the variables, but just in case.
ENV CARGO_HOME=/root/.cargo
ENV PATH=$PATH:/root/.cargo/bin/

RUN apt update \
# Install some basics
    && apt-get install -y \
    gcc \
    openssl \
    libssl-dev \
    sudo curl \
    iputils-ping \
    wget \
    postgresql-16 \
    postgresql-client-16 \
    git \
# Download rust
    &&  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rust-installer.sh \
    && chmod +x rust-installer.sh \
    &&  ./rust-installer.sh -y -v --default-toolchain 1.95.0-x86_64-unknown-linux-gnu \
    && rm rust-installer.sh \
    &&  rustup default \
# Create postgres user
# NB: This must be done in one step or the server shuts down
# before any command is run.
    &&  service postgresql start && service postgresql status && \
    su - postgres -c "psql -c\"CREATE USER aio_core \
    WITH superuser inherit replication bypassrls \
    createdb createrole password 'password'\"" && \
# Some debug lines to make sure that things are created correctly.
su - postgres -c "psql -c\"CREATE DATABASE aio_core OWNER aio_core\"" && \
su - postgres -c "psql -c\"GRANT ALL ON DATABASE aio_core TO aio_core\"" && \
su - postgres -c "psql -c\"CREATE DATABASE ai_omni_test_db_1 OWNER aio_core\"" && \
su - postgres -c "psql -c\"CREATE DATABASE ai_omni_test_db_0 OWNER aio_core\"" && \
su - postgres -c "psql -c\"create user root \
    superuser inherit replication bypassrls \
    createdb createrole password 'password'\"" && \
# Some debug lines to make sure that things are created correctly.
su - postgres -c "psql -c\"create database root owner root\"" && \
su - postgres -c "psql -c\"select datname from pg_database\"" \
&& apt autoremove \
&& apt clean

EXPOSE 7990
EXPOSE 8080
EXPOSE 80
EXPOSE 443
