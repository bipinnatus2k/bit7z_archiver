## MODIFIED Requirements

### Requirement: Linux tray via D-Bus
The system SHALL replace the Linux tray stub with a real implementation using the `zbus` crate.

#### Scenario: Tray icon appears on Linux
- **WHEN** the application starts on Linux
- **THEN** a tray icon appears via D-Bus StatusNotifierItem

#### Scenario: Tray tooltip shows progress
- **WHEN** a compress/extract/test operation is running and minimized to tray
- **THEN** the tray icon tooltip reads "bit7z — Extracting 42%"

#### Scenario: Tray context menu
- **WHEN** user right-clicks the Linux tray icon
- **THEN** a context menu shows "Open" and "Exit"

#### Scenario: Restore window from tray
- **WHEN** user left-clicks the tray icon or selects "Open" from context menu
- **THEN** the main window is restored (or progress window if operation is active)

### Requirement: Linux native file dialogs
The system SHALL replace `#[cfg(not(windows))]` stubs with real file dialogs using the `rfd` crate.

#### Scenario: Pick archive file
- **WHEN** user clicks Open Archive on Linux
- **THEN** a native file dialog opens with filter for supported archive formats

#### Scenario: Pick folder
- **WHEN** user clicks Browse in the Extract dialog on Linux
- **THEN** a native folder picker opens

#### Scenario: Multi-select add files
- **WHEN** user clicks Add Files on Linux
- **THEN** a multi-select file dialog opens
