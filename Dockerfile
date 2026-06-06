FROM rust:1.96.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:aa0b7af67fa8211751ea6e00baa8373ba56cc1417ffc986ec9619bd0e1556b56
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
