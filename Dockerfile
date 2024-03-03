# Build stage
FROM rust:1.76.0-buster AS builder

RUN apt-get update && apt-get install -y libssl-dev pkg-config

WORKDIR /usr/src/place-rs
COPY . .

RUN cargo install --path .
RUN ./bundle.sh

# Run stage
FROM debian:buster-slim

RUN apt-get update && apt-get install -y libssl1.1 sqlite3

COPY --from=builder /usr/local/cargo/bin/place-rs /usr/local/bin/place-rs
COPY ./public /var/www/html
RUN mkdir /data
RUN sqlite3 /data/database.db

VOLUME /data
EXPOSE 3000

CMD ["place-rs"]