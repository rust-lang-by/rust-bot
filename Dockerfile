FROM rust:1.83.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:b7550f0b15838de14c564337eef2b804ba593ae55d81ca855421bd52f19bb480
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
