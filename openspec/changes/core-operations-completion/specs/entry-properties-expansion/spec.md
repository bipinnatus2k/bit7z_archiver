## MODIFIED Requirements

### Requirement: ArchiveEntry expanded with all available bit7z properties
The system SHALL populate all fields that bit7z can provide and add type-appropriate Option fields for format-specific properties.

#### Scenario: CRC populated
- **WHEN** listing archive entries
- **THEN** the `crc` field contains `Some(u32)` from `bit7z_item_crc()`

#### Scenario: Modified time populated
- **WHEN** listing archive entries
- **THEN** the `modified` field contains `Some(DateTime<Utc>)` from `bit7z_item_mtime()`

#### Scenario: Symlink detected
- **WHEN** an entry is a symlink
- **THEN** `is_symlink` is `true`

#### Scenario: Format-specific properties
- **WHEN** listing tar entries
- **THEN** `user`, `group`, `posix_attrib` fields are populated; `attributes` and `created` may be None
- **WHEN** listing 7z entries
- **THEN** `created`, `accessed`, `attributes`, `host_os`, `compression_method` fields are populated
- **WHEN** listing old Zip entries
- **THEN** `created` and `accessed` are None (format does not support them)

#### Scenario: All new fields are Option
- **WHEN** any property is not available for a given format
- **THEN** the corresponding field is `None` — no crashes, no defaults
