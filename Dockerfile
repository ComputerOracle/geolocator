# Stage 1: Build Vite Frontend Assets
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust Backend Binary
FROM rust:1.83-bookworm AS backend-builder
WORKDIR /app
RUN apt-get update && apt-get install -y libdbus-1-dev pkg-config && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src
COPY tests/ ./tests
RUN cargo build --release

# Stage 3: Minimal Production Runtime
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libdbus-1-3 ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/target/release/geolocator /usr/local/bin/geolocator
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
COPY openapi.yaml /app/openapi.yaml

ENV PORT=8080
ENV MOCK_BLUETOOTH=true
EXPOSE 8080

CMD ["geolocator"]
