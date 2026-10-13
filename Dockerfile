# syntax=docker/dockerfile:1
# A static musl binary on a distroless base: no shell, no package manager, about 15–20 MB.

FROM rust:1-alpine AS build
RUN apk add --no-cache gcc musl-dev
# Embeds the dependency list in the binary, so Trivy can scan the Rust crates inside it.
RUN cargo install cargo-auditable --version 0.7.7 --locked
WORKDIR /src
COPY . .
ENV SQLX_OFFLINE=true
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo auditable build --release --locked --bin smoke-tracker \
    && cp target/release/smoke-tracker /smoke-tracker \
    && mkdir /data

FROM gcr.io/distroless/static-debian13:nonroot
COPY --from=build /smoke-tracker /usr/local/bin/smoke-tracker
COPY --from=build --chown=65532:65532 /data /data
ENV DATABASE_URL=sqlite:///data/smoke.db \
    BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
VOLUME ["/data"]
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/smoke-tracker"]
CMD ["serve"]
