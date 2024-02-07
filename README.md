# Pixelrust
My implementation of the [pixelflut](https://github.com/defnull/pixelflut) protocol in rust. It's a simple server that listens for TCP connections and sends pixel data to the connected clients. The server is multithreaded and can handle multiple clients at once.

The frontend uses webassembly for decoding [QOI](https://en.wikipedia.org/wiki/QOI_(image_format)) data sent by the backend to update the canvas in realtime.

The canvas is periodically saved to a file called `image.qoi`. This file is used to restore the canvas when the server is restarted.

## Server Protocol
The server listens for TCP connections on port 1337. The server expects the client to send the following commands:
- `PX x y rrggbb` - Set the pixel at position (x, y) to the color rrggbb.
- `SIZE` - Get the size of the canvas.
- `PX x y` - Get the color of the pixel at position (x, y).
- `QUIT` - Close the connection.
- `HELP` - Get a list of all commands.
- `BIN` - Enable binary mode. In binary mode, the server will send the pixel data in binary format. This is useful for sending large amounts of pixel data.

Binary mode is disabled by default. To enable it, the client has to send the `BIN` command. The server will then only accept binary data on that socket. To disable binary mode, the client needs to close the connection and open a new one.

Pixels in binary mode are sent in the following format:
- 2 bytes for the x position (little endian)
- 2 bytes for the y position (little endian)
- 4 bytes for the color (rrggbbaa) (little endian, therefore it is aabbggrr)

Binary mode on average is about half the size of the text mode, so it is recommended to use binary mode when sending large amounts of pixel data. 
## Usage
### Docker (recommended)
It is easiest and probably best to run this project using docker. There currently are no published images, so you have to build the image yourself. You can do this by running the following command in the root directory of this project:
```sh
docker build -t pixelrust .
```

After the image is built, you can run the server using the following command:
```sh
docker run -p 8080:8080 -p 1337:1337 -v $(pwd)/image.qoi:/app/image.qoi: pixelrust
```

The frontend will then be running on port 8080 and the pixelflut server will be running on port 1337.

### Without docker
You can also run the build the server without docker.
For that you will need to have rust (including the wasm target), cargo, node, make & caddy installed. You can then build the server using the following command in the root directory of this project:
```sh
make build
```

After the server has been built, there will be a folder called `build` in the root directory of this project. You can then run the server using the following commands:
```sh
cd build
./pixelrust &
caddy run
```

The frontend will then be running on port 8080 and the pixelflut server will be running on port 1337.

## Configuration
The server isn't configurable, but changing the canvas size is possible by replacing the `image.qoi` file with a new one, as the server will use the file to restore the canvas when it starts and therefore read its size and use it everywhere.

## License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
