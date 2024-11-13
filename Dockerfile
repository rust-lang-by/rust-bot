FROM rust:1.82.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:2fb69596e692931f909c4c69ab09e50608959eaf8898c44fa64db741a23588b0
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
