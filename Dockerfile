FROM rust:1-alpine3.15 as builder
# This is important, see https://github.com/rust-lang/docker-rust/issues/85
ENV RUSTFLAGS="-C target-feature=-crt-static" \
    APP_TARGET=/app/target/release/stat_server

RUN apk add --no-cache musl-dev git

WORKDIR /app
COPY ./ /app

RUN cargo build --release --bin stat_server
RUN strip /app/target/release/stat_server
# RUN strip /app/target/release/stat_client

FROM alpine:3.15 as production
RUN apk add --no-cache libgcc
COPY docker-entrypoint.sh /
COPY --from=builder /app/target/release/stat_server /stat_server
# COPY --from=builder /app/target/release/stat_client /stat_client

WORKDIR /
EXPOSE 8080
ENTRYPOINT ["/bin/sh", "/docker-entrypoint.sh"]
# CMD ["/stat_server", "-c", "/config.toml"]
