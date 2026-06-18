## ADDED Requirements

### Requirement: Compress CLI fully functional
The system SHALL replace the placeholder Compress CLI with full functionality.

#### Scenario: Compress files to archive
- **WHEN** user runs `bit7z_archiver compress file1.txt file2.pdf --to output.7z`
- **THEN** the files are added to a new 7z archive at output.7z

#### Scenario: Compress with format selection
- **WHEN** user runs `compress --to output.zip --format zip file1.txt`
- **THEN** a Zip archive is created instead of 7z (default)

#### Scenario: Compress with password
- **WHEN** user runs `compress --to output.7z --password secret file1.txt`
- **THEN** the archive is created with AES-256 encryption

#### Scenario: Compress directory recursively
- **WHEN** user runs `compress --to backup.7z myfolder/`
- **THEN** all files in myfolder/ (recursively) are added to the archive

#### Scenario: Print summary
- **WHEN** compression completes
- **THEN** output shows: "Created backup.7z — 42 files, 14.2 MB → 9.8 MB (69% ratio)"

### Requirement: Additional CLI subcommands
The system SHALL add list, checksum, and new-folder CLI commands.

#### Scenario: List archive contents
- **WHEN** user runs `bit7z_archiver list archive.7z`
- **THEN** a table of entries is printed to stdout with columns: Index, Name, Size, Packed, Ratio, Modified

#### Scenario: Compute checksum via CLI
- **WHEN** user runs `checksum archive.7z --sha256 3`
- **THEN** entry index 3 is temp-extracted and its SHA256 hex digest is printed

#### Scenario: Create folder via CLI
- **WHEN** user runs `new-folder archive.7z subdir/nested`
- **THEN** the directory path is created inside the archive
