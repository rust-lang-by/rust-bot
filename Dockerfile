FROM rust:1.93.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:66d87e170bc2c5e2b8cf853501141c3c55b4e502b8677595c57534df54a68cc5
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
