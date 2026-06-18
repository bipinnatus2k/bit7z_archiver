## ADDED Requirements

### Requirement: Archive Properties window
The system SHALL display a properties window showing metadata about the open archive.

#### Scenario: Open archive properties
- **WHEN** user clicks File → Properties (no entries selected) or presses Alt+Enter
- **THEN** a window opens showing General section (type, location, sizes, ratio, file/folder count, modified date) and Advanced section (solid, encrypted, encrypted names, multi-volume, comment, recovery record, locked, dictionary)

#### Scenario: Properties data sourced from FFI
- **WHEN** archive properties are requested
- **THEN** data comes from `get_archive_properties()` repository method, which reads from bit7z reader attributes and `bit7z_item_*` calls

### Requirement: Entry Properties — single entry
The system SHALL display categorized metadata for a single selected entry.

#### Scenario: Single entry properties
- **WHEN** user selects one entry and presses Alt+Enter
- **THEN** a window opens with categories: General (name, type, path, size, packed, ratio, CRC), Time (modified, created, accessed — hidden if not available per format), Platform (host_os, attributes, POSIX, owner, group — shown only when available), Security (encrypted), Technical (compression method, symlink, comment)

#### Scenario: Format-specific categories
- **WHEN** viewing properties of a Zip entry created by PKZip 2.04g
- **THEN** the Time category shows only Modified (created and accessed rows are hidden)
- **WHEN** viewing properties of a Tar entry
- **THEN** the Platform category shows POSIX attrib, owner (uid), group (gid)

### Requirement: Entry Properties — multiple entries
The system SHALL display aggregate totals for multiple selected entries.

#### Scenario: Multiple entry properties
- **WHEN** user selects 3 entries and presses Alt+Enter
- **THEN** window shows aggregate size/packed/files count + a collapsed "▶ Entries" list expandable to a DataTable (columns: Name, Size, Packed, Ratio, CRC, Modified, Encryption — auto-hides columns where no selected entry has the property)
