ARG BASE_IMAGE=rust:alpine

# Our first FROM statement declares the build environment.
FROM ${BASE_IMAGE} AS builder
RUN apk --no-cache add musl-dev openssl-dev

# Add our source code.
COPY --chown=rust:rust . ./

# Build our application.
RUN cargo build --target x86_64-unknown-linux-musl --release

# Now, we need to build our _real_ Docker container, copying in `rust-bot`.
FROM alpine
RUN apk --no-cache add ca-certificates
COPY --from=builder \
    /home/rust/src/target/x86_64-unknown-linux-musl/release/rust-bot \
    /usr/local/bin/
CMD /usr/local/bin/rust-bot
