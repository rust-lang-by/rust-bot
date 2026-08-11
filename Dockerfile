FROM rust:1.97.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:6e1871c34683dc9ee996d13084497783fd98ac0200213d0826625f4e9d4be1d0
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
