# Convco

![GitHub Workflow Status](https://img.shields.io/github/workflow/status/hdevalke/convco/Build%20binary)
[![Crates.io](https://img.shields.io/crates/v/convco)](https://crates.io/crates/convco)

A Conventional commit cli.

`convco` gives tools to work with [Conventional Commits][1].

The tool is still in early development.
It provides already the following commands:

- `convco changelog`: Create a changelog file.
- `convco check`: Checks if a range of commits is following the convention.
- `convco commit`: Helps to make conventional commits.
- `convco version`: Finds out the current or next version.

## Installation

`cargo install convco`

## Tools

### Changelog

A changelog can be generated using the conventional commits.
It is inspired by [conventional changelog][2].
Configuration follows the [conventional-changelog-config-spec][3]

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

#### TODO

- [x] automatic notes for breaking changes
- [x] custom template folder
- [x] use a `.versionrc` file
- [x] limit to a range of versions
- [x] sort sections in changelog
- [x] issue references
- [ ] better documentation
- [ ] better error handling

[1]: https://www.conventionalcommits.org/
[2]: https://github.com/conventional-changelog/conventional-changelog
[3]: https://github.com/conventional-changelog/conventional-changelog-config-spec/blob/master/versions/2.1.0/README.md
