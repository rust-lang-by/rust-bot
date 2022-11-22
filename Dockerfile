####################################################################################################
## Builder
####################################################################################################
FROM rust:alpine AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apk --no-cache add musl-dev openssl-dev
RUN update-ca-certificates

# Create appuser
ENV USER=rust-bot
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /rust-bot

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release

####################################################################################################
## Final image
####################################################################################################
FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /rust-bot

# Copy our build
COPY --from=builder /rust-bot/target/x86_64-unknown-linux-musl/release/rust-bot ./

# Use an unprivileged user.
USER rust-bot:rust-bot

CMD ["/rust-bot/rust-bot"]