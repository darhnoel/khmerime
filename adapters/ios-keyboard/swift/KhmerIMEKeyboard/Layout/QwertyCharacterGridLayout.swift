import UIKit

struct QwertyCharacterGridLayout {
    let availableWidth: CGFloat
    let spacing: CGFloat

    var characterKeyWidth: CGFloat {
        (availableWidth - spacing * 9) / 10
    }

    var row2LeadingSideInset: CGFloat {
        row2SideInset
    }

    var row2TrailingSideInset: CGFloat {
        row2SideInset
    }

    var row3LeadingControlWidth: CGFloat {
        row3ControlWidth
    }

    var row3TrailingControlWidth: CGFloat {
        row3ControlWidth
    }

    var row1ConsumedWidth: CGFloat {
        characterKeyWidth * 10 + spacing * 9
    }

    var row2ConsumedWidth: CGFloat {
        row2LeadingSideInset + characterKeyWidth * 9 + spacing * 8 + row2TrailingSideInset
    }

    var row3ConsumedWidth: CGFloat {
        row3LeadingControlWidth + characterKeyWidth * 7 + spacing * 8 + row3TrailingControlWidth
    }

    private var row2SideInset: CGFloat {
        (availableWidth - characterKeyWidth * 9 - spacing * 8) / 2
    }

    private var row3ControlWidth: CGFloat {
        (availableWidth - characterKeyWidth * 7 - spacing * 8) / 2
    }
}

final class QwertyCharacterGridRowView: UIStackView {
    private enum Role {
        case centeredLetters
        case edgeControls
    }

    private let role: Role
    private let letterWidthConstraints: [NSLayoutConstraint]
    private let leadingControlWidthConstraint: NSLayoutConstraint?
    private let trailingControlWidthConstraint: NSLayoutConstraint?

    static func centeredLetterRow(_ keys: [UIButton], spacing: CGFloat) -> QwertyCharacterGridRowView {
        let constraints = keys.map { key in
            key.widthAnchor.constraint(equalToConstant: 0)
        }
        return QwertyCharacterGridRowView(
            arrangedSubviews: keys,
            spacing: spacing,
            role: .centeredLetters,
            letterWidthConstraints: constraints,
            leadingControlWidthConstraint: nil,
            trailingControlWidthConstraint: nil
        )
    }

    static func edgeControlRow(
        leadingControl: UIButton,
        letters: [UIButton],
        trailingControl: UIButton,
        spacing: CGFloat
    ) -> QwertyCharacterGridRowView {
        let leadingWidth = leadingControl.widthAnchor.constraint(equalToConstant: 0)
        let trailingWidth = trailingControl.widthAnchor.constraint(equalToConstant: 0)
        let letterWidths = letters.map { key in
            key.widthAnchor.constraint(equalToConstant: 0)
        }
        return QwertyCharacterGridRowView(
            arrangedSubviews: [leadingControl] + letters + [trailingControl],
            spacing: spacing,
            role: .edgeControls,
            letterWidthConstraints: letterWidths,
            leadingControlWidthConstraint: leadingWidth,
            trailingControlWidthConstraint: trailingWidth
        )
    }

    private init(
        arrangedSubviews: [UIView],
        spacing: CGFloat,
        role: Role,
        letterWidthConstraints: [NSLayoutConstraint],
        leadingControlWidthConstraint: NSLayoutConstraint?,
        trailingControlWidthConstraint: NSLayoutConstraint?
    ) {
        self.role = role
        self.letterWidthConstraints = letterWidthConstraints
        self.leadingControlWidthConstraint = leadingControlWidthConstraint
        self.trailingControlWidthConstraint = trailingControlWidthConstraint
        super.init(frame: .zero)

        axis = .horizontal
        self.spacing = spacing
        distribution = .fill
        alignment = .fill
        isLayoutMarginsRelativeArrangement = role == .centeredLetters

        arrangedSubviews.forEach(addArrangedSubview)
        NSLayoutConstraint.activate(
            letterWidthConstraints
                + [leadingControlWidthConstraint, trailingControlWidthConstraint].compactMap { $0 }
        )
    }

    required init(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func layoutSubviews() {
        applyCharacterGrid()
        super.layoutSubviews()
    }

    private func applyCharacterGrid() {
        guard bounds.width > 0 else { return }

        let layout = QwertyCharacterGridLayout(availableWidth: bounds.width, spacing: spacing)
        for constraint in letterWidthConstraints {
            constraint.constant = layout.characterKeyWidth
        }

        switch role {
        case .centeredLetters:
            directionalLayoutMargins = NSDirectionalEdgeInsets(
                top: 0,
                leading: layout.row2LeadingSideInset,
                bottom: 0,
                trailing: layout.row2TrailingSideInset
            )
        case .edgeControls:
            directionalLayoutMargins = .zero
            leadingControlWidthConstraint?.constant = layout.row3LeadingControlWidth
            trailingControlWidthConstraint?.constant = layout.row3TrailingControlWidth
        }
    }
}
