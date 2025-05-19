FROM rust:1.87.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:c53c9416a1acdbfd6e09abba720442444a3d1a6338b8db850e5e198b59af5570
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
