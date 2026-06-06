# Minimal CI image for downstream truST test jobs (e.g. plc-tests).
#
# Ships only the release binaries plus their runtime dependencies — no Rust
# toolchain. Binaries are built in a separate CI stage (see .gitlab-ci.yml) and
# copied in from dist/ci/bin/.
#
# Build locally:
#   cargo build --release -p trust-dev -p trust-runtime -p trust-lsp
#   mkdir -p dist/ci/bin
#   cp target/release/trust-dev target/release/trust-runtime \
#      target/release/trust-lsp target/release/trust-bundle-gen dist/ci/bin/
#   docker build -f docker/ci/trust-ci.Dockerfile -t trust-ci:local .
#
# Run:
#   docker run --rm -v "$PWD":/workspace -w /workspace trust-ci:local \
#     trust-dev test --project . --ci --output junit

FROM debian:bookworm-slim

ARG TRUST_VERSION=dev
LABEL org.opencontainers.image.title="trust-platform-ci" \
      org.opencontainers.image.description="Minimal CI image with truST release binaries (trust-dev, trust-runtime, trust-lsp, trust-bundle-gen)" \
      org.opencontainers.image.source="https://gitlab.com/otmatic-group/product/trust-platform" \
      org.opencontainers.image.version="${TRUST_VERSION}"

# ca-certificates for HTTPS; curl is handy for downstream jobs fetching fixtures.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1000 --user-group --create-home --shell /usr/sbin/nologin trust

# Pre-built linux-x86_64 release binaries, produced by the build-ci-binaries job.
COPY dist/ci/bin/ /opt/trust/bin/
ENV PATH="/opt/trust/bin:${PATH}"

# Fail the image build if the binaries are missing or non-functional.
RUN trust-dev --version && trust-runtime --version

WORKDIR /workspace
USER 1000:1000
