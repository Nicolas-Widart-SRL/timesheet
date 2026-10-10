# Keyboard shortcuts (plan)

Status: implemented on `feat/keyboard-shortcuts`. Open questions 1 and 2 use the suggested keys
(`Cmd+J`, `Cmd+/`); plain-key duplicates (question 3) are not added.

Goal: enter a day's time without the mouse. Jump to a day, add a line, type the description, type the
hours, then move on.

`Cmd` means Command on macOS and Ctrl on Windows (`egui::Modifiers::COMMAND`). The approved shortcuts
use `Cmd`, so they work while a text field has focus. They do not use plain keys: plain digits clash
with the hours field and with descriptions such as "ticket #1234".

## Approved shortcuts

### Daily entry (build first)

| Key | Action |
| --- | --- |
| `Cmd+1` … `Cmd+5` | Jump to Monday … Friday of the shown week. |
| `Cmd+Enter` | Add a line to the current day, with the suggested type and remaining hours, and focus its description. |
| `Enter` in a description | Focus that line's hours. |
| `Enter` in hours | Focus the next line's description. On the last line: add a line if the day is under 8 h, or jump to the next day if the day is full. |
| `Cmd+D` | Copy the previous filled day into the current day, if the current day is empty. Same as the "Copy Mon 5" button. |
| `Cmd+T` | Go to today: open its month and week, then jump to the day. |
| `Esc` | Leave the focused field. egui already does this. |

### Navigation

| Key | Action |
| --- | --- |
| `Cmd+[` / `Cmd+]` | Previous / next week. |
| `Cmd+Shift+[` / `Cmd+Shift+]` | Previous / next month. |
| Next open day | Same as the sidebar "Next open day" button. Key not decided, see open questions. |
| `F` / `B` / `S` on a focused task-type button | Set Frontend / Backend / Support. No modifier needed: the button is not a text field. |
| Shortcut list | Show an overlay that lists every shortcut. Key not decided, see open questions. |

## Behavior rules

- **Number keys map to weekdays, not to card positions.** `Cmd+1` is always Monday and `Cmd+5` is
  always Friday. A weekday outside the month does nothing. For example, October 2026 starts on a
  Thursday, so in week 1 only `Cmd+4` and `Cmd+5` do something.
- **Jumping to a day** focuses the description of its first line. On an empty day, it focuses the
  "Add line" button, so `Enter` adds a line. On a day off, it focuses "Mark as workday".
- **"Current day"** for `Cmd+Enter` and `Cmd+D` is the day that holds the focused widget. If nothing has
  focus, use today when it is in the shown week, else the first open day of the shown week.
- **Popups:** shortcuts do nothing while the task-type menu or the "Day off…" menu is open.
- **Other screens:** the day shortcuts only apply on the Timesheet screen. They do nothing in
  Past months and Settings.

## Not adding

- **Remove line:** on macOS, `Cmd+Backspace` deletes to the start of the line in a text field.
  Tab to the × button instead.
- **Day off and export:** used at most once a month.
- **`Alt+letter`:** on macOS it types characters such as ƒ or ß in text fields.
- **`Cmd+←` / `Cmd+→`:** on macOS they move the cursor to the start or end of a text field.

## Implementation notes

- Read all shortcuts in one place in `App::timesheet` (`src/ui/timesheet.rs`), before the day cards are
  drawn. Use `consume_shortcut` / `consume_key` so a key is handled only once.
- Focusing a widget needs a stable id. The hours field already has one
  (`ui.make_persistent_id(("hours", entry.id))`). Make the description `TextEdit` and the
  "Add line" button work the same way: today the description uses `id_salt`, which is not enough.
- Focus a line that was just added on the next frame, because its widget does not exist yet. Store a
  pending focus target on `App`.
- The `Enter` flow: egui's single-line `TextEdit` drops focus on `Enter`. Check
  `response.lost_focus() && i.key_pressed(Key::Enter)`, then move the focus.
- Task-type keys: handle `F` / `B` / `S` in `kind_menu` (`src/ui/widgets.rs`) while its response has
  focus.
- Put any new rules in `src/model/` (for example "next day after a full day" or "current day when
  nothing is focused"), not in the UI.
- Discoverability: add each shortcut to the tooltip of the matching button, for example
  "Next month (⌘])". Add a "Keyboard shortcuts" section to `README.md`.
- Tests: drive the keys through the egui test harness, as `src/ui/screenshots.rs` already does with
  `h.key_press(...)`. Refresh the screenshots if the UI changes visibly.

## Open questions

1. **Next open day:** the proposal used a plain `N`. `Cmd+N` usually means "New". Suggested key:
   `Cmd+J` ("jump").
2. **Shortcut list:** the proposal used a plain `?`. Suggested key: `Cmd+/`, which many apps use for
   this.
3. **Plain-key duplicates:** this plan drops plain `1`–`5`, `T`, `[` and `]`, which would only work
   when no field has focus. Add them back if wanted.
