FROM rust:1.96.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:a90cf0f046efb32466b38b0972fef3a95e7c580e392e79ff1b7ac08c15fed0bc
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
