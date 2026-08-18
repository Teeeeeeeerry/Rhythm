//! Play queue with sequential, random (shuffle), and single-loop modes.
//!
//! The queue holds a sorted list of tracks and a cursor into that list.
//! When the current track finishes, the queue determines the next track
//! based on the active play mode.

use crate::TrackInfo;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Supported play modes for the queue.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum PlayMode {
    /// Play tracks in order; stop after the last track.
    Sequential,
    /// Shuffle all tracks once before repeating.
    Shuffle,
    /// Repeat the current track indefinitely.
    SingleLoop,
    /// Play in order, then loop back to the start (all tracks).
    ListLoop,
}

impl PlayMode {
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => PlayMode::Sequential,
            1 => PlayMode::Shuffle,
            2 => PlayMode::SingleLoop,
            3 => PlayMode::ListLoop,
            _ => PlayMode::Sequential,
        }
    }

    pub fn to_i32(self) -> i32 {
        match self {
            PlayMode::Sequential => 0,
            PlayMode::Shuffle => 1,
            PlayMode::SingleLoop => 2,
            PlayMode::ListLoop => 3,
        }
    }
}

/// Manages an ordered list of tracks with a playback cursor.
#[derive(Debug, Clone)]
pub struct PlayQueue {
    /// Ordered list of track IDs in the queue (source playlist or library order).
    pub tracks: Vec<TrackInfo>,
    /// Internal shuffle order (indices into `tracks`).
    shuffle_order: Vec<usize>,
    /// Current position in the active order.
    cursor: usize,
    /// Active play mode.
    pub mode: PlayMode,
}

impl PlayQueue {
    /// Create a new queue from a track list.
    pub fn new(tracks: Vec<TrackInfo>) -> Self {
        let len = tracks.len();
        let mut order: Vec<usize> = (0..len).collect();
        let mut rng = thread_rng();
        order.shuffle(&mut rng);

        PlayQueue {
            tracks,
            shuffle_order: order,
            cursor: 0,
            mode: PlayMode::Sequential,
        }
    }

    /// Set the play mode.
    pub fn set_mode(&mut self, mode: PlayMode) {
        self.mode = mode;
    }

    /// Get the current track, if any.
    pub fn current(&self) -> Option<&TrackInfo> {
        if self.tracks.is_empty() {
            return None;
        }
        let idx = self.active_index();
        self.tracks.get(idx)
    }

    /// Move to the next track and return it.
    /// Returns `None` if the queue is exhausted (Sequential at end).
    pub fn advance(&mut self) -> Option<&TrackInfo> {
        if self.tracks.is_empty() {
            return None;
        }

        match self.mode {
            PlayMode::SingleLoop => {
                // Stay at current; just return it again.
                let idx = self.active_index();
                self.tracks.get(idx)
            }
            PlayMode::Sequential => {
                self.cursor += 1;
                if self.cursor >= self.tracks.len() {
                    self.cursor = self.tracks.len(); // exhausted
                    return None;
                }
                let idx = self.active_index();
                self.tracks.get(idx)
            }
            PlayMode::ListLoop => {
                self.cursor += 1;
                if self.cursor >= self.tracks.len() {
                    self.cursor = 0;
                }
                let idx = self.active_index();
                self.tracks.get(idx)
            }
            PlayMode::Shuffle => {
                self.cursor += 1;
                if self.cursor >= self.tracks.len() {
                    // Reshuffle for next loop
                    let mut rng = thread_rng();
                    self.shuffle_order = (0..self.tracks.len()).collect();
                    self.shuffle_order.shuffle(&mut rng);
                    self.cursor = 0;
                }
                let idx = self.active_index();
                self.tracks.get(idx)
            }
        }
    }

    /// Move to the previous track and return it.
    pub fn previous(&mut self) -> Option<&TrackInfo> {
        if self.tracks.is_empty() {
            return None;
        }

        if self.cursor > 0 {
            self.cursor -= 1;
        } else {
            // Wrap to end for non-sequential modes
            if self.mode != PlayMode::Sequential && !self.tracks.is_empty() {
                self.cursor = self.tracks.len().saturating_sub(1);
            }
        }

        let idx = self.active_index();
        self.tracks.get(idx)
    }

