# FROM core:latest
FROM ubuntu:24.04


ARG DEBIAN_FRONTEND=noninteractive

# Install probably necessary SSL related
RUN apt update

# Install some basics
# TODO: Check that this is all we need.
RUN apt-get install -y \
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
    sudo curl \
    wget

# Download rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rust-installer.sh
RUN chmod +x rust-installer.sh

# Set the rust update server to fast server
ENV RUSTUP_UPDATE_ROOT=https://fastly-static.rust-lang.org/rustup
ENV RUSTUP_DIST_SERVER=https://fastly-static.rust-lang.org
# Run the download and installation of the toolchain separately

RUN ./rust-installer.sh -y -v --default-toolchain 1.95.0-x86_64-unknown-linux-gnu
RUN rm rust-installer.sh

# We can probably live without setting the variables, but just in case.
ENV CARGO_HOME=/root/.cargo
ENV PATH=$PATH:/root/.cargo/bin/
RUN rustup default

EXPOSE 80
EXPOSE 5432
EXPOSE 7990
EXPOSE 8080
EXPOSE 8088


COPY . /src
RUN mkdir /app
RUN ls -a && cd /src && ls -a && cargo build -p ai-omni-core --target-dir /app

ENTRYPOINT AI_OMNI_CONFIG_PATH=/src/pipeline/fixtures/ai_omni_core_settings.toml /app/debug/ai-omni-core --run-fixtures
