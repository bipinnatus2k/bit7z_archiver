## ADDED Requirements

### Requirement: Open entry with associated program
The system SHALL open a non-directory archive entry with the OS-associated program.

#### Scenario: Open a text file
- **WHEN** user double-clicks a `.txt` entry or selects it and presses Enter
- **THEN** the file is temp-extracted and opened with the default text editor

### Requirement: View entry inline
The system SHALL display file contents in the preview panel.

#### Scenario: View selected entry
- **WHEN** user selects a file and clicks View (Ctrl+V) or the preview panel auto-previews
- **THEN** the file contents are displayed (text for text files, hex for binary, image for images)

### Requirement: Edit entry
The system SHALL allow editing an archive entry by extracting to temp, opening in editor, and updating in archive.

#### Scenario: Edit and save back
- **WHEN** user selects a file and clicks Edit (F4)
- **THEN** file is temp-extracted, opened in associated editor, and when the editor closes, the system prompts "Update file in archive? [Yes] [No] [Cancel]"

#### Scenario: Edit a binary file
- **WHEN** user edits a binary file (no text editor association)
- **THEN** the system opens it with the OS-associated program for that file type

### Requirement: New Folder in archive
The system SHALL allow creating an empty directory within the archive.

#### Scenario: Create new folder
- **WHEN** user clicks File → New Folder or right-clicks empty space and clicks New Folder
- **THEN** a prompt asks for folder name, creates the directory entry in the archive, and refreshes the listing

### Requirement: New File in archive
The system SHALL allow creating a new empty file within the archive.

#### Scenario: Create new file
- **WHEN** user clicks File → New File or context menu → New File
- **THEN** a temp file is created, opened in editor, and on save added to the archive
