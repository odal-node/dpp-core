# The harness the W3C VC-JOSE-COSE test suite runs, as an image.
#
# Build context is the repository root, since the harness takes `dpp-vc` and
# `dpp-crypto` by path:
#
#   docker build -f w3c-suites/vc-jose-cose/Dockerfile -t vc-jose-cose-suite:local .
#
# `Dockerfile.dockerignore` lets only the four paths the build reads into the
# context. `rust-toolchain.toml` is not one of them: the toolchain is the one the
# builder image names, which is the version that file pins. Keep the two in step.
#
# Both base images are pinned by digest, so the bytes a build starts from are the
# bytes that were read. Dependabot does not watch them; moving one is a deliberate
# edit. The tag in front of each digest says what was pinned.

FROM rust:1.100.0-slim-bookworm@sha256:7c7d4eb4d05b7781187a327809466cfddf29753f6defdc3d19a0e291951b7a37 AS build

WORKDIR /src
# The workspace manifest and its members, because a crate taken by path reads the
# fields it inherits from the workspace it sits in.
COPY Cargo.toml ./
COPY crates ./crates
COPY benches ./benches
COPY w3c-suites ./w3c-suites

WORKDIR /src/w3c-suites/vc-jose-cose
# Cargo reads this when it is set, and builds on every core when it is not.
ARG CARGO_BUILD_JOBS
# `--locked`: the lockfile in this directory is what is built, or the build fails.
RUN cargo build --release --locked

# Distroless `cc`: glibc and the C runtime the binary links, with no shell and no
# package manager. It is the same Debian release as the builder.
FROM gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f

COPY --from=build /src/w3c-suites/vc-jose-cose/target/release/vc-jose-cose-suite /usr/local/bin/vc-jose-cose-suite

# Not root. The suite mounts its `tests/` directory at /tests and this process writes
# the verdict there, so whoever runs the image either runs it as the user that owns
# that directory (`docker run --user`, or `user:` in a compose file) or makes the
# directory writable. See README.md.
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/vc-jose-cose-suite"]
