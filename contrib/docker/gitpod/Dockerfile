# syntax = docker/dockerfile-upstream:master-labs
#-*-mode:dockerfile;indent-tabs-mode:nil;tab-width:2;coding:utf-8-*-
# vi: ft=dockerfile tabstop=2 shiftwidth=2 softtabstop=2 expandtab:
# ─── IMAGE DEPENDENCY ALIASES ───────────────────────────────────────────────────
FROM gitpod/workspace-full:build-branch-master
# ────────────────────────────────────────────────────────────────────────────────
USER root
WORKDIR "/root"
SHELL ["/bin/bash", "-o", "pipefail", "-c"]
# ────────────────────────────────────────────────────────────────────────────────
ENV DEBIAN_FRONTEND=noninteractive
# hadolint ignore=SC2016,DL3009
RUN \
  add-apt-repository -y ppa:neovim-ppa/unstable > /dev/null 2>&1 \
  && apt-get update -qq > /dev/null 2>&1 \
  && curl -fsSl https://i.jpillora.com/chisel! | bash > /dev/null 2>&1 \
  && curl -fsSL https://starship.rs/install.sh | bash -s -- --force > /dev/null 2>&1 \
  && install-packages \
  tmux \
  dropbear \
  cmake \
  jq \
  ripgrep \
  curl \
  neovim \
  tree \
  fzf \
  xfonts-utils \
  libssl-dev \
  fonts-symbola \
  xsel \
  xauth \
  xclip
# ────────────────────────────────────────────────────────────────────────────────
USER root
ENV EDITOR="nvim"
ENV VISUAL="nvim"
ENV PATH="${PATH}:/usr/local/bin"
WORKDIR "/etc/profile.d"
# hadolint ignore=DL3009,SC2038,SC2016
RUN \
  [ -d "${HOME}/.bashrc.d" ] && while read i; do ln -sf "$i" "/etc/profile.d/$(basename $i).sh"; done < <(find ${HOME}/.bashrc.d -type f ) || true ; \
  curl \
  -fsSl https://raw.githubusercontent.com/3hhh/fzfuncs/master/bashrc_fzf \
  -o /etc/profile.d/bashrc_fzf.sh \
  && ( \
  echo "export EDITOR=\"${EDITOR}\"" ; \
  echo "export VISUAL=\"${VISUAL}\"" ; \
  echo 'export DOCKER_BUILDKIT="0"' ; \
  echo 'export BUILDKIT_PROGRESS="auto"' ; \
  echo 'export COMPOSE_DOCKER_CLI_BUILD="0"' ; \
  echo 'export GEM_HOME=/workspace/.rvm' ; \
  echo 'export GEM_PATH=$GEM_HOME:$GEM_PATH' ; \
  echo 'export GOROOT="${HOME}/go" ;' ; \
  echo 'export GOPATH="/workspace/go-packages" ;' ; \
  echo 'export PYTHONUSERBASE=/workspace/.pip-modules ;' ; \
  echo 'export PATH="${PATH}:/usr/local/bin"' ; \
  echo '[ -d ${GEM_HOME} ] && export PATH="${GEM_HOME}/bin:$PATH" ;' ; \
  echo '[ -d ${GOROOT} ] && export PATH="${GO_ROOT}/bin:$PATH" ;' ; \
  echo '[ -d ${GOPATH} ] && export PATH="${GOPATH}/bin:$PATH" ;' ; \
  echo '[ -d ${HOME}/.local/bin ] && export PATH="${HOME}/.local/bin:${PATH}"' ; \
  echo '[ -d ${HOME}/.cargo/bin ] && export PATH="${HOME}/.cargo/bin:${PATH}";';  \
  echo '[ -d ${PYTHONUSERBASE} ] && export PATH="${PYTHONUSERBASE}/bin:$PATH" ;' ; \
  echo '[ -d ${HOME}/.pyenv/bin ] && export PATH="${HOME}/.pyenv/bin:${PATH}";' ; \
  echo '[ -d ${HOME}/.pyenv/shims ] && export PATH="${HOME}/.pyenv/shims:${PATH}" ;' ; \
  echo '[ -d ${HOME}/bin ] && export PATH="${HOME}/bin":${PATH}' ; \
  echo 'npm --version >/dev/null 2>&1 && export PATH="$(npm -g bin):${PATH}" ;' ; \
  echo 'yarn --version >/dev/null 2>&1 && export PATH="$(yarn global bin):${PATH}" ;' ; \
  echo 'gp --help >/dev/null 2>&1 && (eval $(gp env | sed -e "s/\(=[[:blank:]]*\)\(.*\)/\1\"\2\" ;/" -e "s/^/export /g") ; source <(gp completion) ;)' ; \
  echo 'starship --version >/dev/null 2>&1 && eval "$(starship init bash)" ;' ; \
  echo 'just --version >/dev/null 2>&1 && eval "$(just --completions bash)" ;' ; \
  echo 'rustup --version >/dev/null 2>&1 && eval "$(rustup completions bash rustup)" && eval "$(rustup completions bash cargo)";' ; \
  echo 'curl --version >/dev/null 2>&1 && source <(curl -sSL git.io/forgit) ;' ; \
  ) | tee 00-env.sh > /dev/null \
  && ( \
  echo "alias apt='sudo apt'" ; \
  echo "alias g='git'" ; \
  echo 'exa --version >/dev/null 2>&1 && alias la="exa -A";alias ll="exa -alhF";alias ls="exa";' ; \
  echo 'bat --version >/dev/null 2>&1 && alias cat="bat -pp";' ; \
  echo 'just --version >/dev/null 2>&1 && alias j="just";' ; \
  ) | tee 01-alias.sh > /dev/null \
  && [ -r /etc/bash.bashrc ] && (\
  echo 'while read i; do source "$i"; done < <(find /etc/profile.d/ -name "*.sh") ;' ; \
  ) | tee -a /etc/bash.bashrc
