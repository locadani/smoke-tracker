# syntax=docker/dockerfile:1
# A static musl binary on a distroless base: no shell, no package manager, about 15–20 MB.

FROM rust:1-alpine AS toolchain
RUN apk add --no-cache gcc musl-dev
# cargo-auditable embeds the dependency list in the binary, so Trivy can scan the Rust crates
# inside it. cargo-chef builds the dependencies in a layer of their own.
RUN cargo install cargo-auditable --version 0.7.7 --locked \
    && cargo install cargo-chef --version 0.1.78 --locked
WORKDIR /src

# Only the manifests reach the recipe, so the cook layer stays cached until a dependency changes.
FROM toolchain AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM toolchain AS build
ENV SQLX_OFFLINE=true
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --locked --bin smoke-tracker --recipe-path recipe.json
COPY . .
RUN cargo auditable build --release --locked --bin smoke-tracker \
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
