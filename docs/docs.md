# buf
These docs serve as a guide on how to use the CLI (command-line interface)

## Usage

```
buf [OPTIONS] --source <FILE> --target <DEVICE>
buf --list
```

`--source` and `--target` are required for all write operations. All other flags
are optional.

## Flags

### `-s, --source <FILE>`

Path to the source ISO or IMG file to flash.

```sh
buf -s archlinux.iso -t /dev/sdb
buf --source /home/user/ubuntu.iso --target /dev/sdc
```

Relative paths are resolved to absolute before any privilege elevation occurs,
so the correct file is always used regardless of working directory changes during
the elevation process.

### `-t, --target <DEVICE>`

Path to the target block device to write to.

| Platform | Example |
|----------|---------|
| Linux    | `/dev/sdb` |
| macOS    | `/dev/disk2` |
| Windows  | `\\.\PhysicalDrive1` |

Use `--list` to see available devices before writing.

**This will overwrite all data on the target device. Double-check the path.**

### `-l, --list`

List all detected storage devices and exit. No write is performed.

```sh
buf --list
buf -l
```

Output example:

```
  DEVICE            SIZE        MODEL
  /dev/sda          931.51 GiB  Samsung SSD 870
  /dev/sdb          57.66 GiB   SanDisk Ultra
```

Devices are sorted with removable drives first.

### `-b, --block-size <SIZE>`

Size of each write block. Default is `32MiB`.

Accepted suffixes (case-insensitive, all powers of 1024):

| Suffix | Multiplier |
|--------|------------|
| B (or none) | 1 |
| K, KB, KiB | 1024 |
| M, MB, MiB | 1048576 |
| G, GB, GiB | 1073741824 |

```sh
buf -s image.iso -t /dev/sdb -b 64MiB
buf -s image.iso -t /dev/sdb --block-size 4096
buf -s image.iso -t /dev/sdb -b 1G
```

Larger block sizes generally give better throughput on fast drives. The default
of 32MiB is a good balance for most USB drives. Very large values (above a few
hundred MiB) are unlikely to help and are capped at 256MiB.

### `--offset <BYTES>`

Start writing at this byte offset into the target device instead of the
beginning. Takes a plain integer in bytes. Default is `0`.

```sh
buf -s image.iso -t /dev/sdb --offset 1048576
```

Useful for writing to a specific partition or past a reserved region. The source
size is checked against the available space after the offset, so buf will error
out rather than run off the end of the device.

### `-f, --force`

Skip the confirmation prompt and write immediately.

```sh
buf -s image.iso -t /dev/sdb --force
buf -s image.iso -t /dev/sdb -f
```

By default buf prints the source path, size, and target device and waits for
`y` before writing. This flag bypasses that. Useful for scripting.

### `--dry-run`

Run all validation checks without writing any data. Exits after validation.

```sh
buf -s image.iso -t /dev/sdb --dry-run
```

Checks performed:
- Source file exists and is non-empty
- Target device exists and is writable
- Source fits within available target space after the offset
- Block size is valid

Nothing is written to the target. The target file is opened for writing as part
of the access check and then immediately closed.

### `-n, --no-logging`

Disable log file creation. Warnings and errors still print to stderr.

```sh
buf -s image.iso -t /dev/sdb --no-logging
buf -s image.iso -t /dev/sdb -n
```

By default, buf creates a timestamped log file in the user's home directory on
each run. This flag suppresses that.

### `-v, --verbose`

Enable debug-level logging. Logs every block written, ioctl results, device
paths, and internal state. Implies log file creation unless `--no-logging` is
also set.

```sh
buf -s image.iso -t /dev/sdb --verbose
buf -s image.iso -t /dev/sdb -v
```

### `--help`

Print usage information and exit.

### `--version`

Print the version and exit.

## Logging

Unless `--no-logging` is passed, buf writes a timestamped log file to the home
directory on each run. The filename format is:

```
buf-MM-DD-YY-HH:MM:SS.log   (Linux, macOS)
buf-MM-DD-YY-HH-MM-SS.log   (Windows, colons are not valid in filenames)
```

The log path is printed at startup:

```
  Logging to: /home/user/buf-05-30-26-14:22:01.log
```

Log files contain info-level output by default, debug-level with `--verbose`.
Warnings and errors are always mirrored to stderr regardless of the log settings.

## Privileges

Writing to block devices requires root on Linux/macOS and Administrator on Windows.
If buf is not already running with the required privileges it will attempt to
re-launch itself elevated automatically.

On Linux it tries `sudo` first, then `pkexec` as a fallback. On Windows it
triggers a UAC prompt via `ShellExecuteW` with the `runas` verb.

If neither elevator is available on Linux, buf exits with an error asking you to
re-run as root manually.

## Examples

List devices to find your USB drive:

```sh
buf --list
```

Flash an ISO with confirmation prompt:

```sh
buf -s ubuntu-24.04.iso -t /dev/sdb
```

Flash silently from a script:

```sh
buf -s ubuntu-24.04.iso -t /dev/sdb --force --no-logging
```

Validate that the image fits on the drive without writing:

```sh
buf -s ubuntu-24.04.iso -t /dev/sdb --dry-run
```

Flash with a larger block size for a fast drive:

```sh
buf -s image.iso -t /dev/sdb -b 128MiB
```

Flash to a specific offset (e.g. past a 1 MiB reserved region):

```sh
buf -s image.iso -t /dev/sdb --offset 1048576
```

Windows, flashing to the second physical drive:

```sh
buf -s image.iso -t \\.\PhysicalDrive1 --force
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Error (validation failure, write error, user abort, etc.) |