

/// Result of an archive integrity test.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub total: usize,
    pub passed: usize,
    pub failed: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub entry_path: String,
    pub error: String,
    pub index: usize,
    pub path: String,
    pub reason: TestFailureReason,
}

#[derive(Debug, Clone)]
pub enum TestFailureReason {
    CrcMismatch { expected: u32, actual: u32 },
    ReadError(String),
    UnsupportedOperation,
}