FROM rust:1.93.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:0c8eac8ea42a167255d03c3ba6dfad2989c15427ed93d16c53ef9706ea4691df
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
