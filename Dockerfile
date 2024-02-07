# Build
FROM rust:bookworm as builder
RUN rustup target add wasm32-unknown-unknown
RUN curl -Ssf --proto '=https' https://pkgx.sh | sh
WORKDIR /build
COPY . .
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall -y wasm-pack wasm-bindgen-cli
RUN apt update && apt install -y binaryen
RUN pkgx +node@21 +make make build
# Run
FROM debian:bookworm as runner
RUN apt update && apt install -y curl
RUN curl -Ssf --proto '=https' https://pkgx.sh | sh
WORKDIR /app
COPY --from=builder /build/build/ /app/
RUN touch image.qoi
EXPOSE 1337
EXPOSE 8080
CMD pkgx +caddy caddy run & ./pixelrust
