# syntax = docker/dockerfile-upstream:master-labs
#-*-mode:dockerfile;indent-tabs-mode:nil;tab-width:2;coding:utf-8-*-
# vi: ft=dockerfile tabstop=2 shiftwidth=2 softtabstop=2 expandtab
# ────────────────────────────────────────────────────────────────────────────────
FROM alpine:edge
# ────────────────────────────────────────────────────────────────────────────────
SHELL ["/bin/ash", "-o", "pipefail", "-c"]
USER root
ARG NEOVIM_BUILD_DEPENDENCIES="\
  automake=1.16.4-r0 \
  autoconf=2.71-r0 \
  build-base=0.5-r2 \
  cmake=3.21.2-r0 \
  coreutils=8.32-r2 \
  curl=7.78.0-r0 \
  git=2.33.0-r0 \
  gettext-tiny-dev=0.3.2-r0 \
  libtool=2.4.6-r7 \
  pkgconf=1.8.0-r0 \
  unzip=6.0-r9 \
  "
RUN \
  IFS=' ' read -a packages <<< $NEOVIM_BUILD_DEPENDENCIES ; \
  apk add --virtual .neovim-build-deps --no-cache "${packages[@]}";
# ────────────────────────────────────────────────────────────────────────────────
USER root
WORKDIR "/usr/src/neovim"
RUN \
  git clone https://github.com/neovim/neovim.git "/usr/src/neovim" \
  && make -j"$(nproc)" CMAKE_BUILD_TYPE=RelWithDebInfo  \
  CMAKE_EXTRA_FLAGS="-DCMAKE_INSTALL_PREFIX=/usr/local"  \
  && make -j"$(nproc)" install  \
  && rm -r /usr/src/neovim \
  && apk del --no-cache --purge .neovim-build-deps
# ────────────────────────────────────────────────────────────────────────────────
USER root
ARG BASE_PACKAGES="\
  alpine-sdk=1.0-r1 \
  bash=5.1.8-r0 \
  bash-completion=2.11-r4 \
  binutils=2.35.2-r2 \
  build-base=0.5-r2 \
  ca-certificates=20191127-r5 \
  cmake=3.21.1-r0 \
  coreutils=8.32-r2 \
  curl=7.79.0-r0 \
  doxygen=1.9.1-r1 \
  findutils=4.8.0-r1 \
  fontconfig=2.13.1-r4 \
  gawk=5.1.0-r0 \
  gcc=10.3.1_git20210625-r0 \
  git=2.32.0-r2 \
  grep=3.6-r0 \
  jq=1.6-r1 \
  less=590-r0 \
  make=4.3-r0 \
  mkfontscale=1.2.1-r1 \
  ncurses=6.2_p20210109-r0 \
  ncurses-dev=6.2_p20210109-r0 \
  ncurses-static=6.2_p20210109-r0 \
  openssl=1.1.1k-r1 \
  openssl-dev=1.1.1k-r1 \
  openssl-libs-static=1.1.1k-r1 \
  perl=5.34.0-r0 \
  shadow=4.8.1-r0 \
  starship=0.56.0-r0 \
  sudo=1.9.7_p2-r0 \
  tmux=3.2a-r0 \
  tree=1.8.0-r0 \
  tzdata=2021a-r0 \
  util-linux=2.37.1-r0 \
  wget=1.21.1-r1 \
  unzip=6.0-r9 \
  xclip=0.13-r1 \
  zlib-dev=1.2.11-r3 \
  "
