# Build stage
FROM rust:latest AS builder
WORKDIR /app

# btleplug (our Bluetooth library) needs this system library to compile.
RUN apt-get update && apt-get install -y libdbus-1-dev pkg-config && rm -rf /var/lib/apt/lists/*

COPY . .
RUN cargo build --release

# Run stage - a much smaller image with just what's needed to run the binary.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libdbus-1-3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/geolocator /usr/local/bin/geolocator

CMD ["geolocator"]
