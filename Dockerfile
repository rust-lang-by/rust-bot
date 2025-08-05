FROM rust:1.88.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:bf0494952368db47e9e38eecc325c33f5ee299b1b1ccc5d9e30bdf1e5e4e3a58
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
