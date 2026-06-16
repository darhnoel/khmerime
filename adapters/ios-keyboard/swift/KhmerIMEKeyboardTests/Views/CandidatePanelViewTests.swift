import XCTest
import UIKit
@testable import KhmerIMEKeyboard

final class CandidatePanelViewTests: XCTestCase {

    // MARK: - CharPick letter chip glass styling

    func test_charPickLetterChip_isGlassKeyButton() {
        let panel = CandidatePanelView(metrics: KeyboardLayoutMetrics(device: .phone))
        panel.render(makeRenderState(
            candidates: ["ក"],
            segments: [IosSegmentEntry(output: "ទៅ", input: "tow", focused: true)]
        ))

        XCTAssertNil(button("ទៅ", in: panel))
        XCTAssertNil(button("✏", in: panel))

        panel.renderCharPickAlphabet()

        guard let chip = button("A", in: panel) else {
            XCTFail("Expected to find an 'A' letter chip after renderCharPickAlphabet")
            return
        }
        XCTAssertNotNil(chip as? GlassKeyButton,
            "CharPick letter chips must be GlassKeyButton for consistent glass styling")
    }

    func test_charPickLetterChip_hasGlassBorder() {
        let panel = CandidatePanelView(metrics: KeyboardLayoutMetrics(device: .phone))
        panel.frame = CGRect(x: 0, y: 0, width: 400, height: 200)
        panel.renderCharPickAlphabet()
        panel.layoutIfNeeded()

        guard let chip = letterChip("A", in: panel) else {
            XCTFail("Expected to find an 'A' letter chip after renderCharPickAlphabet")
            return
        }
        XCTAssertGreaterThan(chip.layer.borderWidth, 0,
            "CharPick letter chip must have a visible border like all other glass keys")
    }

    func test_charPickLetterChip_titleAccessibleViaLegacySlot() {
        let panel = CandidatePanelView(metrics: KeyboardLayoutMetrics(device: .phone))
        panel.renderCharPickAlphabet()

        guard let chip = letterChip("A", in: panel) else {
            XCTFail("Expected to find an 'A' letter chip after renderCharPickAlphabet")
            return
        }
        XCTAssertEqual(chip.title(for: .normal), "A",
            "CharPick chip title must be readable via title(for:) so charPickLetterTapped can extract the letter")
    }

    // MARK: - Helpers

    /// Finds the first button in the panel hierarchy whose displayed title matches `letter`.
    /// Checks both UIButton.Configuration.title (pre-fix) and title(for: .normal) (post-fix).
    private func letterChip(_ letter: String, in panel: CandidatePanelView) -> UIButton? {
        button(letter, in: panel)
    }

    private func button(_ title: String, in panel: CandidatePanelView) -> UIButton? {
        allButtons(in: panel).first {
            $0.title(for: .normal) == title || $0.configuration?.title == title
        }
    }

    private func makeRenderState(
        candidates: [String],
        segments: [IosSegmentEntry] = []
    ) -> IosRenderState {
        IosRenderState(
            candidates: candidates,
            selectedIndex: nil,
            preedit: "",
            segments: segments,
            focusedSegmentIndex: nil,
            commitText: nil,
            segmentEditActive: false,
            segmentEditIndex: nil
        )
    }

    private func allButtons(in view: UIView) -> [UIButton] {
        var result: [UIButton] = []
        for sv in view.subviews {
            if let btn = sv as? UIButton { result.append(btn) }
            result += allButtons(in: sv)
        }
        return result
    }
}
