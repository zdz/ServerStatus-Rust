FROM rust:1.69-alpine3.17 as builder

WORKDIR /app
COPY ./ /app

RUN apk add --no-cache musl-dev git cmake make g++
RUN cargo build --release --bin stat_server
RUN strip /app/target/release/stat_server

FROM scratch as production
LABEL maintainer="doge.py@gmail.com" \
    description="A simple server monitoring tool"

COPY --from=builder /app/config.toml /config.toml
COPY --from=builder /app/target/release/stat_server /stat_server

WORKDIR /
EXPOSE 8080 9394

CMD ["/stat_server", "-c", "/config.toml"]
