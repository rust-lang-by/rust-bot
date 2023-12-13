FROM rust:1.74.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:4ddfea445cfeed54d6c9a1e51b97e7f3a5087f3a6a69cb430ebba3a89c402a41
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
