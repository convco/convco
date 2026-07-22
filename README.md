# Convco

![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/convco/convco/docker.yml)
[![Crates.io](https://img.shields.io/crates/v/convco)](https://crates.io/crates/convco)

A Conventional commit cli.

Documentation: <https://convco.github.io>.

`convco` gives tools to work with [Conventional Commits][1].

It provides the following commands:

- `convco changelog`: Create a changelog file.
- `convco check`: Checks if a range of commits is following the convention.
- `convco commit`: Helps to make conventional commits.
- `convco version`: Finds out the current or next version.
- `convco config`: Prints the effective configuration or the default configuration.
- `convco completions`: Generates tab completions for shells (exists only with the feature `completions` enabled).

## Installation

Convco is built as a single binary. It does not need an additional runtime.

Download release archives from the latest [GitHub release](https://github.com/convco/convco/releases/latest):

```sh
target=x86_64-unknown-linux-musl
curl -OL "https://github.com/convco/convco/releases/latest/download/convco-${target}.tar.gz"
tar -xzf "convco-${target}.tar.gz" --strip-components=1 "convco-${target}/convco"
sudo install -m 755 convco /usr/local/bin/convco
```

For macOS or Linux with Homebrew:

```sh
brew install convco
```

With Cargo:

```sh
cargo install convco
```

With Docker:

```sh
docker run --rm -v "$PWD:/tmp" -w /tmp convco/convco --help
```

## Building from source

Rust 1.87 or newer is required.

Building with `cargo` depends on `git2` and `cmake` due to linking with `zlib-ng`.
You can disable the default backend features for a source build:

```sh
cargo build --no-default-features
```

## Configuration

`convco` follows the [conventional-changelog-config-spec][3].

The configuration is loaded in this order:

1. Load the internal defaults.
    - specified in [src/conventional/config.rs](src/conventional/config.rs),
    - see these defaults at [`convco config --default`](https://convco.github.io/configuration#default-configuration).
2. If `-c` or `--config` is provided, load that file.
3. Otherwise, load `${PWD}/.convco` when it exists.
4. Otherwise, load `${PWD}/.versionrc` for compatibility with conventional-changelog.

To get the final derived configuration run `convco config`.

When `host`, `owner` and `repository` are not supplied, convco derives them from the `origin` git remote.
Additional convco-specific config includes `commitTemplate`, description length limits, `initialBumpVersion`, and `ignoreMessagePattern`.

## Docker usage

```sh
# build the convco image
docker build -t convco .
# run it on any codebase
docker run --rm -v "$PWD:/tmp" -w /tmp convco --help
```

or use it from the Docker Hub:

```sh
docker run --rm -v "$PWD:/tmp" -w /tmp convco/convco --help
```

or use it from the GitHub Container Registry:

```sh
docker run --rm -v "$PWD:/tmp" -w /tmp ghcr.io/convco/convco --help
```

### Use it in .gitlab-ci.yml

If you've created an image and pushed it into your private registry

```yaml
convco:check:
  stage: test
  image:
    name: convco/convco:latest
  script:
    - check
```

## GitHub Actions

Use `convco check` in pull requests to validate the commits in the PR range.

```yaml
name: Pull request
on: [pull_request]

jobs:
  convco:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6
        with:
          fetch-depth: 10
      - name: Validate commit messages
        shell: bash
        run: |
          set -euo pipefail
          target="x86_64-unknown-linux-musl"
          curl -sSfL "https://github.com/convco/convco/releases/latest/download/convco-${target}.tar.gz" \
            | tar -xz --strip-components=1 "convco-${target}/convco"
          chmod +x convco
          ./convco check ${{ github.event.pull_request.base.sha }}..${{ github.event.pull_request.head.sha }}
```

## Tools

### Changelog

A changelog can be generated using the conventional commits.
It is inspired by [conventional changelog][2] and the [configuration file](#configuration) allows changes to the generated output.

```sh
convco changelog > CHANGELOG.md
```

Limit changelog commits with git pathspecs:

```sh
convco changelog --paths 'src,:(exclude)src/generated'
convco changelog --paths src --paths ':(exclude)src/generated'
```

### Check

Check a range of revisions for compliance.

It returns a non-zero exit code if some commits are not conventional.
This is useful in a pre-push hook or CI job.

```sh
convco check $remote_sha..$local_sha
convco check origin/main..HEAD
git log -1 --format=%B | convco check --from-stdin
```

### Commit

Helps to make conventional commits.
A scope, description, body, breaking change and issues will be prompted.
Convco will recover the previous message in case git failed to create the commit.

```sh
convco commit --feat
convco commit --fix --scope parser --message "handle empty input"
convco commit --feat --breaking --trailer "Reviewed-by: Z"
convco commit --interactive --patch
convco commit --feat -- -p --edit
convco commit --intent-to-add new-file.rs --patch
```

`convco commit` can also be used as git [core.editor][4].
In this case `convco commit` will not invoke `git commit`, but `git` will invoke `convco commit`

e.g.:

```sh
GIT_EDITOR='convco commit' git commit -p
```

When persisting the git editor also set [`sequence.editor`][5] when editing the todo list of an interactive rebase.

Or configure a git alias:

```sh
git config --global alias.convco '!GIT_EDITOR="convco commit" git commit'
```

### Version

When no options are given it will return the current version.
When `--bump` is provided, the next version will be printed out.
Conventional commits are used to calculate the next major, minor or patch.
If needed one can provide `--major`, `--minor` or `--patch` to overrule the convention.
SemVer is used by default. CalVer can be enabled with `versionScheme: calver`
and a `calverFormat`, or with command line flags.

```sh
convco version --bump
convco version --bump --version-scheme calver --calver-format YYYY.0M.MICRO
```

Supported CalVer calendar tokens are `YYYY`, `YY`, `0Y`, `MM`, `0M`, `WW`,
`0W`, `DD`, and `0D`. Counter and modifier tokens are `MAJOR`, `MINOR`,
`MICRO`, `PATCH` as an alias for `MICRO`, and `MODIFIER`. For example,
`YYYY.0M.MICRO`, `YY.0M.MICRO`, and `YYYY.0M.0D` are valid formats. The final
counter segment can be optional, for example `YYYY.0M(.MICRO)`, which parses
both `2026.07` and `2026.07.1` while displaying `2026.07.0` as `2026.07`.
Calendar-only formats such as `YYYY.0M.0D` can be read from tags, but `--bump`
fails if that version already exists because there is no counter to make a
second release in the same calendar period distinct.

Use `--major`, `--minor` or `--patch` to force the bump, and `--prerelease` to calculate a prerelease version:

```sh
convco version --bump --minor
convco version --bump --prerelease rc
```

Limit version calculation with git pathspecs:

```sh
convco version --bump --paths src
convco version --bump --paths ':(exclude)charts'
convco version --bump --paths 'packages/app,packages/lib'
```

It is useful to use it with release tools, such as [`cargo-release`](https://crates.io/crates/cargo-release):

```sh
cargo release $(convco version --bump)
```

### Completions

> [!NOTE]
> This subcommand requires the feature `completions` to be enabled.

Generates tab completion for the current shell

```sh
convco completions
```

If your shell cannot be detected (the `$SHELL` variable isn't present) you can specify the shell you want completions generated for.

```sh
convco completions bash
```

The tab completions will be output to stdout so you may want to write them to a file for future use. Here are some example files for given shells:

- Bash: `/usr/share/bash-completion/completions/convco`
- Zsh: `/usr/share/zsh/site-functions/_convco`
- Fish: `/usr/share/fish/vendor_completions.d/convco.fish`
- Elvish: `/usr/share/elvish/lib/convco.elv`

[1]: https://www.conventionalcommits.org/
[2]: https://github.com/conventional-changelog/conventional-changelog
[3]: https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md
[4]: https://git-scm.com/docs/git-var#Documentation/git-var.txt-GITEDITOR
[5]: https://git-scm.com/docs/git-var#Documentation/git-var.txt-GITSEQUENCEEDITOR
