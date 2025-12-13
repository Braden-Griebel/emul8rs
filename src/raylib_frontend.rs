use raylib::{RaylibHandle, ffi::KeyboardKey};
// Keymap
// mapped from
// 1  2  3  4
// Q  W  E  R
// A  S  D  F
// Z  X  C  V
// to
// 1  2  3  C
// 4  5  6  D
// 7  8  9  E
// A  0  B  F
const KEYMAP: [KeyboardKey; 16] = [
    KeyboardKey::KEY_X,
    KeyboardKey::KEY_ONE,
    KeyboardKey::KEY_TWO,
    KeyboardKey::KEY_THREE,
    KeyboardKey::KEY_Q,
    KeyboardKey::KEY_W,
    KeyboardKey::KEY_E,
    KeyboardKey::KEY_A,
    KeyboardKey::KEY_S,
    KeyboardKey::KEY_D,
    KeyboardKey::KEY_Z,
    KeyboardKey::KEY_C,
    KeyboardKey::KEY_FOUR,
    KeyboardKey::KEY_R,
    KeyboardKey::KEY_F,
    KeyboardKey::KEY_V,
];

struct raylib_frontend {}
