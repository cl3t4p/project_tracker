# Stage 1: Build frontend
FROM node:22-alpine AS frontend-build
WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# Stage 2: Build backend
FROM rust:1.87-bookworm AS backend-build
WORKDIR /app
COPY backend/ .
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-build /app/target/release/project-tracker-backend .
COPY --from=frontend-build /app/dist ./static

EXPOSE 8080
VOLUME /app/data
ENV DATABASE_PATH=/app/data/projects.db

CMD ["./project-tracker-backend"]