    /// Jump to a specific track by its database ID. Returns true if found.
    pub fn jump_to(&mut self, track_id: i64) -> bool {
        if let Some(pos) = self.tracks.iter().position(|t| t.id == Some(track_id)) {
            // Find the position in the current ordering
            match self.mode {
                PlayMode::Shuffle => {
                    if let Some(shuf_pos) = self.shuffle_order.iter().position(|&i| i == pos) {
                        self.cursor = shuf_pos;
                        return true;
                    }
                }
                _ => {
                    self.cursor = pos;
                    return true;
                }
            }
        }
        false
    }

    /// Replace the track list (e.g., when user plays a different playlist).
    pub fn replace(&mut self, tracks: Vec<TrackInfo>) {
        let len = tracks.len();
        let mut order: Vec<usize> = (0..len).collect();
        let mut rng = thread_rng();
        order.shuffle(&mut rng);

        self.tracks = tracks;
        self.shuffle_order = order;
        self.cursor = 0;
    }

    /// Whether the queue has more tracks after the current one.
    pub fn has_next(&self) -> bool {
        match self.mode {
            PlayMode::SingleLoop => !self.tracks.is_empty(),
            PlayMode::Sequential => self.cursor + 1 < self.tracks.len(),
            PlayMode::ListLoop | PlayMode::Shuffle => !self.tracks.is_empty(),
        }
    }

    /// Whether there is a previous track.
    pub fn has_previous(&self) -> bool {
        if self.mode == PlayMode::Sequential {
            self.cursor > 0
        } else {
            !self.tracks.is_empty()
        }
    }

    // ─── Private helpers ─────────────────────────────────────────

