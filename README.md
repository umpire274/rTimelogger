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

## 🚀 What's new in **v0.8.7**

### 🐛 Fixed — TGT calculation with non-work gaps (v0.8.7)

The `TGT` (target exit time) calculation was incorrect when non-working gaps were present between pairs.

- **Before**: `TGT = first_in + expected_work_time` (gaps ignored → mismatch with ΔWORK)
- **After**: `TGT = first_in + expected_work_time + total_non_work_gaps` (consistent)
- `ΔWORK` is now always computed as `OUT − TGT`, removing implicit double-counting
- No database or schema changes required

### 🤒 Sick Leave marker day (v0.8.6)

A new day position **Sick Leave** (`S`) has been introduced.

- Use `--pos s` to mark a sick leave day
- Optional `--to <DATE>` to apply sick leave over a date range (the command `DATE` is the start)
- Weekends, national holidays and dates already containing events are automatically skipped
- Sick Leave days do not contribute to ΔWORK totals and display `--:--` for all time fields

### ➕ Show target exit on IN event (v0.8.5)

After adding an `IN` event, the calculated **target exit time (TGT)** is now immediately displayed in the output,
so you always know when you need to leave.

### 📋 National Holiday rendering improvements (v0.8.4)

- The `meta` field (e.g. holiday name) is now shown **instead of** `--:--` placeholders for National Holiday days
- The holiday row layout adapts dynamically to the current table width and weekday display mode
- Meta values are Unicode-safe: filtered, concatenated, and truncated with a trailing `…` when needed

### 📥 JSON / CSV holiday import (v0.8.3)

See the **Import data** section below for full documentation.

---

## ✨ Features

* Event-based time tracking (IN / OUT)
* Multiple working positions:

    * `O` Office
    * `R` Remote
    * `C` Client / On-site
    * `N` National holiday
    * `H` Holiday
    * `S` Sick Leave
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
`C:\Windows\System32\` or create a dedicated folder like `C:\Program Files\rtimelogger\` and add it to your system
`PATH`.

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
lunch_window: 12:30-14:00
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
rtimelogger add 2025-12-25 --pos n
rtimelogger add 2025-03-10 --pos s
rtimelogger add 2025-03-10 --pos s --to 2025-03-14
```

### 📌 Day positions

rTimelogger supports multiple day positions to describe how a working day (or non-working day) is classified.

**Supported positions**

| Code | Name             | Description                                                   |
|------|------------------|---------------------------------------------------------------|
| `O`  | Office           | Regular office working day                                    |
| `R`  | Remote           | Remote working day                                            |
| `C`  | On-site          | Working day at customer site                                  |
| `M`  | Mixed            | Mixed working locations                                       |
| `H`  | Holiday          | Personal holiday (counts against personal leave allowance)    |
| `N`  | National holiday | Public holiday (does **not** affect personal leave allowance) |
| `S`  | Sick Leave       | Sick day (non-working marker, does not reduce holiday budget) |

### ➕ Adding a national holiday

To mark a **public/national holiday**, use the `add` command with the national position.

```bash
rtimelogger add 2025-12-25 --pos n
```

or

```bash
rtimelogger add 2025-12-25 --pos national
```

**Behavior**

- No `--in`, `--out`, `--lunch`, or `--work-gap` parameters are allowed
- The day is recorded as a non-working public holiday
- The day does not contribute to worked time
- The day does not reduce personal holiday allowance

### 📋 List output behavior

**National holiday days**

In both standard and compact list views:

- All time-related fields are displayed as `--:--`
- Target end (`TGT`) is not computed
- Worked delta (`ΔWORK`) is neutral (`-`)
- The day is clearly labeled as **National holiday**

Example:

```text
2025-12-25 (Thu) | National holiday | --:-- | --:-- | --:-- | --:-- | -
```

### ⚖️ Holiday vs National holiday

| Aspect                   | Holiday (`H`) | National holiday (`N`) |
|--------------------------|---------------|------------------------|
| Working day              | ❌             | ❌                      |
| Counts as personal leave | ✅             | ❌                      |
| Expected time            | ❌             | ❌                      |
| ΔWORK contribution       | ❌             | ❌                      |
| Requires time entries    | ❌             | ❌                      |

---

### 🤒 Adding a sick leave day

To mark a **sick leave day**, use the `add` command with the sick leave position.

```bash
rtimelogger add 2025-03-10 --pos s
```

To mark a **sick leave range** (e.g. a week), add the `--to` option:

```bash
rtimelogger add 2025-03-10 --pos s --to 2025-03-14
```

**Behavior**

- No `--in`, `--out`, `--lunch`, or `--work-gap` parameters are allowed
- The day (or range) is recorded as a non-working sick leave marker
- Weekends, national holidays, and dates that already contain events are automatically skipped
- Sick leave days do not contribute to worked time and are not deducted from personal holiday allowance

**Output example**

```text
2025-03-10 (Mon) | Sick Leave | --:-- | --:-- | --:-- | --:-- | -
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

## Import data (JSON / CSV)

Starting from **v0.8.3**, rTimelogger supports importing work sessions and holidays from external files.  
This feature is designed to simplify **preventive data entry**, especially for **national holidays**.

The import system is safe by default and provides a full **dry-run mode**.

---

### Supported formats

#### JSON

Flexible JSON structures are supported. The following formats are valid:

**Root object with `holidays`:**

```json
{
  "year": 2026,
  "holidays": [
    {
      "date": "2026-01-01",
      "name": "New Year"
    },
    {
      "date": "2026-01-06",
      "name": "Epiphany"
    }
  ]
}
```

**Root object with `days`:**

```json
{
  "days": [
    {
      "date": "2026-05-01",
      "position": "N",
      "name": "Labour Day"
    }
  ]
}
```

**Root array of day objects:**

```json
[
  {
    "date": "2026-12-25",
    "name": "Christmas Day"
  }
]

```

**Notes**:

- `position` is optional:
    - if omitted, it defaults to `NationalHoliday`
- `name` is optional and stored in the event `meta` field

#### CSV

CSV files must include a header row.

Example:

```csv
date,position,name
2026-01-01,N,New Year
2026-01-06,N,Epiphany
2026-04-25,N,Liberation Day
```

**Notes**:

- `position` must be a valid location code (`N`, `H`, `O`, `R`, `C`, `M`)
- name is optional

### Import command

```bash
rtimelogger import --file <path> [options]
```

**Options**

- `--file <path>` : Path to the input file (required)

- `--format <json|csv>` : Input format (default: json)

- `--dry-run` : Simulate the import without modifying the database (strongly recommended)

- `--replace` : Replace existing events for conflicting dates (dangerous)

- `--source <label>` : Logical label describing the origin of imported data. The final stored value will include the
  format automatically (e.g. import (from json))

### Import behavior

- Only Holiday and NationalHoliday positions are accepted by default.
- Dates with existing work events are skipped unless --replace is used.
- Imported holidays:
    - do **not** affect the vacation balance
    - are treated as regular timeline entries

- Each import generates a detailed summary:
    - total rows
    - imported
    - skipped
    - conflicts
    - invalid rows

### Example (dry-run)

```bash
rtimelogger import \
  --file holidays_2026.json \
  --format json \
  --dry-run
```

### Example (apply import)

```bash
rtimelogger import \
  --file holidays_2026.csv \
  --format csv \
  --source calendar
```

### Metadata and traceability

- Imported events store additional information in the meta field (JSON).
- The source field tracks the origin of the data:
    - CLI entries → cli
    - Imports → import (from json) / import (from csv)

This ensures full traceability of all events.

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
