#!/bin/bash

get_architecture() {
  local _ostype _cputype _bitness _arch _clibtype
  _ostype="$(uname -s)"
  _cputype="$(uname -m)"
  _clibtype="gnu"

  if [ "$_ostype" = Linux ]; then
    if ldd --_requested_version 2>&1 | grep -q 'musl'; then
      _clibtype="musl"
    fi
  fi

  if [ "$_ostype" = Darwin ] && [ "$_cputype" = i386 ]; then
    # Darwin `uname -m` lies
    if sysctl hw.optional.x86_64 | grep -q ': 1'; then
      _cputype=x86_64
    fi
  fi

  case "$_ostype" in
  Linux)
    check_proc
    _ostype=unknown-linux-$_clibtype
    _bitness=$(get_bitness)
    ;;

  Darwin)
    _ostype=apple-darwin
    ;;

  MINGW* | MSYS* | CYGWIN* | Windows_NT)
    _ostype=pc-windows-gnu
    ;;

  *)
    err "unrecognized OS type: $_ostype"
    ;;
  esac

  case "$_cputype" in
  aarch64 | arm64)
    _cputype=aarch64
    ;;

  x86_64 | x86-64 | x64 | amd64)
    _cputype=x86_64
    ;;
  *)
    err "unknown CPU type: $_cputype"
    ;;
  esac

  _arch="${_cputype}-${_ostype}"

  RETVAL="$_arch"
}

repo="war-in/universal-sierra-compiler"
binary_name="universal-sierra-compiler"

if ! command -v jq &> /dev/null; then
    echo "Please install 'jq' to run this script."
    exit 1
fi

# Fetch the latest release tag from GitHub API
release_tag=$(curl -# -Ls -H 'Accept: application/json' "https://github.com/${repo}/releases/latest" | sed -e 's/.*"tag_name":"\([^"]*\)".*/\1/')

# Define the operating system and architecture
get_architecture
_arch="$RETVAL"

artifact_name=${binary_name}-${release_tag}-${_arch}

# Define the download URL
download_url="https://github.com/${repo}/releases/download/${release_tag}/${artifact_name}.tar.gz"

# Create a temporary directory
temp_dir=$(mktemp -d)

# Download and extract the archive
echo "Downloading and extracting ${artifact_name}..."
curl -L "${download_url}" | tar -xz -C "${temp_dir}"

# Move the binary to a directory in the PATH
sudo mv "${temp_dir}"/"${artifact_name}"/bin/${binary_name} /usr/local/bin

# Clean up temporary files
rm -rf "${temp_dir}"

echo "${binary_name} has been installed successfully."
