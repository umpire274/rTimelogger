<h1 style={text-align:"left"}>
  <img src="res/rtimelogger.svg" width="90" alt="rTimelogger logo" />
  rTimelogger
</h1>

[![Build Status](https://github.com/umpire274/rTimelogger/actions/workflows/ci.yml/badge.svg)](https://github.com/umpire274/rTimelogger/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/umpire274/rTimelogger)](https://github.com/umpire274/rTimelogger/releases)
[![codecov](https://codecov.io/gh/umpire274/rTimelogger/graph/badge.svg)](https://codecov.io/gh/umpire274/rTimelogger)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**rTimelogger** is a cross-platform command-line application for recording working time with IN/OUT events, tracking
work locations and protected absences, and calculating expected time, daily balance, and aggregated statistics. Data is
stored locally in SQLite.

## What's new in v0.9.1

Version 0.9.1 improves the statistics introduced in v0.9.0:

- day counts for every category: `O`, `R`, `C`, `M`, `H`, `S`, and `N`;
- automatic `Mixed` classification when a date contains multiple working locations;
- styled Unicode tables for time totals and day distribution;
- clearer `Paid Leave`, `Sick Leave`, and `National Holiday` terminology;
- distinct colors, title backgrounds, and highlighted total rows;
- `month` as the default value for `stats --group-by`;
- no database migration required.

See [CHANGELOG.md](CHANGELOG.md) for the complete release history.

## Features

- Multiple IN/OUT pairs per day
- Office, remote, on-site, and mixed work locations
- Paid leave, sick leave, and national holiday markers
- Configurable work duration and lunch rules
- Working and non-working gaps between pairs
- Expected exit time and daily balance
- Weekly, monthly, and yearly statistics
- Day counts by work-location category
- Free-text notes attached to work pairs
- Detailed and compact terminal views
- CSV, JSON, XLSX, and PDF exports
- JSON and CSV calendar-day imports
- SQLite migrations, integrity checks, and backups
- Internal audit log
- Linux, macOS, and Windows support

## Installation

### Cargo

```bash
cargo install rtimelogger
```

### Arch Linux (AUR)

```bash
yay -S rtimelogger
# or
paru -S rtimelogger
```

### Homebrew (macOS and Linux)

```bash
brew tap umpire274/tap
brew install rtimelogger
```

### Debian and Ubuntu

Download the `.deb` package from the GitHub Releases page, then run:

```bash
sudo dpkg -i rtimelogger_<version>_amd64.deb
sudo apt --fix-broken install
```

When checksum and signature files are provided, verify the package before installation:

```bash
sha256sum -c rtimelogger_<version>_amd64.deb.sha256
gpg --verify rtimelogger_<version>_amd64.deb.sig
```

### Other Linux distributions

```bash
tar -xvf rtimelogger-<version>-x86_64-unknown-linux-gnu.tar.gz
sudo mv rtimelogger /usr/local/bin/
```

### macOS release archives

Intel:

```bash
tar -xvf rtimelogger-<version>-x86_64-apple-darwin.tar.gz
sudo mv rtimelogger /usr/local/bin/
```

Apple Silicon:

```bash
tar -xvf rtimelogger-<version>-aarch64-apple-darwin.tar.gz
sudo mv rtimelogger /usr/local/bin/
```

### Windows

Download and extract the release ZIP, then place `rtimelogger.exe` in a dedicated directory included in your user or
system `PATH`.

## Quick start

Initialize the configuration and database:

```bash
rtimelogger init
```

Record a complete working day:

```bash
rtimelogger add 2026-09-04 --pos O --in 09:00 --lunch 30 --out 17:30
```

Show the current month:

```bash
rtimelogger list
```

Show monthly statistics for a date range:

```bash
rtimelogger stats --from 2026-01-01 --to 2026-12-31
```

## Configuration

The default configuration file is created by `rtimelogger init`.

Example:

```yaml
database: /home/user/.rtimelogger/rtimelogger.sqlite
default_position: O
min_work_duration: 8h
lunch_window: 12:30-14:00
min_duration_lunch_break: 30
max_duration_lunch_break: 90
separator_char: "-"
show_weekday: None
```

Supported `show_weekday` values are `None`, `Short`, `Medium`, and `Long`.

Inspect or edit the configuration:

```bash
rtimelogger config --print
rtimelogger config --check
rtimelogger config --edit
rtimelogger config --migrate
```

Specify a different editor when required:

```bash
rtimelogger config --edit --editor vim
```

Override the database for any command:

```bash
rtimelogger --db /custom/path/rtimelogger.sqlite list
```

On Windows:

```powershell
rtimelogger.exe --db C:\Data\rtimelogger.sqlite list
```

## Position codes

Each date is classified using one of the following codes:

| Code | Name             | Meaning                                     |
|------|------------------|---------------------------------------------|
| `O`  | Office           | Work performed from the office              |
| `R`  | Remote           | Work performed remotely                     |
| `C`  | On-site          | Work performed at a customer site           |
| `M`  | Mixed            | Multiple work locations during the same day |
| `H`  | Paid Leave       | Personal paid leave                         |
| `S`  | Sick Leave       | Contractually protected sick leave          |
| `N`  | National Holiday | Public or national holiday                  |

`O`, `R`, `C`, and `M` represent worked days. `H`, `S`, and `N` are marker days and do not require IN/OUT times.

## Commands

| Command  | Description                             |
|----------|-----------------------------------------|
| `init`   | Initialize configuration and database   |
| `add`    | Add or edit events, markers, and notes  |
| `list`   | Display daily sessions or raw events    |
| `stats`  | Display aggregated work statistics      |
| `del`    | Delete a day or a selected pair         |
| `backup` | Create a database backup                |
| `export` | Export stored sessions                  |
| `import` | Import calendar days from JSON or CSV   |
| `db`     | Run database maintenance commands       |
| `config` | Inspect, edit, or migrate configuration |
| `log`    | Display the internal audit log          |

Run `rtimelogger <command> --help` for the complete option list.

## Adding and editing data

### Working sessions

Add IN and OUT events together or separately:

```bash
rtimelogger add 2026-09-04 --pos O --in 09:00 --lunch 30 --out 17:30
rtimelogger add 2026-09-04 --pos O --in 09:00
rtimelogger add 2026-09-04 --out 17:30
```

Use lowercase or uppercase position codes:

```bash
rtimelogger add 2026-09-04 --pos r --in 09:00 --out 17:30
```

### Multiple pairs and gaps

A date can contain multiple IN/OUT pairs. A gap normally represents non-working time. Mark the preceding OUT event with
`--work-gap` when the interval before the next IN must count as worked time:

```bash
rtimelogger add 2026-09-04 --pos O --in 09:00 --out 12:00 --work-gap
rtimelogger add 2026-09-04 --pos R --in 13:00 --out 17:30
```

Edit the flag later with `--work-gap` or `--no-work-gap`:

```bash
rtimelogger add 2026-09-04 --edit --pair 1 --work-gap
rtimelogger add 2026-09-04 --edit --pair 1 --no-work-gap
```

When a date contains more than one working location, statistics classify it as `M` (Mixed) and count the date only once.

### Editing a pair

```bash
rtimelogger add 2026-09-04 --edit --pair 1 --in 08:45 --out 17:45
```

When `--pair` is omitted in edit mode, rTimelogger edits the last available pair:

```bash
rtimelogger add 2026-09-04 --edit --out 18:00
```

### Notes

Attach a note when adding or editing a pair:

```bash
rtimelogger add 2026-09-04 --in 09:00 --notes "Planning and backlog review"
rtimelogger add 2026-09-04 --edit --pair 1 --notes "Extended debugging session"
rtimelogger add 2026-09-04 --edit --notes "End-of-day summary"
```

Notes do not affect time calculations and appear only in detailed output.

### Paid leave

```bash
rtimelogger add 2026-09-07 --pos H
```

Do not provide `--in`, `--out`, `--lunch`, or gap options for a marker day.

### Sick leave

Single date:

```bash
rtimelogger add 2026-09-08 --pos S
```

Date range:

```bash
rtimelogger add 2026-09-08 --pos S --to 2026-09-11
```

For sick-leave ranges, weekends, national holidays, and dates that already contain events are skipped.

### National holidays

```bash
rtimelogger add 2026-12-25 --pos N
```

Paid leave, sick leave, and national holidays are neutral in daily `list` balances. In aggregated statistics, they
fulfil the contractual time expected for the corresponding weekday.

## Listing sessions

Show the current month:

```bash
rtimelogger list
```

Select a year, month, date, custom range, or all stored data:

```bash
rtimelogger list --period 2026
rtimelogger list --period 2026-09
rtimelogger list --period 2026-09-04
rtimelogger list --period 2026-09-01:2026-09-30
rtimelogger list --period all
```

Show today:

```bash
rtimelogger list --today
```

Filter by position:

```bash
rtimelogger list --period 2026-09 --pos R
```

### Detailed view

Detailed output shows individual pairs, lunch, position, work-gap state, and notes:

```bash
rtimelogger list --today --details
rtimelogger list --period 2026-09-04 --details
```

`--details` is valid only with `--today` or a single-date `--period`.

### Compact view

```bash
rtimelogger list --period 2026-09 --compact
```

`--compact` cannot be combined with `--details`.

### Raw events

```bash
rtimelogger list --period 2026-09-04 --events
rtimelogger list --period 2026-09-04 --events --pairs 2
```

## Work statistics

The `stats` command reports an inclusive date range:

```bash
rtimelogger stats --from 2026-01-01 --to 2026-12-31
```

The default grouping is `month`. Override it with:

```bash
rtimelogger stats --from 2026-08-01 --to 2026-09-30 --group-by week
rtimelogger stats --from 2026-01-01 --to 2026-12-31 --group-by month
rtimelogger stats --from 2024-01-01 --to 2026-12-31 --group-by year
```

Accepted values are:

| Value   | Grouping rule                   |
|---------|---------------------------------|
| `week`  | ISO week, Monday through Sunday |
| `month` | Calendar month; default         |
| `year`  | Calendar year                   |

The first and last group may be partial when the requested dates fall inside a week, month, or year.

### Time statistics

The first table displays:

- expected contractual time;
- actual worked time;
- paid-leave time (`H`);
- sick-leave time (`S`);
- national-holiday time (`N`);
- recognized time;
- balance.

Working gaps count as worked time; non-working gaps do not. Lunch is not part of contractual working time.

```text
recognized = worked + paid leave + sick leave + national holiday
balance    = recognized - expected
```

Weekdays without a recorded event remain in the requested period: they contribute expected time but no recognized time,
producing the corresponding deficit.

### Day distribution

The second table reports how many dates belong to each category:

```text
O = Office
R = Remote
C = On-site
M = Mixed
H = Paid Leave
S = Sick Leave
N = National Holiday
```

Each date belongs to exactly one category. A working date containing different locations is classified as
`M`. Dates without records are not assigned to a category.

```text
worked_days = O + R + C + M
total_days  = O + R + C + M + H + S + N
```

`Avg/day` is actual worked time divided by effectively worked days (`O + R + C + M`). Protected absences and national
holidays do not lower this average.

## Deleting data

Delete all events for a date:

```bash
rtimelogger del 2026-09-04
```

Delete a selected pair:

```bash
rtimelogger del 2026-09-04 --pair 2
```

Deletion requires confirmation and pair identifiers are recalculated afterward.

## Backups

```bash
rtimelogger backup --file /absolute/path/rtimelogger.sqlite
rtimelogger backup --file /absolute/path/rtimelogger-backup --compress
```

Compressed backups use ZIP on Windows and TAR.GZ on Linux and macOS.

## Exporting data

```bash
rtimelogger export \
  --format xlsx \
  --file /absolute/path/rtimelogger.xlsx \
  --range 2026-09
```

Supported formats:

- `csv`
- `json`
- `xlsx`
- `pdf`

Additional options:

- `--events` exports individual events;
- `--force` overwrites an existing output file;
- `--range` accepts the same period formats used by reporting commands.

The output path must be absolute.

The `export` command currently exports session data. Statistics-specific CSV and XLSX exports are planned for a later
release.

## Importing calendar days

Import national holidays or paid-leave markers from JSON or CSV:

```bash
rtimelogger import --file holidays.json --format json --dry-run
rtimelogger import --file holidays.csv --format csv --source calendar
```

Options:

| Option                 | Meaning                                 |
|------------------------|-----------------------------------------|
| `--file <FILE>`        | Input file; required                    |
| `--format <json\|csv>` | Input format; default `json`            |
| `--dry-run`            | Validate and preview without writing    |
| `--replace`            | Replace conflicting dates               |
| `--source <LABEL>`     | Store an origin label; default `import` |

Use `--dry-run` before importing production data. By default, existing work dates are preserved and conflicts are
skipped unless `--replace` is supplied.

### JSON examples

Root object with `holidays`:

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

Root object with `days`:

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

Root array:

```json
[
  {
    "date": "2026-12-25",
    "name": "Christmas Day"
  }
]
```

When `position` is omitted, it defaults to `N`. The optional `name` is stored as event metadata.

### CSV example

```csv
date,position,name
2026-01-01,N,New Year
2026-01-06,N,Epiphany
2026-04-25,N,Liberation Day
```

Imported records preserve source and format metadata for traceability.

## Database maintenance

```bash
rtimelogger db --info
rtimelogger db --check
rtimelogger db --vacuum
rtimelogger db --migrate
```

Database migrations preserve existing data and create automatic backups when required by the migration.

## Audit log

```bash
rtimelogger log --print
```

The internal log records operations such as additions, deletions, migrations, and backups.

## Validation for contributors

Run the complete project checks before opening a pull request:

```powershell
.\dev_tools\build_check.ps1
```

Or run the Rust checks individually:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Upgrading

Users upgrading from version 0.7.x or earlier should read
[UPGRADE-0.7-to-0.8.md](UPGRADE-0.7-to-0.8.md) before running database migrations.

No database migration is required when upgrading from v0.9.0 to v0.9.1.

## Documentation

- [CHANGELOG.md](CHANGELOG.md)
- [Upgrade guide from 0.7 to 0.8](UPGRADE-0.7-to-0.8.md)

## License

Released under the MIT License. See [LICENSE](LICENSE).
