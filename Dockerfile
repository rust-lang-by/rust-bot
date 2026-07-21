FROM rust:1.97.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:e8e7ee4b8b106d4c5fde9e422a321b2b8a2d5cca546c97adcce927f3e1d36e36
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
