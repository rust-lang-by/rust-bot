FROM rust:1.82.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:3310655aac0d85eb9d579792387af1ff3eb7a1667823478be58020ab0e0d97a8
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
