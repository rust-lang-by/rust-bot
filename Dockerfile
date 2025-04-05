FROM rust:1.86.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:c1cbcec08d39c81adbefb80cabc51cba285465866f7b5ab15ddb2fcae51a1aed
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
