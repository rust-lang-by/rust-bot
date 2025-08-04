FROM rust:1.88.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:aa435f48941dbbd18b4a1f3f71992a3afddc6fb913beb411cd4c0fb174e0bfb8
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
