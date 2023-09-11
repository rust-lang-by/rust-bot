FROM rust:latest AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc@sha256:3603adbdee2906dc3b7a18d7c0424a40633231c61dcd82196ae15de1282a5822
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]