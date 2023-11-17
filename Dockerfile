FROM redhat/ubi9:9.3-1361.1699548029 AS ubi-base
ARG micromount=/mnt/rootfs
RUN mkdir -p $micromount
RUN yum install \
    --installroot $micromount \
    --releasever 9 \
    --setopt install_weak_deps=false \
    --nodocs -y \
    openssl
RUN yum clean all \
    --installroot $micromount

FROM rust:1.74.0 AS build-env
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM redhat/ubi9-micro:9.3
COPY --from=ubi-base $micromount /
COPY --from=build-env /app/target/release/rust-bot ./
CMD ["/rust-bot"]