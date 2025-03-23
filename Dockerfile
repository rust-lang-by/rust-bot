FROM rust:1.85.1 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:85dac24dd2f03e841d986d5ed967385d3a721dcd9dbd21b602ddd82437f364c9
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
