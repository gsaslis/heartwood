# Builds release binaries for Radicle.
FROM rust:1.77.2-alpine3.19 as builder
LABEL maintainer="Radicle Team <team@radicle.xyz>"
WORKDIR /src
COPY . .

# Copy cargo configuration we're going to use to specify compiler options.
RUN mkdir -p .cargo && cp build/config.toml .cargo/config.toml
# Install dependencies.
RUN apk update && apk add --no-cache mold clang git musl-dev minisign curl xz asciidoctor
# Build man pages.
RUN asciidoctor --doctype manpage --backend manpage --destination-dir . *.1.adoc
# Add cargo targets.
RUN rustup target add \
    x86_64-unknown-linux-musl \
    aarch64-unknown-linux-musl \
    x86_64-apple-darwin \
    aarch64-apple-darwin

# Linux x86_64 and aarch64 build.
ENV CC=clang C_INCLUDE_PATH=/usr/include/x86_64-linux-musl
RUN cargo build --locked --release \
    --target=x86_64-unknown-linux-musl \
    --target=aarch64-unknown-linux-musl \
    -p radicle-node \
    -p radicle-httpd \
    -p radicle-remote-helper \
    -p radicle-cli

# Install dependencies for cross-compiling to macOS.
# We use Zig as the linker to perform the compilation from a Linux host.
# Zig is not yet available on Debian, so we download the official binary.
# Compilation is done via `cargo-zigbuild` which is a wrapper around `zig`.
RUN curl -sSf -o zig.tar.xz         https://ziglang.org/download/0.11.0/zig-linux-x86_64-0.11.0.tar.xz && \
    curl -sSf -o zig.tar.xz.minisig https://ziglang.org/download/0.11.0/zig-linux-x86_64-0.11.0.tar.xz.minisig && \
    minisign -Vm zig.tar.xz -P RWSGOq2NVecA2UPNdBUZykf1CCb147pkmdtYxgb3Ti+JO/wCYvhbAb/U && \
    xz -d -c zig.tar.xz | tar -x && \
    mv zig-linux-x86_64-0.11.0/zig /usr/bin/zig && \
    mv zig-linux-x86_64-0.11.0/lib /usr/lib/zig && \
    cargo install cargo-zigbuild

# Parts of the macOS SDK are required to build Radicle, we make these available
# here. So far only `CoreFoundation` and `Security` frameworks are needed.
RUN xz -d -c build/macos-sdk-11.3.tar.xz | tar -x
# This env var is used by `cargo-zigbuild` to find the SDK.
ENV SDKROOT /src/macos-sdk-11.3

# Darwin x86_64 and aarch64 build.
RUN cargo zigbuild --locked --release \
    --target=x86_64-apple-darwin \
    --target=aarch64-apple-darwin \
    -p radicle-node \
    -p radicle-httpd \
    -p radicle-remote-helper \
    -p radicle-cli

# Now copy the files to a new image without all the intermediary artifacts to
# save some space.
FROM alpine:3.19
COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/*  /bin/x86_64-unknown-linux-musl/
COPY --from=builder /src/target/aarch64-unknown-linux-musl/release/* /bin/aarch64-unknown-linux-musl/
COPY --from=builder /src/target/x86_64-apple-darwin/release/*        /bin/x86_64-apple-darwin/
COPY --from=builder /src/target/aarch64-apple-darwin/release/*       /bin/aarch64-apple-darwin/
COPY --from=builder /src/*.1                                         /src/
