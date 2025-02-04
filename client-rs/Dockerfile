FROM rust:1.72 as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

COPY src ./src

RUN cargo build --release --bin daemon

FROM debian:bullseye

RUN apt-get update && apt-get install -y \
    libpq-dev \
    postgresql \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /usr/src/app/batches

WORKDIR /usr/src/app

COPY --from=builder /usr/src/app/target/release/daemon /usr/src/app/

RUN mkdir -p /var/lib/postgresql/data && chown -R postgres:postgres /var/lib/postgresql

COPY scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

EXPOSE 5432

USER postgres

RUN /usr/lib/postgresql/14/bin/initdb -D /var/lib/postgresql/data

USER root

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]

CMD ["/usr/src/app/daemon"]
