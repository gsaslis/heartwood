#!/bin/sh
set -e
echo "Running build.."

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

  if ! command -v xz > /dev/null; then
    echo "fatal: xz is not installed" ; exit 1
  fi

  if ! command -v sed > /dev/null; then
    echo "fatal: sed is not installed" ; exit 1
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

  if ! version="$(git describe --match='v*' --candidates=1 2>/dev/null)"; then
    echo "fatal: no version tag found by 'git describe'" ; exit 1
  fi
  # Remove `v` prefix from version.
  version=${version#v}

  if [ ! -f "$keypath" ]; then
    echo "fatal: no key found at $keypath" ; exit 1
  fi

  echo "Building Radicle $version.."
  echo "Creating archive of repository at $rev in $gitarchive.."
  git archive --format tar.gz -o "$gitarchive" HEAD

  echo "Building image (radicle-build).."
  podman --cgroup-manager=cgroupfs build \
    --env GIT_COMMIT_TIME=$SOURCE_DATE_EPOCH \
    --env GIT_HEAD=$rev \
    --env RADICLE_VERSION=$version \
    --arch amd64 --tag radicle-build-$version -f ./build/Dockerfile - < $gitarchive

  echo "Creating container (radicle-build-container).."
  podman --cgroup-manager=cgroupfs create --replace --name radicle-build-container radicle-build

  targets="\
    x86_64-unknown-linux-musl \
    aarch64-unknown-linux-musl \
    x86_64-apple-darwin \
    aarch64-apple-darwin"

  for target in $targets; do
    outdir=build/artifacts/$target

    echo "Copying artifacts for $target.."
    mkdir -p $outdir
    rm -f $outdir/*

    # Copy binaries to target folder.
    podman cp radicle-build-container:/bin/$target/rad            $outdir
    podman cp radicle-build-container:/bin/$target/radicle-node   $outdir
    podman cp radicle-build-container:/bin/$target/radicle-httpd  $outdir
    podman cp radicle-build-container:/bin/$target/git-remote-rad $outdir

    # Copy man pages from repo root.
    for adoc in $(find . -maxdepth 1 -type f -name '*.1.adoc'); do
      podman cp radicle-build-container:"/src/${adoc%.adoc}" $outdir
      # Remove all comments, since they include non-reproducible information,
      # such as version numbers.
      sed -i '/^.\\\"/d' "$outdir/${adoc%.adoc}"
    done

    filename="radicle-$version-$target"
    filepath="$outdir/$filename.tar.xz"

    # Create and compress reproducible archive.
    echo "Creating $filepath.."
    tar --sort=name \
        --mtime="@$SOURCE_DATE_EPOCH" \
        --owner=0 --group=0 --numeric-owner \
        --format=posix \
        --pax-option=exthdr.name=%d/PaxHeaders/%f,delete=atime,delete=ctime \
        --mode='go+u,go-w' \
        --create --file $tempdir/$filename.tar $outdir
    xz --compress -6 --stdout $tempdir/$filename.tar > $filepath

    # Output SHA256 digest of archive.
    checksum="$(sha256sum $filepath)"
    echo "Checksum of $filepath is $(echo "$checksum" | cut -d' ' -f1)"
    echo "$checksum" > $filepath.sha256

    # Sign archive and verify archive.
    rm -f $filepath.sig # Delete existing signature
    ssh-keygen -Y sign -n file -f $keypath $filepath
    ssh-keygen -Y check-novalidate -n file -s $filepath.sig < $filepath
  done

  rm -f $gitarchive
  podman rm radicle-build-container
}

# Run build with timings.
time main "$@"
echo

# Show artifact checksums.
build/checksums.sh
echo

echo "Build ran successfully."
