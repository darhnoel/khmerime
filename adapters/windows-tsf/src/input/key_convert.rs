//! Windows virtual-key conversion for the shared session key contract.

use khmerime_session::NativeKeyEvent;

/// Key conversion is implemented for the pure Rust contract layer.
pub const KEY_CONVERSION_IMPLEMENTED: bool = true;

pub const VK_BACK: u32 = 0x08;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_SPACE: u32 = 0x20;
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;

pub const SESSION_KEY_BACKSPACE: u32 = 0xFF08;
pub const SESSION_KEY_ESCAPE: u32 = 0xFF1B;
pub const SESSION_KEY_LEFT: u32 = 0xFF51;
pub const SESSION_KEY_UP: u32 = 0xFF52;
pub const SESSION_KEY_RIGHT: u32 = 0xFF53;
pub const SESSION_KEY_DOWN: u32 = 0xFF54;
pub const SESSION_KEY_RETURN: u32 = 0xFF0D;
pub const SESSION_KEY_SPACE: u32 = 0x20;

/// Mirrors the session's modifier bits for shortcut pass-through.
pub const STATE_CONTROL_MASK: u32 = 1 << 2;
pub const STATE_ALT_MASK: u32 = 1 << 3;
pub const STATE_SUPER_MASK: u32 = 1 << 26;
pub const STATE_RELEASE_MASK: u32 = 1 << 30;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WindowsKeyInput {
    /// Windows virtual-key code from `wParam`.
    pub virtual_key: u32,
    /// Native scan/lParam data for diagnostics/future layout work.
    pub scan_code: u32,
    /// Normalized modifier state using session modifier masks.
    pub state: u32,
    /// Translated printable character, when the caller already has one.
    pub translated_char: Option<char>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvertedKey {
    /// Send this normalized key to `ImeSession`.
    Event(NativeKeyEvent),
    /// Let the host application handle the key.
    PassThrough,
}

pub fn convert_windows_key(input: WindowsKeyInput) -> ConvertedKey {
    if is_shortcut_state(input.state) {
        return ConvertedKey::PassThrough;
    }

    let keyval = match input.virtual_key {
        VK_BACK => SESSION_KEY_BACKSPACE,
        VK_RETURN => SESSION_KEY_RETURN,
        VK_ESCAPE => SESSION_KEY_ESCAPE,
        VK_SPACE => SESSION_KEY_SPACE,
        VK_LEFT => SESSION_KEY_LEFT,
        VK_UP => SESSION_KEY_UP,
        VK_RIGHT => SESSION_KEY_RIGHT,
        VK_DOWN => SESSION_KEY_DOWN,
        VK_TAB => return ConvertedKey::PassThrough,
        _ => match input.translated_char {
            Some(ch) if ch.is_ascii_graphic() => ch as u32,
            _ => return ConvertedKey::PassThrough,
        },
    };

    ConvertedKey::Event(NativeKeyEvent {
        keyval,
        keycode: input.virtual_key,
        state: input.state,
    })
}

pub fn would_handle_windows_key(input: WindowsKeyInput) -> bool {
    matches!(convert_windows_key(input), ConvertedKey::Event(_))
}

fn is_shortcut_state(state: u32) -> bool {
    state & (STATE_CONTROL_MASK | STATE_ALT_MASK | STATE_SUPER_MASK | STATE_RELEASE_MASK) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn printable_ascii_becomes_session_keyval() {
        let converted = convert_windows_key(WindowsKeyInput {
            virtual_key: 0x41,
            translated_char: Some('a'),
            ..WindowsKeyInput::default()
        });

        assert_eq!(
            converted,
            ConvertedKey::Event(NativeKeyEvent {
                keyval: 'a' as u32,
                keycode: 0x41,
                state: 0,
            })
        );
    }

    #[test]
    fn special_keys_map_to_session_contract() {
        assert_eq!(keyval_for(VK_BACK), Some(SESSION_KEY_BACKSPACE));
        assert_eq!(keyval_for(VK_RETURN), Some(SESSION_KEY_RETURN));
        assert_eq!(keyval_for(VK_ESCAPE), Some(SESSION_KEY_ESCAPE));
        assert_eq!(keyval_for(VK_LEFT), Some(SESSION_KEY_LEFT));
        assert_eq!(keyval_for(VK_DOWN), Some(SESSION_KEY_DOWN));
    }

    #[test]
    fn shortcuts_pass_through() {
        let converted = convert_windows_key(WindowsKeyInput {
            virtual_key: 0x41,
            state: STATE_CONTROL_MASK,
            translated_char: Some('a'),
            ..WindowsKeyInput::default()
        });

        assert_eq!(converted, ConvertedKey::PassThrough);
    }

    #[test]
    fn test_and_down_prediction_share_conversion() {
        let input = WindowsKeyInput {
            virtual_key: 0x41,
            translated_char: Some('a'),
            ..WindowsKeyInput::default()
        };

        assert_eq!(
            would_handle_windows_key(input),
            matches!(convert_windows_key(input), ConvertedKey::Event(_))
        );
    }

    fn keyval_for(virtual_key: u32) -> Option<u32> {
        match convert_windows_key(WindowsKeyInput {
            virtual_key,
            ..WindowsKeyInput::default()
        }) {
            ConvertedKey::Event(event) => Some(event.keyval),
            ConvertedKey::PassThrough => None,
        }
    }
}
