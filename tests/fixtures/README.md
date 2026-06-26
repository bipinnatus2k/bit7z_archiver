# Test Fixtures

This directory holds archive files used by integration tests.

## Required Fixtures

| File             | Contents                                              |
|------------------|-------------------------------------------------------|
| `basic.7z`       | 3 text files + 1 subdirectory (7z format)             |
| `basic.zip`      | Same content in Zip format                            |
| `basic.tar`      | Same content in Tar format                            |
| `corrupted.7z`   | 1 good entry + 1 corrupted entry                      |
| `encrypted.7z`   | Password-protected 7z with encrypted filenames        |
| `empty.7z`       | Empty archive (0 entries)                             |
| `multi_file.7z`  | 4 files of varying sizes (a.txt, b.txt, c.txt, large.txt) |

## Creating Fixtures

Fixtures can be created manually using 7-Zip:

```powershell
# Create empty.7z
7z a -mx0 tests/fixtures/empty.7z

# Create basic.7z with sample content
7z a -mx0 tests/fixtures/basic.7z f1.txt f2.txt sub/

# Create basic.zip
7z a -mx0 -tzip tests/fixtures/basic.zip f1.txt f2.txt sub/

# Create basic.tar
7z a -mx0 -ttar tests/fixtures/basic.tar f1.txt f2.txt sub/

# Create encrypted.7z (password: secret123, encrypted headers)
7z a -mx0 -psecret123 -mhe=on tests/fixtures/encrypted.7z f1.txt

# Create multi_file.7z
7z a -mx0 tests/fixtures/multi_file.7z a.txt b.txt c.txt large.txt

# Create corrupted.7z (modify bytes after creation)
7z a -mx0 tests/fixtures/corrupted.7z good.txt bad.txt
# Then corrupt a data byte in the archive
```

## Notes

- Tests that require real archive files check fixture availability with `ensure_fixtures()`.
- The `tests/common/mod.rs` module provides helpers to check fixture availability.
- The bit7z library (`C:\Program Files\7-Zip\7z.dll`) must be available at runtime.
