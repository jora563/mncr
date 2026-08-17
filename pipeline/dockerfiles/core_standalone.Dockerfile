# A container for running the build and test scripts of this thing.
# build with `docker build --tag=aio-core-standalone -f=pipeline/core_standalone.Dockerfile . `
# The container can be run manually with:
#     `docker run -i -v $PWD:/app aio-core-standalone:latest /bin/bash`
# TODO: Determine whether we need to localise to MSK
FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

ENV RUSTUP_UPDATE_ROOT=https://fastly-static.rust-lang.org/rustup
ENV RUSTUP_DIST_SERVER=https://fastly-static.rust-lang.org
ENV CARGO_HOME=/root/.cargo
ENV PATH=/root/.cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    gcc \
    git \
    iputils-ping \
    libssl-dev \
    openssl \
    pkg-config \
    postgresql-16 \
    postgresql-client-16 \
    sudo \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Download rust
RUN curl --silent --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.95.0-x86_64-unknown-linux-gnu \
    && export PATH="/root/.cargo/bin:$PATH" \
    && rustup default

# ВСТРОЕННЫЙ SMOKE-ТЕСТ (Сработает при сборке docker build)
# Проверяем компилятор Rust, временно поднимаем Postgres и тестируем создание баз данных
RUN echo "=== ЗАПУСК SMOKE-ТЕСТОВ ===" \
    && cargo --version \
    && rustc --version \
    && echo "Проверка Rust: УСПЕШНО" \
    && echo "Запуск временного инстанса PostgreSQL для проверки..." \
    && pg_ctlcluster 16 main start \
    && until pg_isready -q; do sleep 1; done \
    && sudo -u postgres psql -c "CREATE USER aio_core WITH superuser inherit password 'password'" \
    && sudo -u postgres psql -c "CREATE DATABASE aio_core OWNER aio_core" \
    && sudo -u postgres psql -c "CREATE DATABASE ai_omni_test_db_0 OWNER aio_core" \
    && echo "Тестирование подключения к созданной базе данных..." \
    && PGPASSWORD=password psql -h localhost -U aio_core -d ai_omni_test_db_0 -c "SELECT 'База данных отвечает корректно!' AS test_status;" \
    && pg_ctlcluster 16 main stop \
    && echo "=== SMOKE-ТЕСТЫ УСПЕШНО ПРОЙДЕНЫ ==="

RUN cat << 'EOF' > /entrypoint.sh
#!/bin/bash
service postgresql start

# Ждем, пока Postgres поднимется
until pg_isready -q; do sleep 1; done

# Проверяем, созданы ли уже базы (чтобы не дублировать при перезапуске)
if ! psql -U postgres -lqt | cut -d \| -f 1 | grep -qw aio_core; then
  psql -U postgres -c "CREATE USER aio_core WITH superuser inherit replication bypassrls createdb createrole password 'password'"
  psql -U postgres -c "CREATE DATABASE aio_core OWNER aio_core"
  psql -U postgres -c "GRANT ALL ON DATABASE aio_core TO aio_core"
  psql -U postgres -c "CREATE DATABASE ai_omni_test_db_1 OWNER aio_core"
  psql -U postgres -c "CREATE DATABASE ai_omni_test_db_0 OWNER aio_core"
  psql -U postgres -c "CREATE USER root WITH superuser inherit replication bypassrls createdb createrole password 'password'"
  psql -U postgres -c "CREATE DATABASE root OWNER root"
fi

exec "$@"
EOF

RUN chmod +x /entrypoint.sh

EXPOSE 7990 8080 80 443

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/bash"]

HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
  CMD pg_isready -h localhost -U postgres -d aio_core || exit 1
