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

## Installation

`cargo install convco`

## Building from source

Rust 1.60 or newer is required.

Building with `cargo` depends on `git2` and `cmake` due to linking with `zlib-ng`.
You can optionally disable this by changing the defaults for a build:
```console
$ cargo build --no-default-features
```

## Configuration

`convco` uses follows the [conventional-changelog-config-spec][3].

The configuration file is loaded in the following order

1. Load the internal defaults
    - specified in [src/conventional/config.rs](src/conventional/config.rs),
    - see these defaults at https://convco.github.io/configuration.
2. Then override with values from the command line, `convco -c|--config path/to/.versionrc`
3. Or, if not specified via `-c|--config`, load `${PWD}/.versionrc` if it exists.

To get the final derived configuration run `convco config`.

The `host: ...`, `owner: ...` and `repository: ...` when not supplied via custom or the `.versionrc` are loaded
from the `git remote origin` value.

## Docker usage

```shell script
# build the convco image
docker build -t convco .
# run it on any codebase
docker run -v "$PWD:/tmp" --workdir /tmp --rm convco
```

or use it from the Docker Hub:

```shell script
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

```sh
# commit a new feature and then run git commit with the interactive patch switch
convco commit --feat -- --patch
```

### Version

When no options are given it will return the current version.
When `--bump` is provided, the next version will be printed out.
Conventional commits are used to calculate the next major, minor or patch.
If needed one can provide `--major`, `--minor` or `--patch` to overrule the convention.

```sh
convco version --bump
```

It is useful to use it with release tools, such as [`cargo-release`](https://crates.io/crates/cargo-release):

```sh
cargo release $(convco version --bump)
```

[1]: https://www.conventionalcommits.org/
[2]: https://github.com/conventional-changelog/conventional-changelog
[3]: https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md
