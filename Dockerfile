FROM rust:1.90.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/cc-debian12@sha256:0000f9dc0290f8eaf0ecceafbc35e803649087ea7879570fbc78372df7ac649b
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]
