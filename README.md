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

## 📋 Listing sessions — `rtimelogger list`

The `list` command displays saved work sessions, supporting multiple layouts and levels of detail.

### **Basic usage**:

```bash
rtimelogger list                     # current month
```

Shows the sessions for the current month using the default tabular layout.

### 📅 **Supported periods**

```bash
rtimelogger list --period 2025-12
rtimelogger list --period 2025
rtimelogger list --period 2025-12-01
rtimelogger list --period 2025-12-01:2025-12-31
rtimelogger list --period all
```

### 📆 **Weekday display**

The weekday is shown inside the date column, using the format:

```text
YYYY-MM-DD (Mo)
YYYY-MM-DD (Monday)
```

The format is controlled by the show_weekday configuration option:

| Value    | Output example        |
|----------|-----------------------|
| `none`   | `2025-12-19`          |
| `short`  | `2025-12-19 (Mo)`     |
| `medium` | `2025-12-19 (Mon)`    |
| `long`   | `2025-12-19 (Monday)` |

### 📊 Standard output

```bash
rtimelogger list --period 2025-12
```

Example:

```text
DATE (WD)        | POSITION        |  IN   | LNCH  |  OUT  |  TGT  |  ΔWORK
---------------------------------------------------------------------------
2025-12-19 (Fr)  | Remote          | 08:55 | 00:30 | 18:27 | 17:01 | -02h04m
```

**Columns explained**:

- **IN** – first check-in of the day
- **LNCH** – total lunch break duration
- **OUT** – last check-out
- **TGT** – planned exit time (minimum required work time)
- **ΔWORK** – worked surplus or deficit

### 🧾 Pair details (--details)

```bash
rtimelogger list --period 2025-12-19 --details
```

Displays the **individual** IN/OUT pairs for the selected day. It is available **only** for single-day periods or
`--today`.

**Output example**:

```text
DETAILS
PAIR |  IN   |  OUT  | WORKED | LUNCH | POSITION | WG
------------------------------------------------------
  1  | 08:55 | 09:37 | 00h42m |  0m   | Remote   |
  2  | 13:07 | 18:27 | 04h50m | 30m   | Remote   |
```

**Columns explained**:

- **PAIR** – pair index
- **IN** / **OUT** – timestamps for the pair
- **WORKED** – worked time for the pair
- **LUNCH** – lunch break for the pair
- **POSITION** – position for the pair
- **WG** – working gap indicator (🔗 for working gap, ✂️ for non-working gap)

### 📦 Compact view (--compact)

```bash
rtimelogger list --period 2025-12 --compact
```

Shows a condensed, single-line-per-day view, suitable for long periods.

Example:

```text
DATE (WD)        | POSITION | IN / LNCH / OUT       | TGT   | ΔWORK
--------------------------------------------------------------------
2025-12-19 (Fr)  | Remote   | 08:55 / 00:30 / 18:27 | 17:01 | Δ -02h04m
2025-12-22 (Mo)  | Holiday  | --:-- / --:-- / --:-- | --:-- | Δ -
```

**Characteristics**:

- compact horizontal layout
- weekday forced to short format
- no pair details

> ⚠️ `--compact` **cannot be combined** with `--details`

### Events listing (--events)

```bash
rtimelogger list --period 2025-12-15 --events
```

Displays the raw IN / OUT events for the selected day.

**Output example**:

```text
EVENTS:

     Date Time     | Type |    Lunch     |     Position     | Source | Pair | Work Gap
----------------------------------------------------------------------------------------
→ 2025-12-19 08:55 |   in | lunch  0 min | Remote           |  cli   |   1  |
             09:37 |  out | lunch  0 min | Remote           |  cli   |   1  |
             13:07 |   in | lunch  0 min | Remote           |  cli   |   2  |
             18:27 |  out | lunch 30 min | Remote           |  cli   |   2  |
```

### 🏖️ Holiday days

Days marked as **Holiday**:

- display no time values (--:--)
- do not affect surplus calculations
- are rendered as neutral rows

### ➕ Period total

At the end of the output, a cumulative total is always displayed:

```text
Σ Total ΔWORK: +02h04m
```

The total accounts for:

- lunch breaks
- work gaps
- holidays (neutral contribution)

### 🔢 JSON output (--json)

```bash
rtimelogger list --period 2025-12 --json
```
Outputs the data in JSON format for easy integration with other tools or scripts.

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
