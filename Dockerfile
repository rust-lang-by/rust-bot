FROM rust:1.72.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