RUN \
  IFS=' ' read -a packages <<< $BASE_PACKAGES ; \
  ( \
  echo "http://dl-cdn.alpinelinux.org/alpine/edge/main" ; \
  echo "http://dl-cdn.alpinelinux.org/alpine/edge/community" ; \
  echo "http://dl-cdn.alpinelinux.org/alpine/edge/testing" ; \
  echo "http://dl-cdn.alpinelinux.org/alpine/v3.14/main" ; \
  echo "http://dl-cdn.alpinelinux.org/alpine/v3.14/community" ; \
  echo "http://dl-cdn.alpinelinux.org/alpine/v3.13/main" ; \
  echo "http://dl-cdn.alpinelinux.org/alpine/v3.13/community" ; \
  ) | tee /etc/apk/repositories > /dev/null  \
  && apk add --no-cache "${packages[@]}" \
  || ( \
  sed -i -e 's/dl-cdn/dl-4/g' /etc/apk/repositories \
  && apk add --no-cache "${packages[@]}" \
  )
# ────────────────────────────────────────────────────────────────────────────────
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
# ────────────────────────────────────────────────────────────────────────────────
USER root
ARG RUST_VERSION="1.54.0"
ARG RUSTUP_URL="https://sh.rustup.rs"
ENV RUSTUP_HOME="/usr/local/rustup"
ENV CARGO_HOME="/usr/local/cargo"
ENV PATH="${CARGO_HOME}/bin:${PATH}"
ENV RUST_VERSION "${RUST_VERSION}"
RUN \
  case "$(apk --print-arch)" in \
  x86_64 | aarch64) \
  true \
  ;; \
  *) \
  exit 1 \
  ;; \
  esac; \
  curl --proto '=https' --tlsv1.2 -fSsl "${RUSTUP_URL}" | bash -s -- -y \
  --no-modify-path \
  --profile default \
  --default-toolchain "${RUST_VERSION}" \
  --default-host "$(apk --print-arch)-unknown-linux-musl" \
  && chmod -R a+w "${RUSTUP_HOME}" "${CARGO_HOME}" \
  && chown "$(id -u ${USER}):$(id -g ${USER})" "${RUSTUP_HOME}" -R \
  && chown "$(id -u ${USER}):$(id -g ${USER})" "${CARGO_HOME}" -R
# ────────────────────────────────────────────────────────────────────────────────
USER root
WORKDIR "/root"
# cargo-make : fails to build on aarch64
ARG CARGO_PACKAGES="\
  cargo-watch \
  "
RUN \
  rustup default stable ; \
  IFS=' ' read -a packages <<< "${CARGO_PACKAGES}" ; \
  cargo install --locked --all-features --root /usr/local "${packages[@]}"
# ────────────────────────────────────────────────────────────────────────────────
USER root
ARG USER=devel
ENV USER "${USER}"
ARG UID="1000"
ENV UID $UID
ENV SHELL="/bin/bash"
ENV HOME="/home/${USER}"
RUN \
  useradd \
  --no-log-init \
  --create-home \
  --user-group \
  --home-dir "/home/${USER}" \
  --uid "${UID}" \
  --shell "/bin/bash" \
  --password \
  $(perl -e 'print crypt($ARGV[0], "password")' "${USER}_${UID}" 2>/dev/null) \
  "${USER}"
RUN \
  getent group sudo > /dev/null || addgroup sudo > /dev/null 2>&1 \
  && sed -i \
    -e '/%sudo\s\+ALL=(ALL\(:ALL\)\?)\s\+ALL/d' \
    -e '/%sudo.*NOPASSWD:ALL/d' \
  /etc/sudoers \
  && echo '%sudo ALL=(ALL) ALL' >> /etc/sudoers \
  && echo '%sudo ALL=(ALL) NOPASSWD: ALL' >> /etc/sudoers \
  && usermod -aG wheel,root,sudo "${USER}"
# ────────────────────────────────────────────────────────────────────────────────
USER "${USER}"
RUN \
  echo '[ -r /etc/profile ] && . /etc/profile' > "${HOME}/.bashrc"
