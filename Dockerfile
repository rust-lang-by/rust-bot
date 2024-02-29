FROM rust:1.76.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:db482c5c6ef39662a0b55a718e19b8c0cabfbbd57c2dd342f1791c4f096d0b7b
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
