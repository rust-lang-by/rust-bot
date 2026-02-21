FROM rust:1.93.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:329e54034ce498f9c6b345044e8f530c6691f99e94a92446f68c0adf9baa8464
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