# ────────────────────────────────────────────────────────────────────────────────
USER root
ENV EDITOR="nvim"
ENV VISUAL="nvim"
ENV PATH="${PATH}:/usr/local/bin"
WORKDIR "/etc/profile.d"
RUN \
  ( \
  echo 'export PATH="${PATH}:/usr/local/bin"' ; \
  echo "export EDITOR=\"${EDITOR}\"" ; \
  echo "export VISUAL=\"${VISUAL}\"" ; \
  echo 'npm --version >/dev/null 2>&1 && export PATH="${PATH}:$(npm -g bin)" ;' ; \
  echo 'yarn --version >/dev/null 2>&1 && export PATH="${PATH}:$(yarn global bin)" ;' ; \
  echo 'starship --version >/dev/null 2>&1 && eval "$(starship init bash)" ;' ; \
  echo 'rustup --version >/dev/null 2>&1 && eval "$(rustup completions bash rustup)" && eval "$(rustup completions bash cargo)";' ; \
  echo 'curl --version >/dev/null 2>&1 && source <(curl -sSL git.io/forgit) ;' ; \
  ) | tee 00-env.sh > /dev/null \
  && ( \
  echo "alias apk='sudo apk'" ; \
  echo "alias g='git'" ; \
  ) | tee 01-alias.sh > /dev/null
# ────────────────────────────────────────────────────────────────────────────────
USER root
ARG IMAGE_SPECIFIC_PACKAGES="\
  aria2=1.35.0-r2 \
  bzip2=1.0.8-r1 \
  docker=20.10.8-r0 \
  docker-compose=1.29.2-r0 \
  fzf=0.27.2-r1 \
  fzf-bash-completion=0.27.2-r1 \
  git-secret=0.3.2-r0 \
  glow=1.3.0-r0 \
  gnupg=2.2.29-r0 \
  gtest-dev=1.11.0-r0 \
  htop=3.0.5-r2 \
  libcap=2.52-r0 \
  libffi-dev=3.3-r2 \
  lua5.4=5.4.3-r0 \
  musl-dev=1.2.2-r5 \
  nerd-fonts=2.1.0-r6 \
  ripgrep=13.0.0-r0 \
  ripgrep-bash-completion=13.0.0-r0 \
  rng-tools-extra=6.12-r0 \
  "
RUN \
  IFS=' ' read -a packages <<< $IMAGE_SPECIFIC_PACKAGES ; \
  apk add --no-cache "${packages[@]}" ) \
  && usermod -aG docker "${USER}"
# ────────────────────────────────────────────────────────────────────────────────
USER root
RUN  \
  find "${HOME}" \
    -not -group "$(id -g ${USER})" \
    -not -user "$(id -u ${USER})" \
    -print \
  | xargs -I {} -P `nproc` --no-run-if-empty \
  chown --no-dereference "$(id -u ${USER}):$(id -g ${USER})" {} \
  && find "/usr/local/cargo" \
    -not -group "$(id -g ${USER})" \
    -not -user "$(id -u ${USER})" \
    -print \
  | xargs -I {} -P `nproc` --no-run-if-empty \
  chown --no-dereference "$(id -u ${USER}):$(id -g ${USER})" {} \
# ────────────────────────────────────────────────────────────────────────────────
USER "${USER}"
ENV OPENSSL_STATIC=yes
ENV OPENSSL_LIB_DIR="/usr/lib"
ENV OPENSSL_INCLUDE_DIR="/usr/include"
RUN \
  rustup --version \
  && cargo --version \
  && rustc --version \
  && rustup toolchain install "nightly-$(apk --print-arch)-unknown-linux-musl" \
  && rustup default "stable-$(apk --print-arch)-unknown-linux-musl" \
  && rustup component add rust-src rustfmt rls clippy
# ────────────────────────────────────────────────────────────────────────────────
COPY <<-"EOT" /usr/local/cargo/config
[target.x86_64-unknown-linux-musl]
  rustflags = ["-C", "target-feature=+crt-static"]
[target.aarch64-unknown-linux-musl]
  rustflags = ["-C", "target-feature=+crt-static"]
EOT
# ────────────────────────────────────────────────────────────────────────────────
USER "${USER}"
ARG WORKDIR="/workspace"
ENV WORKDIR "${WORKDIR}"
WORKDIR "${WORKDIR}"
ENTRYPOINT [ "/bin/bash" ]
