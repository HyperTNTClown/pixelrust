# Build
FROM rust:bookworm as builder
RUN rustup target add wasm32-unknown-unknown
RUN curl -fsSL https://raw.githubusercontent.com/mklement0/n-install/stable/bin/n-install | bash -s 22
WORKDIR /build
COPY . .
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall -y wasm-pack wasm-bindgen-cli
RUN apt update && apt install -y binaryen build-essential
RUN make build
# Run
FROM debian:bookworm as runner
RUN apt update && apt install -y curl caddy
WORKDIR /app
COPY --from=builder /build/build/ /app/
RUN touch image.qoi
EXPOSE 1337
EXPOSE 8080
CMD caddy run & ./pixelrust