    fn active_index(&self) -> usize {
        match self.mode {
            PlayMode::Shuffle => self
                .shuffle_order
                .get(self.cursor)
                .copied()
                .unwrap_or(0),
            _ => self.cursor.min(self.tracks.len().saturating_sub(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_track(id: i64, title: &str) -> TrackInfo {
        TrackInfo {
            id: Some(id),
            file_path: Some(format!("/music/{title}.mp3")),
            source_type: crate::SourceType::Local,
            source_url: None,
            title: title.to_string(),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
            album_artist: None,
            track_number: Some(1),
            disc_number: Some(1),
            genre: None,
            year: Some(2024),
            duration: 180.0,
            format: Some("mp3".to_string()),
            bitrate: Some(320),
            sample_rate: Some(44100),
            channels: Some(2),
            file_size: Some(5000000),
            date_added: None,
            last_played: None,
            play_count: 0,
            artwork_path: None,
            is_available: true,
        }
    }

    #[test]
    fn test_sequential_plays_in_order() {
        let tracks = vec![dummy_track(1, "A"), dummy_track(2, "B"), dummy_track(3, "C")];
        let mut q = PlayQueue::new(tracks);
        q.set_mode(PlayMode::Sequential);

        assert_eq!(q.current().unwrap().title, "A");
        assert_eq!(q.advance().unwrap().title, "B");
        assert_eq!(q.advance().unwrap().title, "C");
        assert!(q.advance().is_none()); // exhausted
    }

    #[test]
    fn test_list_loop_wraps() {
        let tracks = vec![dummy_track(1, "A"), dummy_track(2, "B")];
        let mut q = PlayQueue::new(tracks);
        q.set_mode(PlayMode::ListLoop);

        assert_eq!(q.current().unwrap().title, "A");
        assert_eq!(q.advance().unwrap().title, "B");
        assert_eq!(q.advance().unwrap().title, "A"); // wraps
    }

    #[test]
    fn test_single_loop_repeats() {
        let tracks = vec![dummy_track(1, "A"), dummy_track(2, "B")];
        let mut q = PlayQueue::new(tracks);
        q.set_mode(PlayMode::SingleLoop);

        assert_eq!(q.current().unwrap().title, "A");
        assert_eq!(q.advance().unwrap().title, "A"); // same
        assert_eq!(q.advance().unwrap().title, "A"); // still same
    }

    #[test]
    fn test_previous_in_sequential() {
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        q.set_mode(PlayMode::Sequential);

        q.advance(); // B
        q.advance(); // C
        assert_eq!(q.previous().unwrap().title, "B");
        assert_eq!(q.previous().unwrap().title, "A");
    }

    #[test]
    fn test_jump_to() {
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        assert!(q.jump_to(3));
        assert_eq!(q.current().unwrap().title, "C");
    }

    #[test]
    fn test_shuffle_covers_all() {
        let tracks: Vec<_> = (1..=20).map(|i| dummy_track(i, &format!("T{i:02}"))).collect();
        let mut q = PlayQueue::new(tracks);
        q.set_mode(PlayMode::Shuffle);

        let mut seen: Vec<i64> = Vec::new();
        for _ in 0..20 {
            seen.push(q.current().unwrap().id.unwrap());
            q.advance();
        }
        seen.sort();
        let expected: Vec<i64> = (1..=20).collect();
        assert_eq!(seen, expected);
    }

    /// Q-05 (missing branch): `jump_to` on an unknown id returns false and
    /// leaves the cursor untouched.
    #[test]
    fn test_jump_to_missing_id_leaves_cursor() {
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        assert!(!q.jump_to(99));
        assert_eq!(q.current().unwrap().title, "A");
    }

    /// Q-07: `replace` resets the cursor to the new first track (#72).
    #[test]
    fn test_replace_resets_cursor() {
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        q.advance(); // B
        assert_eq!(q.current().unwrap().title, "B");

        q.replace(vec![dummy_track(10, "X"), dummy_track(11, "Y")]);
        assert_eq!(q.current().unwrap().title, "X");
        assert!(!q.has_previous());
    }

    /// Q-08: the `has_next` matrix across modes and empty queues.
    #[test]
    fn test_has_next_matrix() {
        let tracks = vec![dummy_track(1, "A"), dummy_track(2, "B")];

        // Empty queue: never a next, in any mode.
        for mode in [
            PlayMode::Sequential,
            PlayMode::ListLoop,
            PlayMode::SingleLoop,
            PlayMode::Shuffle,
        ] {
            let mut q = PlayQueue::new(vec![]);
            q.set_mode(mode);
            assert!(!q.has_next(), "{mode:?} on empty queue");
        }

        // Sequential: true before the last track, false on it.
        let mut q = PlayQueue::new(tracks.clone());
        q.set_mode(PlayMode::Sequential);
        assert!(q.has_next());
        q.advance();
        assert!(!q.has_next());

        // Non-sequential: always true while non-empty.
        for mode in [PlayMode::ListLoop, PlayMode::SingleLoop, PlayMode::Shuffle] {
            let mut q = PlayQueue::new(tracks.clone());
            q.set_mode(mode);
            assert!(q.has_next(), "{mode:?}");
        }
    }

    /// Q-09: the `has_previous` matrix across modes and empty queues.
    #[test]
    fn test_has_previous_matrix() {
        let tracks = vec![dummy_track(1, "A"), dummy_track(2, "B")];

        // Empty queue: never a previous, in any mode.
        for mode in [
            PlayMode::Sequential,
            PlayMode::ListLoop,
            PlayMode::SingleLoop,
            PlayMode::Shuffle,
        ] {
            let mut q = PlayQueue::new(vec![]);
            q.set_mode(mode);
            assert!(!q.has_previous(), "{mode:?} on empty queue");
        }

        // Sequential: false at the head, true after moving.
        let mut q = PlayQueue::new(tracks.clone());
        q.set_mode(PlayMode::Sequential);
        assert!(!q.has_previous());
        q.advance();
        assert!(q.has_previous());

        // Non-sequential: true while non-empty, even at the head.
        for mode in [PlayMode::ListLoop, PlayMode::SingleLoop, PlayMode::Shuffle] {
            let mut q = PlayQueue::new(tracks.clone());
            q.set_mode(mode);
            assert!(q.has_previous(), "{mode:?}");
        }
    }

    /// Q-10: an empty queue yields no track and no navigation.
    #[test]
    fn test_empty_queue() {
        let mut q = PlayQueue::new(vec![]);
        assert!(q.current().is_none());
        assert!(q.advance().is_none());
        assert!(q.previous().is_none());
        assert!(!q.has_next());
        assert!(!q.has_previous());
    }

    /// Q-11: non-sequential modes wrap `previous` at the head to the last
    /// track of the active order.
    #[test]
    fn test_previous_wraps_in_non_sequential() {
        for mode in [PlayMode::ListLoop, PlayMode::SingleLoop] {
            let mut q = PlayQueue::new(vec![
                dummy_track(1, "A"),
                dummy_track(2, "B"),
                dummy_track(3, "C"),
            ]);
            q.set_mode(mode);
            assert_eq!(q.previous().unwrap().title, "C", "{mode:?}");
        }

        // Shuffle: previous walks backwards along the shuffle order; the head
        // wraps to the shuffle order's last entry (any track).
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        q.set_mode(PlayMode::Shuffle);
        let first = q.current().unwrap().id.unwrap();
        let wrapped = q.previous().unwrap().id.unwrap();
        let back = q.previous().unwrap().id.unwrap();
        assert_ne!(wrapped, back);
        for id in [wrapped, back] {
            assert!([1, 2, 3].contains(&id), "unexpected track id {id}");
        }
        q.previous();
        assert_eq!(q.current().unwrap().id.unwrap(), first);
    }

    /// Q-12: after Sequential exhausts (cursor parked at len), `previous`
    /// returns the last track.
    #[test]
    fn test_previous_after_exhaustion() {
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        q.set_mode(PlayMode::Sequential);
        q.advance();
        q.advance();
        assert!(q.advance().is_none());
        assert_eq!(q.previous().unwrap().title, "C");
    }

    /// Q-13: after a full Shuffle round the order is rebuilt and the second
    /// round still covers every track once.
    #[test]
    fn test_shuffle_second_round_reshuffles() {
        let tracks: Vec<_> = (1..=20).map(|i| dummy_track(i, &format!("T{i:02}"))).collect();
        let mut q = PlayQueue::new(tracks);
        q.set_mode(PlayMode::Shuffle);

        for round in 0..2 {
            let mut seen: Vec<i64> = Vec::new();
            for _ in 0..20 {
                seen.push(q.current().unwrap().id.unwrap());
                q.advance();
            }
            seen.sort();
            let expected: Vec<i64> = (1..=20).collect();
            assert_eq!(seen, expected, "round {round}");
        }
    }

    /// Q-14: `PlayMode::from_i32` maps known values and falls back to
    /// Sequential for anything else.
    #[test]
    fn test_play_mode_from_i32() {
        assert_eq!(PlayMode::from_i32(0), PlayMode::Sequential);
        assert_eq!(PlayMode::from_i32(1), PlayMode::Shuffle);
        assert_eq!(PlayMode::from_i32(2), PlayMode::SingleLoop);
        assert_eq!(PlayMode::from_i32(3), PlayMode::ListLoop);
        assert_eq!(PlayMode::from_i32(4), PlayMode::Sequential);
        assert_eq!(PlayMode::from_i32(-1), PlayMode::Sequential);
    }

    /// Q-15: `jump_to` in Shuffle mode positions the cursor via the shuffle
    /// order.
    #[test]
    fn test_jump_to_in_shuffle() {
        let mut q = PlayQueue::new(vec![
            dummy_track(1, "A"),
            dummy_track(2, "B"),
            dummy_track(3, "C"),
        ]);
        q.set_mode(PlayMode::Shuffle);
        assert!(q.jump_to(2));
        assert_eq!(q.current().unwrap().id, Some(2));
    }
}
