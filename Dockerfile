FROM rust:1.71 AS builder

WORKDIR /usr/src/meow-coverage
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/meow-coverage /usr/local/bin/meow-coverage
ADD entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
