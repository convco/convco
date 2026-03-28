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
- `convco completions`: Generates tab completions for shells (exists only with the feature `completions` enabled).

## Installation

`cargo install convco`

## Building from source

Rust 1.60 or newer is required.

Building with `cargo` depends on `git2` and `cmake` due to linking with `zlib-ng`.
You can optionally disable this by changing the defaults for a build:

```sh
cargo build --no-default-features
```

## Configuration

`convco` uses follows the [conventional-changelog-config-spec][3].

The configuration file is loaded in the following order

1. Load the internal defaults
    - specified in [src/conventional/config.rs](src/conventional/config.rs),
    - see these defaults at [`convco config --default`](https://convco.github.io/configuration#default-configuration).
2. Then override with values from the command line, `convco -c|--config path/to/.convco`
3. Or, if not specified via `-c|--config`, load `${PWD}/.convco` if it exists (or `${PWD}/.versionrc` for compatibility with conventional-changelog).

To get the final derived configuration run `convco config`.

The `host: ...`, `owner: ...` and `repository: ...` when not supplied via custom or the `.versionrc` are loaded
from the `git remote origin` value.

## Docker usage

```sh
# build the convco image
docker build -t convco .
# run it on any codebase
docker run -v "$PWD:/tmp" --workdir /tmp --rm convco
```

or use it from the Docker Hub:

```sh
docker run -v "$PWD:/tmp" --workdir /tmp --rm convco/convco
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

## Tools

### Changelog

A changelog can be generated using the conventional commits.
It is inspired by [conventional changelog][2] and the [configuration file](#configuration) allows changes to the generated the output.

```sh
convco changelog > CHANGELOG.md
```

To ignore commits that only touch certain paths, use `--ignore-path` (repeatable):

```sh
convco changelog --ignore-path docs --ignore-path .github > CHANGELOG.md
```

### Check

Check a range of revisions for compliance.

It returns a non zero exit code if some commits are not conventional.
This is useful in a pre-push hook.

```sh
convco check $remote_sha..$local_sha
```

### Commit

Helps to make conventional commits.
A scope, description, body, breaking change and issues will be prompted.
Convco will recover the previous message in case git failed to create the commit.

```sh
convco commit --feat
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

```sh
convco version --bump
```

You can ignore commits that only touch certain paths (e.g. docs or CI config):

```sh
convco version --bump --ignore-path docs --ignore-path .github
```

The equivalent configuration key is `ignore_paths`.

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

The tab completions will be outputed to the stdout so you may want to output them to a certain file to save them for future use. Here are some example files for given shells:

- Bash: `/usr/share/bash-completion/completions/convco`
- Zsh: `/usr/share/zsh/site-functions/_convco`
- Fish: `/usr/share/fish/vendor_completions.d/convco.fish`
- Elvish: `/usr/share/elvish/lib/convco.elv`

[1]: https://www.conventionalcommits.org/
[2]: https://github.com/conventional-changelog/conventional-changelog
[3]: https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md
[4]: https://git-scm.com/docs/git-var#Documentation/git-var.txt-GITEDITOR
[5]: https://git-scm.com/docs/git-var#Documentation/git-var.txt-GITSEQUENCEEDITOR
