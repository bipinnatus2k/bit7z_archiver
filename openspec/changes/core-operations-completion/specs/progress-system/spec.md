## ADDED Requirements

### Requirement: Progress for all long-running operations
The system SHALL display a progress window during extract, compress, add, delete, and test operations.

#### Scenario: Dual progress bars
- **WHEN** an operation is in progress
- **THEN** the top bar shows per-file bytes progress (resets per file), the bottom bar shows overall items-done/total with aggregate bytes

#### Scenario: Current file display
- **WHEN** processing multiple files
- **THEN** the progress window shows the name of the file currently being processed

### Requirement: Pause and Resume
The system SHALL allow pausing and resuming long operations.

#### Scenario: Pause extraction
- **WHEN** user clicks Pause during extraction
- **THEN** the worker thread suspends, bars freeze, button changes to Resume

#### Scenario: Resume extraction
- **WHEN** user clicks Resume after pausing
- **THEN** the operation continues from where it paused

### Requirement: Hide to system tray
The system SHALL allow minimizing the progress window to the system tray.

#### Scenario: Hide extraction to tray
- **WHEN** user clicks Hide during extraction
- **THEN** the progress window closes, operation continues in background, tray tooltip shows "bit7z — Extracting 42%"

#### Scenario: Restore from tray
- **WHEN** user left-clicks the tray icon during an active operation
- **THEN** the progress window reopens showing current progress

### Requirement: Completion notification
The system SHALL notify the user when a background operation completes.

#### Scenario: Extraction completes while minimized
- **WHEN** an extraction finishes while the progress window is hidden to tray
- **THEN** a notification/toast shows "Extraction complete — 156 files extracted"

### Requirement: Error display
The system SHALL show error information in the progress window.

#### Scenario: Fatal error during compression
- **WHEN** a compression operation encounters a fatal error
- **THEN** the progress window shows the error message and a Close button
