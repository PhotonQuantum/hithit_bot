FROM rust:alpine AS builder

WORKDIR /work

RUN apk update && apk add musl-dev

COPY src ./src

COPY Cargo.toml Cargo.lock ./

RUN cargo build --bin hithit_bot --release

FROM alpine:latest

WORKDIR /work

COPY --from=builder ./work/target/release/hithit_bot ./

EXPOSE 8080

CMD ["./hithit_bot"]
