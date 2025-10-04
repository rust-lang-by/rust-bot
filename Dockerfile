FROM rust:1.90.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:14f6999db515330e5d00537bd457289a8968b6456e9197c7a28101ee63a7522f
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
