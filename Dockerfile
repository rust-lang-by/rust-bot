FROM rust:1.89.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:00cc20b928afcc8296b72525fa68f39ab332f758c4f2a9e8d90845d3e06f1dc4
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
