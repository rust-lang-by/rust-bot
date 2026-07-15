FROM rust:1.96.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:7ee09f36862efbdbf70422db263e411c2618409ca46faa555bd5b636155307df
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
