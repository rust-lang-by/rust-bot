FROM rust:1.76.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:4049e8f163161818a52e028c3c110ee0ba9d71a14760ad2838aabba52b3f9782
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
