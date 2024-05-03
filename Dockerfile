FROM rust:1.78.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:eed8bd290a9f83d0451e7812854da87a8407f1d68f44fae5261c16556be6465b
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
