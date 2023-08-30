FROM rust:latest AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc@sha256:b53fbf5f81f4a120a489fedff2092e6fcbeacf7863fce3e45d99cc58dc230ccc
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]