# Convco

Conventional commit cli

`convco` gives tools to work with [Conventional Commits](https://www.conventionalcommits.org/). 

- `convco check`: Checks if a range of commits is following the convention.
- `convco version`: Finds out the current or next version.
- `convco changelog`: Create a changelog file.

## Installation

`cargo install convco`

## Tools

### Check

Check a range of revisions for compliance.

It returns a non zero exit code if some commits are not conventional.
This is useful in a pre-push hook.

```sh
convco check $remote_sha..$local_sha
```

### Version

When no options are given it will return the current version.
When `--bump` is provided, the next version will be printed out.
Conventional commits are used to calculate the next major, minor or patch.
If needed one can provide `--major`, `--minor` or `--patch` to overrule the convention.

```sh
convco version --bump
```

### Changelog

A changelog can be generated using the conventional commits.
For now it just prints out the features and fixes for each version.

```sh
convco changelog > CHANGELOG.md
```

#### TODO

- [ ] automatic notes for breaking changes
- [ ] custom template folder
- [ ] use a `.versionrc` file
- [ ] limit to a range of versions