## MODIFIED Requirements

### Requirement: Create dialog has interactive controls
The system SHALL replace static text placeholders in the Create dialog with real GPUI controls.

#### Scenario: Format selection
- **WHEN** user opens the Create dialog
- **THEN** a PickList dropdown shows writable formats: 7z, Zip, Tar, Tar.gz, Tar.bz2, Tar.xz

#### Scenario: Compression level slider
- **WHEN** user adjusts compression level
- **THEN** a slider with 6 positions: None, Fastest, Fast, Normal, Max, Ultra — displayed with labels

#### Scenario: Advanced section collapsed by default
- **WHEN** user opens the Create dialog
- **THEN** an "Advanced" section is collapsed. Expanding it shows: compression method (format-dependent options), dictionary size (for LZMA/LZMA2), word size, solid archive checkbox (7z only), volume size, thread count

### Requirement: Extract dialog has destination browse and overwrite mode
The system SHALL add missing controls to the Extract dialog.

#### Scenario: Browse for destination
- **WHEN** user clicks the Browse button next to the destination field
- **THEN** the OS folder picker opens and writes the selected path to the field

#### Scenario: Overwrite mode selection
- **WHEN** user expands overwrite options
- **THEN** a dropdown shows: Ask (prompt per file), Overwrite (silently replace), Skip (don't extract existing), Rename extracted (auto-rename to avoid conflict)

#### Scenario: Keep broken files
- **WHEN** user checks "Keep broken files"
- **THEN** extraction continues even when a file's CRC check fails

### Requirement: Add Files dialog matches bit7z capabilities
The system SHALL provide a comprehensive Add Files dialog with all writer settings bit7z supports.

#### Scenario: Format-dependent controls
- **WHEN** user selects 7z format in the Add Files dialog
- **THEN** all controls are shown: solid checkbox, encrypt filenames checkbox, dictionary size, word size, all compression methods
- **WHEN** user selects Zip format
- **THEN** solid and encrypt-filenames are hidden, methods show Deflate/Deflate64/BZip2/LZMA/Ppmd/Copy
- **WHEN** user selects Tar format
- **THEN** compression level, method, dictionary, solid, volumes, and encryption are all hidden

#### Scenario: File filter with exclude policy
- **WHEN** user adds a folder and sets filter to "*.tmp" with policy "Exclude"
- **THEN** all files EXCEPT .tmp files are added from the chosen directory
