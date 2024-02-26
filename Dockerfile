FROM rust:1.76.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:efafe74d452c57025616c816b058e3d453c184e4b337897a8d38fef5026b079d
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
