FROM rust:1.80.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:3b75fdd33932d16e53a461277becf57c4f815c6cee5f6bc8f52457c095e004c8
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
