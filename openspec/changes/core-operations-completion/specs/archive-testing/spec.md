## ADDED Requirements

### Requirement: User can test entire archive integrity
The system SHALL verify all entries in an archive for corruption.

#### Scenario: Test entire archive — all pass
- **WHEN** user clicks Test (no entries selected) or File → Test Archive
- **THEN** all entries are verified, and status bar shows "156 files tested, all passed"

#### Scenario: Test entire archive — some fail
- **WHEN** user tests an archive with corrupted entries
- **THEN** a Test Results window opens showing "154 passed, 2 failed" with collapsed failed list

### Requirement: User can test selected entries
The system SHALL allow testing specific entries including directories.

#### Scenario: Test selected files
- **WHEN** user selects specific entries and clicks Test Selected
- **THEN** only those entries are verified

#### Scenario: Test a directory recursively
- **WHEN** user selects a directory and tests it
- **THEN** all child entries within the directory are verified

### Requirement: Test progress reporting
The system SHALL display progress during long test operations.

#### Scenario: Test with progress
- **WHEN** testing a large archive
- **THEN** a progress window shows per-file progress bar and overall items-done/total bar

### Requirement: Test Results window
The system SHALL display results in a dedicated window showing failed entries collapsed by default.

#### Scenario: Expand failed list
- **WHEN** user clicks "▶ Failed entries (2)" in the results window
- **THEN** the list expands showing each failed entry with index, path, and reason (CRC mismatch, read error)
