#!/bin/sh
set -e

main() {
  # Use UTC time for everything.
  export TZ=UTC0
  # Set minimal locale.
  export LC_ALL=C
  # Set source date. This is honored by `asciidoctor` and other tools.
  export SOURCE_DATE_EPOCH=$(git log -1 --pretty=%ct)

  if ! command -v rad > /dev/null; then
    echo "fatal: rad is not installed" ; exit 1
  fi

  if ! command -v podman > /dev/null; then
    echo "fatal: podman is not installed" ; exit 1
  fi

  if ! command -v sha256sum > /dev/null; then
    echo "fatal: sha256sum is not installed" ; exit 1
  fi

  rev="$(git rev-parse --short HEAD)"
  tempdir="$(mktemp -d)"
  gitarchive="$tempdir/heartwood-$rev.tar.gz"
  keypath="$(rad path)/keys/radicle.pub"
  version="$(build/version.sh)"
  image=radicle-build-$version

  if [ ! -f "$keypath" ]; then
    echo "fatal: no key found at $keypath" ; exit 1
  fi
  # Authenticate user for signing
  rad auth

  echo "Building Radicle $version.."
  echo "Creating archive of repository at $rev in $gitarchive.."
  git archive --format tar.gz -o "$gitarchive" HEAD

  echo "Building image ($image).."
  podman --cgroup-manager=cgroupfs build \
    --env SOURCE_DATE_EPOCH \
    --env GIT_COMMIT_TIME=$SOURCE_DATE_EPOCH \
    --env GIT_HEAD=$rev \
    --env RADICLE_VERSION=$version \
    --arch amd64 --tag $image -f ./build/Dockerfile - < $gitarchive

  echo "Creating container (radicle-build-container).."
  podman --cgroup-manager=cgroupfs create --replace --name radicle-build-container $image

  # Copy build artifacts to output folder.
  outdir=build/artifacts/
  mkdir -p $outdir
  podman cp --overwrite radicle-build-container:/builds/. $outdir

  for target in $(cat build/targets); do
    echo "Signing artifacts for $target.."

    filename="radicle-$version-$target.tar.xz"
    filepath="$outdir/$filename"

    # Output SHA256 digest of archive.
    checksum="$(cd $outdir && sha256sum $filename)"
    echo "Checksum of $filepath is $(echo "$checksum" | cut -d' ' -f1)"
    echo "$checksum" > $filepath.sha256

    # Sign archive and verify archive.
    rm -f $filepath.sig # Delete existing signature
    ssh-keygen -Y sign -n file -f $keypath $filepath
    ssh-keygen -Y check-novalidate -n file -s $filepath.sig < $filepath
  done

  # Remove build artifacts that aren't needed anymore.
  rm -f $gitarchive
  podman rm radicle-build-container > /dev/null
  podman rmi --ignore localhost/$image
}

# Run build.
echo "Running build.."
main "$@"

# Show artifact checksums.
echo
build/checksums.sh
echo

echo "Build ran successfully."
