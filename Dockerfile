
FROM rust:1.86 AS builder

WORKDIR /usr/src/app
COPY . .

RUN cargo build --release
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

RUN groupadd -r appuser && useradd -r -g appuser appuser

COPY --from=builder /usr/src/app/target/release/cursor-tracking-rs /usr/local/bin/

RUN chown appuser:appuser /usr/local/bin/cursor-tracking-rs
USER appuser

ENTRYPOINT ["cursor-tracking-rs"]