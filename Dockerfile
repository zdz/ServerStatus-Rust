FROM rust:1-alpine3.16 as builder
# This is important, see https://github.com/rust-lang/docker-rust/issues/85
ENV RUSTFLAGS="-C target-feature=-crt-static"
ENV RUST_BACKTRACE=full

WORKDIR /app
COPY ./ /app

RUN apk add --no-cache musl-dev git cmake make g++
RUN cargo build --release --bin stat_server
RUN strip /app/target/release/stat_server

FROM alpine:3.16 as production
LABEL version="1.0.0" \
    description="A simple server monitoring tool" \
    by="Doge" \
    maintainer="doge.py@gmail.com"

RUN apk add --no-cache libgcc
COPY --from=builder /app/target/release/stat_server /stat_server

WORKDIR /
EXPOSE 8080 9394

CMD ["/stat_server", "-c", "/config.toml"]
