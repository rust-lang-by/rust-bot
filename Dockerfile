FROM rust:1.96.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:d703b626ba455c4e6c6fbe5f36e6f427c85d51445598d564652a2f334179f96e
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
