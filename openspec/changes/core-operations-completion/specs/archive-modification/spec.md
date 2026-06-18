## ADDED Requirements

### Requirement: User can add files to an existing archive
The system SHALL allow adding files or folders to an already-opened writable archive.

#### Scenario: Add a file to an open archive
- **WHEN** user clicks Add, selects a file via the OS file picker, and confirms
- **THEN** the file is compressed and added to the archive, and the entry list refreshes to show the new entry

#### Scenario: Add a folder with wildcard filter
- **WHEN** user clicks Add, picks a folder, enters a wildcard filter (e.g., `*.pdf;*.txt`), selects Include or Exclude policy, chooses recursive or not, and confirms
- **THEN** only matching files from the folder are added to the archive

#### Scenario: Add files with custom archive path prefix
- **WHEN** user enters a path prefix (e.g., `docs/`) and adds files
- **THEN** files appear under `docs/<filename>` inside the archive

#### Scenario: Add files with compression overrides
- **WHEN** user selects a compression method other than the archive default and confirms
- **THEN** added files are compressed with the selected method and level

### Requirement: User can delete entries from an archive
The system SHALL allow deleting one or more selected entries from an open writable archive.

#### Scenario: Delete selected entries
- **WHEN** user selects entries, clicks Delete (or presses Del), and confirms
- **THEN** the entries are removed and the entry list refreshes

#### Scenario: Delete with progress
- **WHEN** deleting a large number of entries
- **THEN** a progress window shows entries deleted / total

### Requirement: User can rename an entry within an archive
The system SHALL allow renaming a single selected entry within an open writable archive.

#### Scenario: Rename an entry
- **WHEN** user selects one entry, clicks Rename (or presses F2), types a new name, and confirms
- **THEN** the entry is renamed and the entry list refreshes

#### Scenario: Rename preserving directory
- **WHEN** user renames `subdir/file.txt` to `newname.txt`
- **THEN** the full path becomes `subdir/newname.txt`

### Requirement: Modification operations disabled for read-only formats
The system SHALL disable Add, Delete, and Rename when the archive is read-only (e.g., Rar).

#### Scenario: Open a Rar archive
- **WHEN** a Rar archive is opened
- **THEN** Add, Delete, and Rename are disabled with tooltip "Not supported for read-only archives"
