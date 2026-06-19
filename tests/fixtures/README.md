# Test Fixtures

This directory holds archive files used by integration tests.

## Required Fixtures

| File           | Contents                                    |
|----------------|---------------------------------------------|
| `basic.7z`     | 3 text files + 1 subdirectory               |
| `basic.zip`    | Same content in Zip format                  |
| `corrupted.7z` | 1 good entry + 1 CRC-mismatched entry       |
| `encrypted.7z` | Password-protected with encrypted filenames |
| `empty.7z`     | Empty archive (0 entries)                   |

## Creating Fixtures

Fixtures can be created manually using 7-Zip or by running the application in CLI mode:

```sh
# Create empty.7z
7z a -mx0 tests/fixtures/empty.7z

# Create basic.7z with sample content
echo "hello" > /tmp/f1.txt
echo "world" > /tmp/f2.txt
echo "subdir content" > /tmp/sub/f3.txt
7z a tests/fixtures/basic.7z /tmp/f1.txt /tmp/f2.txt /tmp/sub/

# Create encrypted.7z
7z a -psecret -mhe=on tests/fixtures/encrypted.7z /tmp/f1.txt

# Create basic.zip
zip tests/fixtures/basic.zip /tmp/f1.txt /tmp/f2.txt /tmp/sub/f3.txt
```

## Notes

- Tests that require real archive files are marked with `#[ignore]` or skip when fixtures are absent.
- The `tests/common/mod.rs` module provides helpers to check fixture availability.
