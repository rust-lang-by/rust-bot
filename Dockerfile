FROM rust:1.82.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:6f05aba4de16e89f8d879bf2a1364de3e41aba04f1dcbba8c75494f6134b4b13
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