# ────────────────────────────────────────────────────────────────────────────────
USER gitpod
WORKDIR "${HOME}"
# hadolint ignore=SC2016
RUN \
  [ -r "${HOME}/.bashrc.d/60-python" ] \
  && sed -i -e 's/pyenv/${HOME}\/.pyenv\/bin\/pyenv/g'  "${HOME}/.bashrc.d/60-python" \
  && sed -i \
  -e '/\$HOME\/\.bashrc\.d/d' \
  -e '/PIP/d' \
  -e '/PYTHONUSERBASE/d' \
  -e '/gp completion/d' \
  -e '/\.cargo\/env/d' \
  -e '/rvm/d' \
  -e '/GEM/d' \
  "${HOME}/.bashrc" \
  && ( \
  echo "[ -r '\${HOME}/.bashrc' ] && sed -i \\" ; \
  echo "-e '/\\\$HOME\/\.bashrc\.d/d' \\" ; \
  echo "-e '/cargo/d' \\" ; \
  echo "-e '/PIP/d' \\" ; \
  echo "-e '/PYTHONUSERBASE/d' \\" ; \
  echo "-e '/gp completion/d' \\" ; \
  echo "-e '/rvm/d' \\" ; \
  echo "-e '/GEM/d' \\" ; \
  echo "'\${HOME}/.bashrc' \\" ; \
  echo '&& . ${HOME}/.bashrc' ; \
  ) | tee "${HOME}/.profile" > /dev/null ;
# ────────────────────────────────────────────────────────────────────────────────
USER "root"
WORKDIR "/root"
# TODO address this linter issue as this is a temporary fix.
# hadolint ignore=SC2046
RUN \
  curl -fsSl "https://bootstrap.pypa.io/get-pip.p"y | python3 \
  && python2 --version > /dev/null 2>&1 \
  && curl \
  -fsSl https://bootstrap.pypa.io/pip/$(python2 --version 2>&1 | gawk 'BEGIN{FS=OFS="."}{gsub("[^[:digit:].]*","");print $1,$2}')/get-pip.py | python2 \
  || true ;
# ────────────────────────────────────────────────────────────────────────────────
USER gitpod
WORKDIR "${HOME}"
# ENV PIP_USER="false"
ENV PATH="${PATH}:${HOME}/bin"
RUN \
  curl -fSsl https://pre-commit.com/install-local.py | $(pyenv which python) - \
  || curl -fSsl https://pre-commit.com/install-local.py | python3 -
# ────────────────────────────────────────────────────────────────────────────────
USER root
ARG NODEJS_PACKAGES="\
  bash-language-server \
  dockerfile-language-server-nodejs \
  standard-readme-spec \
  "
# [ NOTE ] neovim could not detect 'neovim' NodeJS package
# installed through Yarn.
# hadolint ignore=DL3016,SC2162
RUN \
  IFS=' ' read -a NODEJS_PACKAGES <<< "$NODEJS_PACKAGES" ; \
  npm install -g "${NODEJS_PACKAGES[@]}"
# ────────────────────────────────────────────────────────────────────────────────
USER gitpod
ARG CARGO_PACKAGES="\
  just \
  convco \
  jsonfmt \
  "
# hadolint ignore=SC2162
RUN \
  IFS=' ' read -a packages <<< "${CARGO_PACKAGES}" ; \
  rustup run stable --install cargo install \
  --locked \
  --all-features \
  --jobs "$(nproc)" \
  "${packages[@]}"
# ────────────────────────────────────────────────────────────────────────────────
USER root
ARG APT_PACKAGES="\
  pandoc \
  librsvg2-bin \
  ncdu \
  pdftk \
  poppler-utils \
  texmaker \
  texlive-xetex \
  texlive-fonts-recommended \
  texlive-fonts-extra \
  texlive-latex-extra \
  ttf-mscorefonts-installer \
  lmodern \
  "
RUN install-packages ${APT_PACKAGES}
# ────────────────────────────────────────────────────────────────────────────────
USER root
# TODO address this linter issue as this is a temporary fix.
# hadolint ignore=SC2038
RUN \
  find "${HOME}" \
  -not -group "$(id -g gitpod)" \
  -not -user "$(id -u gitpod)" \
  -print \
  | xargs -I {} -P "$(nproc)" --no-run-if-empty \
  chown --no-dereference "$(id -u gitpod):$(id -g gitpod)" {} ; \
  [ -d /workspace ] && chown "$(id -u gitpod):$(id -g gitpod)" "/workspace" -R ; \
  yarn cache clean --all \
  && npm -g cache clean --force > /dev/null 2>&1 \
  && rm -rf \
  /tmp/*
# ────────────────────────────────────────────────────────────────────────────────
USER gitpod
WORKDIR /workspace
