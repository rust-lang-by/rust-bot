FROM rust:1.78.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:e1065a1d58800a7294f74e67c32ec4146d09d6cbe471c1fa7ed456b2d2bf06e0
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
