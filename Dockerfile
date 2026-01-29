FROM rust:1.93.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:72344f7f909a8bf003c67f55687e6d51a441b49661af8f660aa7b285f00e57df
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
