#!/bin/sh
# Shared by the App and PKG. The optional volume argument is for Installer/test volumes.
set -eu
operation=${1:?operation required}
source_cli=${2:-}
new_version=${3:-}
volume=${4:-/}
volume=${volume%/}
bin_dir="$volume/usr/local/bin"
lib_dir="$volume/usr/local/lib/overstory"
command_path="$bin_dir/overstory"
installed="$lib_dir/overstory-cli"
marker="$lib_dir/.managed-by-overstory"
owner="app.overstory.desktop.cli"
fail() { printf '%s\n' "$*" >&2; exit 1; }
exists() { [ -e "$1" ] || [ -L "$1" ]; }
is_managed() {
  [ -f "$marker" ] && [ ! -L "$marker" ] &&
    [ "$(cat "$marker")" = "$owner" ] &&
    [ ! -L "$lib_dir" ] && [ ! -L "$installed" ]
}
check_paths() {
  for path in "$volume/usr" "$volume/usr/local" "$volume/usr/local/bin" "$volume/usr/local/lib" "$lib_dir"; do
    [ ! -L "$path" ] || fail "安装路径是符号链接：$path"
    if exists "$path"; then [ -d "$path" ] || fail "安装目录被文件占用：$path"; fi
  done
  if exists "$lib_dir"; then is_managed || fail "CLI 目录被其他程序占用：$lib_dir"; fi
  if exists "$command_path"; then
    [ -L "$command_path" ] &&
      [ "$(readlink "$command_path")" = "/usr/local/lib/overstory/overstory-cli" ] ||
      fail "overstory 命令被其他程序占用：$command_path"
    is_managed || fail "CLI 安装信息不完整，请先检查 $lib_dir"
  fi
}
valid_version() { printf '%s\n' "$1" | /usr/bin/grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; }
check_version() {
  valid_version "$new_version" || fail "CLI 版本格式无效"
  if [ -f "$installed" ]; then
    previous=$("$installed" --version) || fail "已安装的 CLI 无法读取版本"
    previous=${previous#overstory }
    valid_version "$previous" || fail "已安装的 CLI 版本格式无效"
    /usr/bin/awk -v old="$previous" -v new="$new_version" 'BEGIN {
      split(old,a,"."); split(new,b,".");
      for(i=1;i<=3;i++) { if(a[i]+0>b[i]+0) exit 1; if(a[i]+0<b[i]+0) exit 0 }
    }' || fail "已安装版本 $previous 高于当前版本 ${new_version}，不能降级"
  fi
}
check_paths
case "$operation" in
  check) check_version ;;
  install)
    check_version
    [ -f "$source_cli" ] && [ ! -L "$source_cli" ] || fail "App 内置 CLI 不存在"
    [ "$("$source_cli" --version)" = "overstory $new_version" ] || fail "App 与内置 CLI 版本不一致"
    /bin/mkdir -p "$bin_dir" "$lib_dir"
    temp=$(/usr/bin/mktemp "$lib_dir/.overstory-cli.XXXXXX")
    trap 'rm -f "$temp"' EXIT HUP INT TERM
    /bin/cp "$source_cli" "$temp"
    /bin/chmod 755 "$temp"
    /bin/mv -f "$temp" "$installed"
    printf '%s\n' "$owner" > "$marker"
    /bin/chmod 644 "$marker"
    if ! exists "$command_path"; then /bin/ln -s /usr/local/lib/overstory/overstory-cli "$command_path"; fi
    ;;
  uninstall)
    if exists "$command_path"; then /bin/rm "$command_path"; fi
    if is_managed; then
      /bin/rm -f "$installed" "$marker"
      /bin/rmdir "$lib_dir" 2>/dev/null || true
    fi
    ;;
  *) fail "不支持的 CLI 操作" ;;
esac
