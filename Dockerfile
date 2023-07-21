# syntax = docker/dockerfile-upstream:master-labs
#-*-mode:dockerfile;indent-tabs-mode:nil;tab-width:2;coding:utf-8-*-
# vi: ft=dockerfile tabstop=2 shiftwidth=2 softtabstop=2 expandtab:
FROM --platform=$BUILDPLATFORM messense/cargo-zigbuild AS base
ARG TARGETARCH
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
FROM base AS builder-layer
RUN rustup --version \
  && cargo --version \
  && rustc --version; \
  case "${TARGETARCH}" in \
     aarch64|arm64) \
       target_arch='aarch64'; \
       ;; \
     amd64|x86_64) \
       target_arch='x86_64'; \
       ;; \
     *) \
       echo "Unsupported arch: ${TARGETARCH}"; \
       exit 1; \
       ;; \
  esac ; \
  rustup target add "$target_arch-unknown-linux-musl"
COPY <<-"EOT" /usr/local/cargo/config
[target.x86_64-unknown-linux-musl]
  rustflags = ["-C", "target-feature=+crt-static"]
[target.aarch64-unknown-linux-musl]
  rustflags = ["-C", "target-feature=+crt-static"]
EOT
WORKDIR "/workspace"
COPY . /workspace
RUN \
  --mount=type=cache,target=/root/.cargo \
  --mount=type=cache,target=/usr/local/cargo/registry \
    case "${TARGETARCH}" in \
     aarch64|arm64) \
       target_arch='aarch64'; \
       export CFLAGS="-mno-outline-atomics"; \
       ;; \
     amd64|x86_64) \
       target_arch='x86_64'; \
       ;; \
     *) \
       echo "Unsupported arch: ${TARGETARCH}"; \
       exit 1; \
       ;; \
  esac ; \
  cargo zigbuild \
    --release \
    --no-default-features \
    --jobs "$(nproc)" \
    --target "$target_arch-unknown-linux-musl" \
  && if [[ ! -z $(readelf -d "/workspace/target/$target_arch-unknown-linux-musl/release/convco" | grep NEED) ]]; then \
    if ldd "/workspace/target/$target_arch-unknown-linux-musl/release/convco" > /dev/null 2>&1; then \
      echo >&2 "*** '/workspace/target/$target_arch-unknown-linux-musl/release/convco' was not linked statically" ; \
      exit 1 ; \
    fi \
  fi \
  && mv "/workspace/target/$target_arch-unknown-linux-musl/release/convco" /convco
FROM alpine:latest
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
COPY --chmod=0755 --from=builder-layer /convco /entrypoint
WORKDIR /workspace
VOLUME /workspace
ENTRYPOINT [ "/entrypoint" ]
CMD [ "check" ]
