FROM rust:1.77.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:e6ae66a5a343d7112167f9117c4e630cfffcd80db44e44302759ec13ddd2d22b
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
