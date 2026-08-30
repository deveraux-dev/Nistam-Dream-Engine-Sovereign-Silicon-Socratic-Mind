# Multi-stage build for forge-envelope and agent_loop
FROM rust:1.80-slim as builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
COPY tests ./tests
RUN cargo build --release --bin attest --bin chaos_monkey --features cli

FROM python:3.11-slim
WORKDIR /app
COPY --from=builder /app/target/release/attest /usr/local/bin/attest
COPY --from=builder /app/target/release/chaos_monkey /usr/local/bin/chaos_monkey
COPY . /app
RUN pip install --no-cache-dir google-genai pydantic google-cloud-firestore google-cloud-storage

ENV FORGE_ENVELOPE_BIN=/usr/local/bin/attest
ENV PYTHONUNBUFFERED=1

CMD ["python", "scripts/agent_loop.py"]
