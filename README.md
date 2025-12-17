<h1 style="text-align: left; display: flex; align-items: center;">
  <img src="res/rtimelogger.svg" width="90" style="vertical-align: middle; margin-right: 8px;" alt="rTimelogger Logo"/>
  rTimelogger
</h1>

[![Build Status](https://github.com/umpire274/rTimelogger/actions/workflows/ci.yml/badge.svg)](https://github.com/umpire274/rTimelogger/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/umpire274/rTimelogger)](https://github.com/umpire274/rTimelogger/releases)
[![codecov](https://codecov.io/gh/umpire274/rTimelogger/graph/badge.svg)](https://codecov.io/gh/umpire274/rTimelogger)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**rTimelogger** is a cross-platform **command-line time tracking tool** written in Rust.
It tracks working time using **IN / OUT events**, supports multiple locations, lunch rules, working gaps,
and computes **expected exit time** and **daily surplus** accurately.

---

## 🚀 What’s new in **v0.8.0**

Version **0.8.0** is the first **stable release** based on the new **timeline engine**.

### ✅ Timeline engine (stable)

* Unlimited **IN / OUT pairs per day**
* Deterministic reconstruction:
  **events → timeline → pairs**
* Correct handling of:

    * lunch breaks
    * working gaps
    * multi-position days
* Legacy `work_sessions` logic fully retired

---

### 🔗 Working gap support

Time **between pairs** can be explicitly controlled:

* `--work-gap` → gap counts as working time
* `--no-work-gap` → gap does **not** count as working time

Features:

* Stored in the database
* Editable retroactively
* Fully reflected in:

    * worked time
    * expected exit
    * surplus

Visual indicators:

* 🔗 working gap
* ✂️ non-working gap

---

### 🧮 Accurate calculations

* Worked time:

    * sum of all pairs
    * minus non-working gaps
    * plus working gaps only when marked
* Expected exit:

    * based on first IN
    * configured minimum working time
    * lunch rules and lunch window
* Surplus is correct in **all multi-pair scenarios**

---

### 🧠 Consistency improvements

* `OUT` events inherit position from `IN` when `--pos` is omitted
* Pair details show the **actual position of each pair**
* Clean event listing:

    * no duplicated dates
    * aligned output
* Unified CLI message system:

    * ℹ️ info · ⚠️ warning · ❌ error · ✅ success

---

## ✨ Features

* Event-based time tracking (IN / OUT)
* Multiple working positions:

    * `O` Office
    * `R` Remote
    * `C` Client / On-site
    * `H` Holiday
    * `M` Mixed
* Automatic calculation of:

    * expected exit
    * daily surplus
* Configurable lunch rules
* Event mode with:

    * pairing
    * per-pair summaries
    * JSON output
    * unmatched detection
* Internal audit log
* Safe database migrations with automatic backups
* Cross-platform (Linux, macOS, Windows)

---

## 📦 Installation

### 🦀 Cargo (recommended)

```bash
cargo install rtimelogger
```

### 🐧 Arch Linux (AUR)

```bash
yay -S rtimelogger
# or
paru -S rtimelogger
```

### 🍺 Homebrew (macOS / Linux)

```bash
brew tap umpire274/tap
brew install rtimelogger
```

### 🐧📦 Linux (Debian / Ubuntu)

Starting from **v0.8.0**, rFortune provides an official **`.deb` package**.

You can install it directly from the GitHub Releases page:

```bash
sudo dpkg -i rtimelogger_<version>_amd64.deb
```

To verify integrity, download the corresponding `.sig` file and verify it with GPG (see below).

```bash
sha256sum -c rtimelogger_<version>_amd64.deb.sha256
gpg --verify rtimelogger_<version>_amd64.deb.sig
```

If dependencies are missing, complete the installation with:

```bash
sudo apt --fix-broken install
```

### 🐧🔧 Other Linux distros

You can still use the prebuilt tarball:

```bash
tar -xvf rtimelogger-<version>-x86_64-unknown-linux-gnu.tar.gz
sudo mv rtimelogger /usr/local/bin/
```

### 🍎 macOS

You can use the prebuilt tarballs for Intel or Apple Silicon:

```bash
tar -xvf rtimelogger-<version>-x86_64-apple-darwin.tar.gz
sudo mv rtimelogger /usr/local/bin/
```

or

```bash
tar -xvf rtimelogger-<version>-aarch64-apple-darwin.tar.gz
sudo mv rtimelogger /usr/local/bin/
```

### 🪟 Windows

Download the prebuilt zip file, extract it, and move `rtimelogger.exe` to a directory in your `PATH`, e.g.,
`C:\Windows\System32\` or create a dedicated folder like `C:\Program Files\rtimelogger\` and add it to your system `PATH`.

---

## ⚙️ Configuration

Initialize configuration and database:

```bash
rtimelogger init
```

Example `rtimelogger.conf`:

```yaml
database: /home/user/.rtimelogger/rtimelogger.sqlite
default_position: O
min_work_duration: 8h
min_duration_lunch_break: 30
max_duration_lunch_break: 90
separator_char: "-"
show_weekday: None   # None | Short | Medium | Long
```

Override database path at runtime:

```bash
rtimelogger --db /custom/path/db.sqlite <command>
```

---

## 🧭 Main commands overview

| Command  | Description                                |
|----------|--------------------------------------------|
| `init`   | Initialize DB and config                   |
| `add`    | Add or edit IN / OUT events                |
| `list`   | Show sessions, events, or details          |
| `del`    | Delete events or pairs (with confirmation) |
| `backup` | Backup database (optional compression)     |
| `export` | Export data (CSV / JSON / XLSX / PDF)      |
| `db`     | Database utilities                         |
| `config` | Manage configuration file                  |
| `log`    | Show internal audit log                    |

---

## ➕ Add work sessions — `rtimelogger add`

```bash
rtimelogger add <DATE> [OPTIONS]
```

Examples:

```bash
rtimelogger add 2025-12-15 --in 09:00
rtimelogger add 2025-12-15 --out 17:30
rtimelogger add 2025-12-15 --in 09:00 --lunch 30 --out 17:30
rtimelogger add 2025-12-15 --edit --pair 1 --out 18:00
rtimelogger add 2025-12-15 --out 10:30 --work-gap
rtimelogger add 2025-12-15 --edit --pair 2 --no-work-gap
```

---

## 📋 List data — `rtimelogger list`

```bash
rtimelogger list                     # current month
rtimelogger list --today
rtimelogger list --period 2025-12
rtimelogger list --period 2025-12-15
rtimelogger list --events
```

### ℹ️ Note on `--details`

`--details` is valid **only** with:

* `--today`
* `--period <single day>`

Examples:

```bash
rtimelogger list --today --details
rtimelogger list --period 2025-12-15 --details
```

---

## 🗑️ Delete data — `rtimelogger del`

```bash
rtimelogger del 2025-12-15
rtimelogger del --pair 2 2025-12-15
```

All deletions require confirmation and automatically reindex pairs.

---

## 💾 Backup database — `rtimelogger backup`

```bash
rtimelogger backup --file /abs/path/backup.sqlite
rtimelogger backup --file /abs/path/backup.sqlite --compress
```

* confirmation before overwrite
* ZIP on Windows
* TAR.GZ on Linux/macOS

---

## 📤 Export data — `rtimelogger export`

```bash
rtimelogger export --format pdf --file /abs/path/report.pdf --range 2025-12
```

Supported formats:

* `csv`
* `json`
* `xlsx`
* `pdf`

Output path must be **absolute**.

---

## 🗄️ Database utilities — `rtimelogger db`

```bash
rtimelogger db --info
rtimelogger db --check
rtimelogger db --vacuum
rtimelogger db --migrate
```

---

## ⚙️ Configuration management — `rtimelogger config`

```bash
rtimelogger config --print
rtimelogger config --edit
rtimelogger config --migrate
```

Missing fields are added automatically with defaults.

---

## 📜 Internal audit log — `rtimelogger log`

```bash
rtimelogger log --print
```

Shows timestamped internal operations (add, del, migrate, backup, …).

---

## 🔄 Upgrading from older versions

If you are upgrading from **0.7.x or earlier**, read:

➡️ **[UPGRADE-0.7-to-0.8.md](UPGRADE-0.7-to-0.8.md)**

This document explains:

* schema changes
* migration behavior
* removed legacy features
* important behavioral differences

---

## 📚 Documentation

* 📄 [CHANGELOG.md](CHANGELOG.md)
* 🔄 [UPGRADE-0.7-to-0.8.md](UPGRADE-0.7-to-0.8.md)

---

## 📜 License

MIT License – see [LICENSE](LICENSE).
