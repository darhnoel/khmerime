import Foundation

enum StripPresentationSpec {

    static func romanRowText(state: IosRenderState, romanBuffer: String) -> String {
        if state.segmentEditActive {
            return editModeText(state)
        } else if state.segments.isEmpty {
            return romanBuffer
        } else {
            return state.segments.map { $0.input }.joined(separator: " · ")
        }
    }

    static func segmentKhmerTexts(state: IosRenderState) -> [String] {
        state.segments.map { $0.output }
    }

    static func focusedSegmentIndex(state: IosRenderState) -> Int? {
        state.segments.firstIndex(where: { $0.focused })
    }

    private static func editModeText(_ state: IosRenderState) -> String {
        let editIdx = state.segmentEditIndex.map { Int($0) } ?? 0
        let parts = state.segments.enumerated().map { i, seg in
            i == editIdx ? "[\(seg.input)]" : seg.input
        }
        return "✏ " + parts.joined(separator: " · ")
    }
}
