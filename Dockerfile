FROM rust:1.94-alpine3.23 AS builder

WORKDIR /app
COPY ./ /app

RUN apk add --no-cache musl-dev linux-headers git cmake make g++
RUN cargo build --release --locked --bin stat_server
RUN strip /app/target/release/stat_server

FROM scratch AS production
LABEL maintainer="doge.py@gmail.com" \
    description="A simple server monitoring tool"

COPY --from=builder /app/config.toml /config.toml
COPY --from=builder /app/target/release/stat_server /stat_server

WORKDIR /
EXPOSE 8080 9394

CMD ["/stat_server", "-c", "/config.toml"]
