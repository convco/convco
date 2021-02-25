# Releasing Convco

Run `cargo release $(convco version --bump)`.
This will publish the new version to [crates.io] and push the new commit and tag to the remote origin.

The docker [build](https://hub.docker.com/r/convco/convco/builds) will start automatically.
A build on github actions will be to be triggered.
Download the artifacts and create a new release from the latest tag.

[crates.io]: https://crates.io
