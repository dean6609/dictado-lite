#!/bin/sh
# Install Dictado Lite for the current user, from the unpacked release folder.
#   ./install.sh [--prefix DIR]     install into DIR (default ~/.local)
#   ./install.sh --uninstall        remove the program; settings and models stay
# Program files go to DIR/lib/dictado-lite and the command to DIR/bin.
set -eu
prefix="$HOME/.local"
action=install
while [ $# -gt 0 ]; do
    case $1 in
    --prefix)
        prefix=${2:?"--prefix needs a folder"}
        shift
        ;;
    --uninstall) action=uninstall ;;
    *)
        sed -n '2,5s/^# \{0,1\}//p' "$0"
        exit 2
        ;;
    esac
    shift
done
home="$prefix/lib/dictado-lite"
link="$prefix/bin/dictado-lite"
if [ "$(uname -s)" = Darwin ]; then
    data="$HOME/Library/Application Support/DictadoLite"
else
    data="${XDG_DATA_HOME:-$HOME/.local/share}/dictado-lite"
fi

# Only a link into our own folder is ours to replace or remove.
owned_link() {
    [ -L "$link" ] && [ "$(readlink "$link")" = "$home/dictado-lite" ]
}

if [ $action = uninstall ]; then
    owned_link && rm "$link"
    rm -rf "$home"
    echo "Removed Dictado Lite from $prefix."
    echo "Settings, word list and models were kept in: $data"
    exit 0
fi

source=$(cd "$(dirname "$0")" && pwd)
if [ ! -x "$source/dictado-lite" ]; then
    echo "Run this script from the unpacked release folder." >&2
    exit 1
fi
if [ -e "$link" ] || [ -L "$link" ]; then
    if ! owned_link; then
        echo "$link already exists and is not from this installer; nothing was changed." >&2
        exit 1
    fi
fi

# Copy into a fresh folder first, so a failed copy never breaks a working install.
mkdir -p "$prefix/lib" "$prefix/bin"
rm -rf "$home.new"
mkdir "$home.new"
for item in dictado-lite lib LICENSE NOTICE THIRD_PARTY_NOTICES.md licenses; do
    [ -e "$source/$item" ] && cp -R "$source/$item" "$home.new/"
done
rm -rf "$home"
mv "$home.new" "$home"
ln -sfn "$home/dictado-lite" "$link"
if [ "$(uname -s)" = Darwin ]; then
    # The binary is not signed by a registered developer; clear the download
    # flag on this copy so macOS lets it run from the terminal.
    xattr -dr com.apple.quarantine "$home" 2>/dev/null || true
elif command -v ldd >/dev/null 2>&1 && ldd "$home/dictado-lite" | grep -q "not found"; then
    echo "Missing system libraries:" >&2
    ldd "$home/dictado-lite" | grep "not found" >&2
    echo "Install ALSA (libasound2 on Debian/Ubuntu, alsa-lib on Fedora/Arch) and run this again." >&2
    exit 1
fi
echo "Installed Dictado Lite in $home"

case ":$PATH:" in
*":$prefix/bin:"*) command=dictado-lite ;;
*)
    command="$link"
    echo "Add $prefix/bin to your PATH to run it as just 'dictado-lite'."
    ;;
esac

# Weights are downloaded separately, once, and verified before use.
if ! ls "$data/models" 2>/dev/null | grep -q '\.gguf$'; then
    answer=n
    if [ -t 0 ]; then
        printf "Download the recommended speech model now (about 705 MiB)? [Y/n] "
        read -r answer || answer=n
        answer=${answer:-y}
    fi
    case $answer in
    [Yy]*) "$link" --download-model >/dev/null && echo "Model saved in $data/models" ;;
    *) echo "Download it later with: $command --download-model" ;;
    esac
fi
echo "Dictate with: $command --record"
