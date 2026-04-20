FROM rust:1.95.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:847433844c7e04bcf07a3a0f0f5a8de554c6df6fa9e3e3ab14d3f6b73d780235
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
