## ADDED Requirements

### Requirement: Menu bar with all standard menus
The system SHALL provide a menu bar above the toolbar with File, Edit, View, Tools, Favorites, and Help menus.

#### Scenario: File menu
- **WHEN** user clicks File
- **THEN** menu shows: Open Archive (Ctrl+O), Create Archive (Ctrl+N), Add Files, ─, Test Archive (Ctrl+T), ─, Open (Enter), View (Ctrl+V), Edit (F4), ─, New Folder (Ctrl+Shift+N), New File, ─, Close Archive, ─, Properties (Alt+Enter), ─, Exit

#### Scenario: Test Archive with selection
- **WHEN** entries are selected and user opens File → Test Archive
- **THEN** submenu shows "Test Selected Files" and "Test Entire Archive"

#### Scenario: Edit menu
- **WHEN** user clicks Edit
- **THEN** menu shows: Select All (Ctrl+A), Invert Selection, ─, Copy, Cut, Paste, ─, Delete (Del), Rename (F2)

#### Scenario: View menu
- **WHEN** user clicks View
- **THEN** menu shows: Large Icons, Small Icons, List, Details, ─, Flat View, ─, Show: Toolbar, Status Bar, Preview Panel, Directory Tree (all toggleable checkmarks)

#### Scenario: Tools menu
- **WHEN** user clicks Tools
- **THEN** menu shows: Checksum submenu (CRC32, MD5, SHA1, SHA256 — active when entries selected), ─, Settings

#### Scenario: Favorites menu
- **WHEN** user clicks Favorites
- **THEN** menu shows: Add to Favorites, Organize Favorites, ─, (list of recent archives)

### Requirement: Keyboard shortcuts
The system SHALL support keyboard shortcuts for all menu items.

| Shortcut | Action |
|---|---|
| Ctrl+O | Open Archive |
| Ctrl+N | Create Archive |
| Ctrl+T | Test Archive |
| Enter | Open selected entry |
| Ctrl+V | View selected entry |
| F4 | Edit selected entry |
| Ctrl+A | Select All |
| Del | Delete selected |
| F2 | Rename selected |
| F5 | Refresh |
| Alt+Enter | Properties |
| Ctrl+Shift+N | New Folder |

### Requirement: Menu actions dispatch same events as toolbar/context menu
The system SHALL ensure menu clicks, toolbar button clicks, and context menu clicks all dispatch identical events — no duplicate logic.
