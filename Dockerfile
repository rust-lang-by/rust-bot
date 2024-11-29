FROM rust:1.82.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:f913198471738d9eedcd00c0ca812bf663e8959eebff3a3cbadb027ed9da0c38
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
