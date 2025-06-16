FROM rust:1.87.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:eccec5274132c1be0ce5d2c8e6fe41033e64af5e987ccee9007826e4c012069d
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
