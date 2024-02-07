build:
	mkdir build
	cd app && npm ci && npm run build && cp -r dist ../build
	cargo build --release && cp target/release/pixelrust build
	cp Caddyfile build

clean:
	rm -rf build
	cargo clean
	cd app && npm run clean && cargo clean