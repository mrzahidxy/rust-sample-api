FROM rust:1.99-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --locked --release --bins

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 app
COPY --from=builder /app/target/release/{{project-name}} /usr/local/bin/server
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate
USER app
ENV APP_ADDRESS=0.0.0.0:3000
EXPOSE 3000
CMD ["server"]
