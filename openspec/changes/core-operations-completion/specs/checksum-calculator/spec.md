## ADDED Requirements

### Requirement: Checksum calculation on selected entries
The system SHALL compute and display CRC32, MD5, SHA1, and SHA256 checksums for selected entries.

#### Scenario: Single file checksum
- **WHEN** user selects one non-directory entry and clicks Tools → Checksum → SHA256
- **THEN** the entry is temp-extracted and its SHA256 hex string is displayed

#### Scenario: Multiple file checksum
- **WHEN** user selects multiple entries and requests checksum
- **THEN** each entry's hash is computed and displayed in a list

#### Scenario: Checksum from context menu
- **WHEN** user right-clicks entries and navigates Checksum ▶ CRC32
- **THEN** a popup window shows the CRC32 hex value for each selected file

#### Scenario: All four algorithms at once
- **WHEN** user requests checksum
- **THEN** the result popup shows all four hashes: CRC32, MD5, SHA1, SHA256 — each as a hex string

### Requirement: Checksum unavailable for directories
The system SHALL disable or hide checksum options when only directory entries are selected.

#### Scenario: Context menu with only directories selected
- **WHEN** user right-clicks a directory entry
- **THEN** the Checksum submenu is grayed out
