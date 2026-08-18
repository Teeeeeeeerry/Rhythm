//! Integration tests for the player FFI layer — seek, position, and duration.

use rhythm_core::ffi::*;

#[test]
fn test_seek_returns_zero_on_success() {
    let player = rhythm_player_create();
    assert!(!player.is_null(), "player handle should not be null");

    // Seek to a position in a fresh (stopped) player — the engine's seek
    // validates seconds >= 0 and within duration, but 0 is always valid.
    let ret = unsafe { rhythm_player_seek(player, 0.0) };
    assert_eq!(ret, 0, "seek to 0 in a fresh player should succeed");

    unsafe { rhythm_player_destroy(player) };
}

#[test]
fn test_seek_returns_error_for_null_pointer() {
    let ret = unsafe { rhythm_player_seek(std::ptr::null_mut(), 42.0) };
    assert_eq!(ret, -1, "seek with null pointer should return -1");
}

#[test]
fn test_get_position_returns_zero_for_fresh_player() {
    let player = rhythm_player_create();
    assert!(!player.is_null());

    let pos = unsafe { rhythm_player_get_position(player) };
    assert_eq!(pos, 0.0, "fresh player should start at position 0");

    unsafe { rhythm_player_destroy(player) };
}

#[test]
fn test_get_duration_returns_zero_for_fresh_player() {
    let player = rhythm_player_create();
    assert!(!player.is_null());

    let dur = unsafe { rhythm_player_get_duration(player) };
    assert_eq!(dur, 0.0, "fresh player should have 0 duration");

    unsafe { rhythm_player_destroy(player) };
}
