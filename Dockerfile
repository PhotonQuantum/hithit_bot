FROM rust:slim-bullseye AS builder

WORKDIR /work

RUN apt-get -y update

RUN apt-get -y install pkg-config libssl-dev

COPY src ./src

COPY Cargo.toml Cargo.lock ./

RUN cargo build --bin hithit_bot --release

FROM debian:bullseye-slim

WORKDIR /work

COPY --from=builder ./work/target/release/hithit_bot ./

CMD ["./hithit_bot"]
