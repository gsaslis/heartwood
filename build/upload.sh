#!/bin/sh
set -e

SSH_LOGIN=${SSH_LOGIN:-release}
SSH_ADDRESS=${SSH_ADDRESS:-$SSH_LOGIN@files.radicle.xyz}
SSH_KEY="$(rad path)/keys/radicle"

main() {
  version="$(build/version.sh)"

  echo "Uploading Radicle $version..."

  if [ -z "$version" ]; then
    echo "fatal: empty version number" >&2 ; exit 1
  fi

  # Create remote folder.
  ssh -i $SSH_KEY $SSH_ADDRESS mkdir -p /mnt/radicle/files/releases/$version
  # Copy files over.
  scp -i $SSH_KEY build/artifacts/radicle-$version* $SSH_ADDRESS:/mnt/radicle/files/releases/$version
  scp -i $SSH_KEY build/artifacts/radicle.json $SSH_ADDRESS:/mnt/radicle/files/releases/$version

  for target in $(cat build/targets); do
    archive=/mnt/radicle/files/releases/$version/radicle-$version-$target.tar.xz
    symlink=/mnt/radicle/files/releases/$version/radicle-$target.tar.xz

    echo "Creating symlinks for $target.."

    ssh -i $SSH_KEY $SSH_ADDRESS ln -snf $archive $symlink
    ssh -i $SSH_KEY $SSH_ADDRESS ln -snf $archive.sig $symlink.sig
    ssh -i $SSH_KEY $SSH_ADDRESS ln -snf $archive.sha256 $symlink.sha256
  done

  echo "Creating 'latest' symlink.."
  ssh -i $SSH_KEY $SSH_ADDRESS ln -snf /mnt/radicle/files/releases/$version /mnt/radicle/files/releases/latest
  echo "Done."
}

main "$@"
