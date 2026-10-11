# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Entries are generated from [Conventional Commits](https://www.conventionalcommits.org/) by [git-cliff](https://git-cliff.org).

## [0.4.0] - 2026-10-11

### Added

- **timesheet:** Autocomplete Azure DevOps tickets in descriptions

## [0.3.0] - 2026-10-10

### Added

- Mark days off and Belgian public holidays
- Add dark mode with System, Light and Dark appearance
- **ui:** Style settings dropdowns like text fields
- **ui:** Draw hour units in muted sans next to mono figures
- **ui:** Group settings into sections and shorten the data path
- **ui:** Add the Week clock app icon
- **ui:** Show the app icon in the top bar
- **ui:** Pick a line's task type from a compact menu
- **ui:** Step hours with arrow keys
- **ui:** Add keyboard shortcuts for daily entry
- **ui:** Open settings with Cmd+,

### Fixed

- **ui:** Center signed file icon in its row
- **ui:** Rename Timesheet tab to This month
- **ui:** Tab through a line in visual order
- **ui:** Pick a task type with F/B/S while its menu is open

### Changed

- Split model, pdf and ui into module directories

### Documentation

- Add hexagonal architecture notes
- Add README screenshots rendered from test data
- Add AGENTS.md with project guidance for agents
- Link the design system artifact in AGENTS.md
- Add the keyboard shortcuts screenshot
- Move keyboard shortcuts below screenshots in README

## [0.2.0] - 2026-10-09

### Added

- Export and import all data as one JSON file

### Fixed

- Round hour totals so decimal sums reach exactly 8 h
- Never overwrite a data file that could not be read
- **export:** Break the page inside a day with many lines

## [0.1.0] - 2026-10-09

First release of Timesheet, a native desktop app for monthly timesheet entry on a single project.
Available for macOS (universal DMG, Apple silicon and Intel) and Windows (x64 EXE).

### Added

- **Monthly timesheet.** Browse months and weeks, with one card per weekday. Each week shows its
  progress (Empty, In progress, Done, Over) and the month shows its total against the target hours.
- **Time entry.** Add lines with a task type (Frontend, Backend or Support), a description and hours.
  A new line proposes the last task type used and the hours left in the day. Hours accept a comma
  or a dot as the decimal separator.
- **Copy a day.** Fill an empty day with the lines of the previous filled day in one click.
- **8-hour limit.** Days over 8 h are flagged, and a month with such a day cannot be exported until
  it is fixed.
- **PDF exports.** Two reports per month, with signature blocks:
  - *Detailed report*: every line, grouped by week.
  - *Daily summary*: hours per task type for each weekday.
- **Past months.** Every closed month is listed with its status. Export both PDFs, tick when they are
  sent to and approved by your manager (with dates), and attach the signed PDF. A month waiting
  more than 5 days for approval is flagged for follow-up.
- **Settings.** Set the consultant, client and project names (click the avatar). They appear in the
  app header and on both PDFs.
- **Automatic saving.** Data is saved as JSON after each change. An unreadable data file is kept
  as `timesheet.corrupt-<timestamp>.json` and never overwritten.
- **Releases.** CI builds and tests on macOS and Windows, and publishes the DMG and EXE to a GitHub
  release for each version tag.

### Known limitations

- The app is not code-signed. On macOS, open it the first time with right-click › **Open**, or
  System Settings › Privacy & Security › **Open Anyway**. On Windows, SmartScreen may ask you to
  confirm with **More info** › **Run anyway**.

