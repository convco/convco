# syntax = docker/dockerfile-upstream:master-labs
#-*-mode:dockerfile;indent-tabs-mode:nil;tab-width:2;coding:utf-8-*-
# vi: ft=dockerfile tabstop=2 shiftwidth=2 softtabstop=2 expandtab:
FROM alpine:3.14 AS upx-downloader
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
RUN \
  apk add --no-cache \
    curl \
    jq \
    xz ;
ARG REPO="upx/upx"
ARG LATEST_RELEASE_ENDPOINT="https://api.github.com/repos/${REPO}/releases/latest"
RUN \
  tag_name="$(curl -sL ${LATEST_RELEASE_ENDPOINT} | jq -r '.tag_name')"; \
  architecture="$(apk --print-arch)"; \
  case "$architecture" in \
    x86_64|amd64) \
      architecture="amd64" \
    ;; \
    aarch64) \
      architecture="arm64" \
    ;; \
    *) \
      echo >&2 "[ WARN ] compression utilities are not available: $architecture"; \
      exit 0 \
    ;; \
  esac ; \
  version="$(echo ${tag_name} | sed 's/v//g')"; \
  download_url="https://github.com/upx/upx/releases/download/${tag_name}/upx-${version}-${architecture}_linux.tar.xz"; \
  rm -rf \
    /tmp/{upx.tar,upx.tar.xz} \
    /usr/local/bin/upx \
  && echo "$download_url" > /tmp/dl \
  && curl -fsSLo /tmp/upx.tar.xz "${download_url}" \
  && xz -d -c /tmp/upx.tar.xz \
  | tar \
    -xOf - upx-${version}-${architecture}_linux/upx > /upx
FROM alpine:3.14 AS base
# ────────────────────────────────────────────────────────────────────────────────
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
# ────────────────────────────────────────────────────────────────────────────────
RUN \
  apk add --no-cache \
  build-base \
  cmake \
  curl \
  libgit2-static \
  musl-dev \
  openssl-dev \
  openssl-libs-static
# ────────────────────────────────────────────────────────────────────────────────
FROM base AS builder-layer
ARG RUST_VERSION="1.54.0"
ARG RUSTUP_URL="https://sh.rustup.rs"
ENV RUSTUP_HOME="/usr/local/rustup"
ENV CARGO_HOME="/usr/local/cargo"
ENV PATH="${CARGO_HOME}/bin:${PATH}"
ENV RUST_VERSION "${RUST_VERSION}"
RUN \
  case "$(apk --print-arch)" in \
    x86_64 | aarch64 ) \
      true \
    ;; \
    *) \
    exit 1 \
    ;; \
  esac; \
  curl --proto '=https' --tlsv1.2 -fSsl "${RUSTUP_URL}" | sh -s -- -y \
  --no-modify-path \
  --profile minimal \
  --default-toolchain "${RUST_VERSION}" \
  --default-host "$(apk --print-arch)-unknown-linux-musl" \
  && chmod -R a+w "${RUSTUP_HOME}" "${CARGO_HOME}" \
  && rustup --version \
  && cargo --version \
  && rustc --version \
  && rustup toolchain install "stable-$(apk --print-arch)-unknown-linux-musl"
# ────────────────────────────────────────────────────────────────────────────────
COPY <<-"EOT" /usr/local/cargo/config
[target.x86_64-unknown-linux-musl]
  rustflags = ["-C", "target-feature=+crt-static"]
[target.aarch64-unknown-linux-musl]
  rustflags = ["-C", "target-feature=+crt-static"]
EOT
# ────────────────────────────────────────────────────────────────────────────────
ENV OPENSSL_STATIC=yes
ENV OPENSSL_LIB_DIR="/usr/lib"
ENV OPENSSL_INCLUDE_DIR="/usr/include"
WORKDIR "/workspace"
COPY . /workspace
RUN \
  --mount=type=cache,target=/root/.cargo \
  --mount=type=cache,target=/usr/local/cargo/registry \
  [ "$(apk --print-arch)" == "aarch64" ] && export CFLAGS="-mno-outline-atomics" ; \
  rustup run stable cargo build \
    --release \
    --jobs "$(nproc)" \
    --target "$(apk --print-arch)-unknown-linux-musl" \
  && if [[ ! -z $(readelf -d "/workspace/target/$(apk --print-arch)-unknown-linux-musl/release/convco" | grep NEED) ]]; then \
    if ldd "/workspace/target/$(apk --print-arch)-unknown-linux-musl/release/convco" > /dev/null 2>&1; then \
      echo >&2 "*** '/workspace/target/$(apk --print-arch)-unknown-linux-musl/release/convco' was not linked statically" ; \
      exit 1 ; \
    fi \
  fi \
  && mv "/workspace/target/$(apk --print-arch)-unknown-linux-musl/release/convco" /convco
FROM base AS compression-layer
COPY --chmod=0755 --from=upx-downloader /upx /usr/local/bin/upx
RUN \
  upx --version \
WORKDIR /workspace
COPY --chmod=0755 --from=builder-layer /convco /workspace/convco
RUN \
  /workspace/convco --version \
  && strip /workspace/convco \
  && upx /workspace/convco \
  && /workspace/convco --version
FROM alpine:3.14
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
COPY --chmod=0755 --from=compression-layer /workspace/convco /entrypoint
WORKDIR /workspace
VOLUME /workspace
ENTRYPOINT [ "/entrypoint" ]
CMD [ "check" ]
