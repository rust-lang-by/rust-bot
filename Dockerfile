FROM rust:1.76.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:899570acf85a1f1362862a9ea4d9e7b1827cb5c62043ba5b170b21de89618608
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
