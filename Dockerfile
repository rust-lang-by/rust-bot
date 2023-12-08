FROM rust:1.74.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:a9056d2232d16e3772bec3ef36b93a5ea9ef6ad4b4ed407631e534b85832cf40
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
