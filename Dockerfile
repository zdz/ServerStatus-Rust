FROM rust:1-alpine3.15 as builder
# This is important, see https://github.com/rust-lang/docker-rust/issues/85
ENV RUSTFLAGS="-C target-feature=-crt-static" \
    APP_TARGET=/app/target/release/stat_srv

RUN apk add --no-cache musl-dev upx openssl-dev

WORKDIR /app
COPY ./ /app

RUN cargo build --release
RUN strip /app/target/release/stat_srv
RUN upx -q --best --lzma /app/target/release/stat_srv

FROM alpine:3.15 as production
RUN apk add --no-cache libgcc
COPY --from=builder /app/target/release/stat_srv /stat_srv

WORKDIR /
EXPOSE 8080
ENTRYPOINT ["/stat_srv", "-c", "/config.toml"]
