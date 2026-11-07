import XCTest

@testable import torrenter

final class FormatTests: XCTestCase {
    func testFormatsZeroAndKnownSizes() {
        XCTAssertEqual(formatSize(0), "0.00 B")
        XCTAssertEqual(formatSize(512), "512.00 B")
        XCTAssertEqual(formatSize(1024), "1.00 KB")
        XCTAssertEqual(formatSize(1024 * 1024), "1.000 MB")
        XCTAssertEqual(formatSize(1024 * 1024 * 1024), "1.0000 GB")
    }

    func testFormatsSpeedAndEtaUnits() {
        XCTAssertEqual(formatSpeed(0), "0.00 B/s")
        XCTAssertEqual(formatSpeed(.nan), "0.00 B/s")
        XCTAssertEqual(formatSpeed(1024), "1.00 KB/s")
        XCTAssertEqual(formatEta(nil), "N/A")
        XCTAssertEqual(formatEta(1), "1 second")
        XCTAssertEqual(formatEta(2), "2 seconds")
        XCTAssertEqual(formatEta(60), "1 minute")
        XCTAssertEqual(formatEta(3600), "1 hour")
        XCTAssertEqual(formatEta(86_400), "1 day")
    }
}
