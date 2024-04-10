# Builds

This build pipeline is designed to be run on a developer's linux machine.
The output is a set of binaries for the supported platforms, stored as
`.tar.xz` files and signed by the developer's Radicle key.

These binaries are statically linked to be maximally portable, and designed to
be reproducible, byte for byte.

To run the build, simply enter the following command from the repository root:

    build/build.sh

This will build all targets and place the output in `build/artifacts` with
one sub-directory per build target.

Note that it will use `git describe` to get a version number for the build.
You *must* have a commit tagged with a version in your history or the build
will fail, eg. `v1.0.0`.

A script is included in `build/checksums.sh` to output the SHA-256 checksums
of all the archives built.

## Requirements

The following software is required for the build:

  * `podman`
  * `rad` (The Radicle CLI)
  * `xz` (`xz-utils` package on Debian)
  * `sha256sum`

## macOS

macOS binaries are not signed or notarized, so they have to be downloaded via
the CLI to avoid issues. A copy of a small subset of the Apple SDK is included
here to be able to cross-compile.

## Podman

We use `podman` to make the build reproducible on any machine by controlling
the build environment. We prefer `podman` to `docker` because it doesn't
require a background process to run and can be run without root access out of
the box.

The first time you run `podman`, you may have to give yourself some extra UIDs
for `podman` to use, with:

    sudo usermod --add-subuids 100000-165535 --add-subgids 100000-165535 $USER

Then update `podman` with:

    podman system migrate
