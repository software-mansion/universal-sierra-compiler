#!/bin/bash

BINARY_NAME="universal-sierra-compiler"
LOCAL_BIN="${HOME}/.local/bin"

main () {
  download_and_extract_binary

  addBinaryToPath

  echo "${BINARY_NAME} (${release_tag}) has been installed successfully."
}

download_and_extract_binary() {
  repo="https://github.com/software-mansion/universal-sierra-compiler"
  # Fetch the latest release tag from GitHub API
  release_tag=$(curl -# -Ls -H 'Accept: application/json' "${repo}/releases/latest" | sed -e 's/.*"tag_name":"\([^"]*\)".*/\1/')

  # Define the operating system and architecture
  get_architecture
  _arch="$RETVAL"

  artifact_name=${BINARY_NAME}-${release_tag}-${_arch}

  echo "Downloading and extracting ${artifact_name}..."
  # Create a temporary directory
  temp_dir=$(mktemp -d)

  # Download and extract the archive
  curl -L "${repo}/releases/download/${release_tag}/${artifact_name}.tar.gz" | tar -xz -C "${temp_dir}"

  # Move the binary to a LOCAL_BIN directory
  mkdir -p "${LOCAL_BIN}"
  mv "${temp_dir}/${artifact_name}/bin/${BINARY_NAME}" "${LOCAL_BIN}"

  # Clean up temporary files
  rm -rf "${temp_dir}"
}

get_architecture() {
  local _ostype _cputype _arch _clibtype
  _ostype="$(uname -s)"
  _cputype="$(uname -m)"
  _clibtype="gnu"

  if [ "$_ostype" = Linux ] && ldd --_requested_version 2>&1 | grep -q 'musl'; then
    _clibtype="musl"
  fi

  if [ "$_ostype" = Darwin ] && [ "$_cputype" = i386 ] && sysctl hw.optional.x86_64 | grep -q ': 1'; then
    _cputype=x86_64
  fi

  case "$_ostype" in
  Linux)
    _ostype=unknown-linux-$_clibtype
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

addBinaryToPath() {
  # Store the correct profile file (i.e. .profile for bash or .zshenv for ZSH).
  case $SHELL in
  */zsh)
      PROFILE=${ZDOTDIR-"$HOME"}/.zshenv
      ;;
  */bash)
      PROFILE=$HOME/.bashrc
      ;;
  */fish)
      PROFILE=$HOME/.config/fish/config.fish
      ;;
  */ash)
      PROFILE=$HOME/.profile
      ;;
  *)
      echo "universal-sierra-compiler: could not detect shell, manually add ${LOCAL_BIN} to your PATH."
      exit 0
  esac

  # Only add universal-sierra-compiler if it isn't already in PATH.
  case ":$PATH:" in
      *":${LOCAL_BIN}/${BINARY_NAME}:"*)
          # The path is already in PATH, do nothing
          echo tutaj
          ;;
      *)
          # Add the universal-sierra-compiler directory to the path
          echo >> "$PROFILE" && echo "export PATH=\"\$PATH:$LOCAL_BIN/$BINARY_NAME\"" >> "$PROFILE"
          ;;
  esac
}

main