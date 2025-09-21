FROM rust:1.90.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:620d8b11ae800f0dbd7995f89ddc5344ad603269ea98770588b1b07a4a0a6872
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
