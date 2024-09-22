FROM rust:1.81.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:682ff941956437ab1fc0f6fe969b18ede078839cc4f4fbc156ab546d2a9055fd
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
