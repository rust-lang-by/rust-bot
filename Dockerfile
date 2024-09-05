FROM rust:1.80.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:b6e1e913f633495eeb80a41e03de1a41aa863e9b19902309b180ffdc4b99db2c
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
