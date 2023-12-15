FROM rust:1.74.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:6714977f9f02632c31377650c15d89a7efaebf43bab0f37c712c30fc01edb973
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
