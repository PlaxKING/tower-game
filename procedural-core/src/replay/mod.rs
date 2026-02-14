//! Replay System (FEAT-003)
//!
//! Records player inputs and deterministically replays combat encounters.
//! Built on top of the existing DeltaLog system.
//!
//! Workflow:
//! 1. Start recording → captures all inputs + initial floor state (seed + floor_id)
//! 2. Player plays → inputs recorded per tick
//! 3. Stop recording → create ReplayRecording with header + frames
//! 4. Playback → regenerate floor from seed, apply inputs frame-by-frame
//! 5. Verify → compare resulting DeltaLog to original for determinism

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::generation::TowerSeed;
use crate::replication::Delta;

pub struct ReplayPlugin;

impl Plugin for ReplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ReplayRecorder::default())
            .add_event::<ReplayEvent>()
            .add_systems(Update, process_replay_events);
    }
}

/// Type of player input
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    Move,         // WASD movement
    Attack,       // Left mouse button
    Parry,        // Right mouse button
    Dodge,        // Shift
    UseAbility,   // 1-6 hotbar
    Interact,     // E key
    Jump,         // Space
    ChangeWeapon, // Tab
}

/// A single input frame (1 tick = 100ms at 10 tick/s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputFrame {
    pub tick: u64,
    pub input_type: InputType,
    pub payload: String, // JSON: movement vector, ability slot, etc.
}

impl InputFrame {
    pub fn new(tick: u64, input_type: InputType, payload: &str) -> Self {
        Self {
            tick,
            input_type,
            payload: payload.to_string(),
        }
    }

    /// Hash of this frame for integrity verification
    pub fn hash(&self) -> u64 {
        let mut hasher = Sha3_256::new();
        hasher.update(self.tick.to_le_bytes());
        hasher.update((self.input_type as u32).to_le_bytes());
        hasher.update(self.payload.as_bytes());
        let result = hasher.finalize();
        u64::from_le_bytes(result[0..8].try_into().unwrap())
    }
}

/// Metadata for a replay recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayHeader {
    pub replay_id: String,
    pub seed: u64,
    pub floor_id: u32,
    pub player_name: String,
    pub player_build: String, // JSON: weapon, stats, abilities
    pub start_time_utc: u64,  // Unix timestamp
    pub duration_ticks: u64,
    pub total_frames: usize,
    pub outcome: ReplayOutcome,
    pub version: u32, // Replay format version
}

impl ReplayHeader {
    pub fn new(
        replay_id: &str,
        seed: u64,
        floor_id: u32,
        player_name: &str,
        player_build: &str,
    ) -> Self {
        Self {
            replay_id: replay_id.to_string(),
            seed,
            floor_id,
            player_name: player_name.to_string(),
            player_build: player_build.to_string(),
            start_time_utc: 0,
            duration_ticks: 0,
            total_frames: 0,
            outcome: ReplayOutcome::InProgress,
            version: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayOutcome {
    InProgress,
    Victory,
    Death,
    Abandoned,
}

/// Complete replay recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRecording {
    pub header: ReplayHeader,
    pub frames: Vec<InputFrame>,
    pub final_deltas: Vec<Delta>, // DeltaLog snapshot for verification
    pub recording_hash: u64,
}

impl ReplayRecording {
    pub fn new(header: ReplayHeader, frames: Vec<InputFrame>, final_deltas: Vec<Delta>) -> Self {
        let mut recording = Self {
            header,
            frames,
            final_deltas,
            recording_hash: 0,
        };
        recording.recording_hash = recording.compute_hash();
        recording
    }

    fn compute_hash(&self) -> u64 {
        let mut hasher = Sha3_256::new();
        hasher.update(self.header.seed.to_le_bytes());
        hasher.update(self.header.floor_id.to_le_bytes());
        for frame in &self.frames {
            hasher.update(frame.hash().to_le_bytes());
        }
        let result = hasher.finalize();
        u64::from_le_bytes(result[0..8].try_into().unwrap())
    }

    pub fn verify(&self) -> bool {
        self.recording_hash == self.compute_hash()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Estimated file size in bytes
    pub fn estimated_size(&self) -> usize {
        200 + self.frames.len() * 50 + self.final_deltas.len() * 80
    }
}

/// Playback state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
    Seeking,
    Finished,
    Error,
}

/// Replay playback controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayPlayback {
    pub recording_id: String,
    pub state: PlaybackState,
    pub current_tick: u64,
    pub current_frame_idx: usize,
    pub total_frames: usize,
    pub speed: f32, // 1.0 = normal, 2.0 = 2x, 0.5 = slow-mo
    pub loop_playback: bool,
}

impl ReplayPlayback {
    pub fn new(recording: &ReplayRecording) -> Self {
        Self {
            recording_id: recording.header.replay_id.clone(),
            state: PlaybackState::Idle,
            current_tick: 0,
            current_frame_idx: 0,
            total_frames: recording.frames.len(),
            speed: 1.0,
            loop_playback: false,
        }
    }

    pub fn play(&mut self) {
        if self.state == PlaybackState::Finished && !self.loop_playback {
            return;
        }
        self.state = PlaybackState::Playing;
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Idle;
        self.current_tick = 0;
        self.current_frame_idx = 0;
    }

    pub fn seek(&mut self, target_tick: u64) {
        self.current_tick = target_tick;
        self.state = PlaybackState::Seeking;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.1, 10.0);
    }

    pub fn progress(&self) -> f32 {
        if self.total_frames == 0 {
            return 0.0;
        }
        (self.current_frame_idx as f32 / self.total_frames as f32).clamp(0.0, 1.0)
    }

    pub fn advance<'a>(&mut self, recording: &'a ReplayRecording) -> Option<&'a InputFrame> {
        // If finished and looping, reset to start
        let did_loop = if self.state == PlaybackState::Finished && self.loop_playback {
            self.current_frame_idx = 0;
            self.current_tick = 0;
            self.state = PlaybackState::Playing;
            true
        } else {
            false
        };

        if self.state != PlaybackState::Playing {
            return None;
        }

        if self.current_frame_idx >= recording.frames.len() {
            self.state = PlaybackState::Finished;
            return None;
        }

        let frame = &recording.frames[self.current_frame_idx];
        self.current_tick = frame.tick;
        self.current_frame_idx += 1;

        // After consuming a frame, check if that was the last one
        // Don't transition to Finished if we just looped (stay in Playing state)
        if !did_loop && self.current_frame_idx >= recording.frames.len() {
            self.state = PlaybackState::Finished;
        }

        Some(frame)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Active replay recorder (Bevy resource)
#[derive(Resource, Debug, Clone, Default)]
pub struct ReplayRecorder {
    pub recording: bool,
    pub current_header: Option<ReplayHeader>,
    pub frames: Vec<InputFrame>,
    pub start_tick: u64,
}

impl ReplayRecorder {
    pub fn start_recording(
        &mut self,
        seed: &TowerSeed,
        floor_id: u32,
        player_name: &str,
        player_build: &str,
        current_tick: u64,
    ) {
        let replay_id = format!("replay_{}_{}", floor_id, current_tick);
        let header = ReplayHeader::new(&replay_id, seed.seed, floor_id, player_name, player_build);

        self.recording = true;
        self.current_header = Some(header);
        self.frames.clear();
        self.start_tick = current_tick;
    }

    pub fn record_frame(&mut self, tick: u64, input_type: InputType, payload: &str) {
        if !self.recording {
            return;
        }
        let frame = InputFrame::new(tick, input_type, payload);
        self.frames.push(frame);
    }

    pub fn stop_recording(
        &mut self,
        outcome: ReplayOutcome,
        final_deltas: Vec<Delta>,
        current_tick: u64,
    ) -> Option<ReplayRecording> {
        if !self.recording {
            return None;
        }

        self.recording = false;

        let mut header = self.current_header.take()?;
        header.duration_ticks = current_tick.saturating_sub(self.start_tick);
        header.total_frames = self.frames.len();
        header.outcome = outcome;
        header.start_time_utc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let recording = ReplayRecording::new(header, self.frames.clone(), final_deltas);

        self.frames.clear();

        Some(recording)
    }

    pub fn cancel_recording(&mut self) {
        self.recording = false;
        self.current_header = None;
        self.frames.clear();
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }
}

/// Event for recording inputs
#[derive(Event, Debug, Clone)]
pub struct ReplayEvent {
    pub input_type: InputType,
    pub payload: String,
}

fn process_replay_events(
    mut events: EventReader<ReplayEvent>,
    mut recorder: ResMut<ReplayRecorder>,
    time: Res<Time>,
) {
    if !recorder.recording {
        return;
    }

    let tick = (time.elapsed_secs() * 10.0) as u64;

    for event in events.read() {
        recorder.record_frame(tick, event.input_type, &event.payload);
    }
}

/// Snapshot for FFI
#[derive(Debug, Serialize, Deserialize)]
pub struct ReplaySnapshot {
    pub is_recording: bool,
    pub current_replay_id: Option<String>,
    pub recorded_frames: usize,
    pub available_input_types: Vec<String>,
}

impl ReplaySnapshot {
    pub fn capture(recorder: &ReplayRecorder) -> Self {
        Self {
            is_recording: recorder.recording,
            current_replay_id: recorder
                .current_header
                .as_ref()
                .map(|h| h.replay_id.clone()),
            recorded_frames: recorder.frames.len(),
            available_input_types: vec![
                "Move".to_string(),
                "Attack".to_string(),
                "Parry".to_string(),
                "Dodge".to_string(),
                "UseAbility".to_string(),
                "Interact".to_string(),
                "Jump".to_string(),
                "ChangeWeapon".to_string(),
            ],
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_frame_creation() {
        let frame = InputFrame::new(100, InputType::Attack, r#"{"combo":1}"#);
        assert_eq!(frame.tick, 100);
        assert_eq!(frame.input_type, InputType::Attack);
        assert!(frame.hash() > 0);
    }

    #[test]
    fn test_input_frame_hash_deterministic() {
        let f1 = InputFrame::new(100, InputType::Attack, r#"{"combo":1}"#);
        let f2 = InputFrame::new(100, InputType::Attack, r#"{"combo":1}"#);
        assert_eq!(f1.hash(), f2.hash());
    }

    #[test]
    fn test_replay_header_creation() {
        let header = ReplayHeader::new("replay_1", 42, 10, "Player1", r#"{"weapon":"Sword"}"#);
        assert_eq!(header.replay_id, "replay_1");
        assert_eq!(header.seed, 42);
        assert_eq!(header.floor_id, 10);
        assert_eq!(header.version, 1);
        assert_eq!(header.outcome, ReplayOutcome::InProgress);
    }

    #[test]
    fn test_replay_recording_creation() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![
            InputFrame::new(0, InputType::Move, r#"{"x":1.0,"y":0.0}"#),
            InputFrame::new(1, InputType::Attack, r#"{"combo":1}"#),
        ];
        let recording = ReplayRecording::new(header, frames, vec![]);
        assert_eq!(recording.frames.len(), 2);
        assert!(recording.verify());
    }

    #[test]
    fn test_replay_recording_verify() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![InputFrame::new(0, InputType::Move, r#"{}"#)];
        let mut recording = ReplayRecording::new(header, frames, vec![]);
        assert!(recording.verify());

        recording
            .frames
            .push(InputFrame::new(1, InputType::Attack, ""));
        assert!(
            !recording.verify(),
            "Modified recording should fail verification"
        );
    }

    #[test]
    fn test_replay_recording_json_roundtrip() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![InputFrame::new(0, InputType::Move, r#"{"x":1.0}"#)];
        let recording = ReplayRecording::new(header, frames, vec![]);

        let json = recording.to_json();
        assert!(!json.is_empty());

        let restored = ReplayRecording::from_json(&json).unwrap();
        assert_eq!(restored.header.replay_id, "test");
        assert_eq!(restored.frames.len(), 1);
        assert!(restored.verify());
    }

    #[test]
    fn test_playback_creation() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![
            InputFrame::new(0, InputType::Move, "{}"),
            InputFrame::new(1, InputType::Attack, "{}"),
        ];
        let recording = ReplayRecording::new(header, frames, vec![]);
        let playback = ReplayPlayback::new(&recording);

        assert_eq!(playback.state, PlaybackState::Idle);
        assert_eq!(playback.total_frames, 2);
        assert_eq!(playback.speed, 1.0);
    }

    #[test]
    fn test_playback_controls() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let recording = ReplayRecording::new(header, vec![], vec![]);
        let mut playback = ReplayPlayback::new(&recording);

        playback.play();
        assert_eq!(playback.state, PlaybackState::Playing);

        playback.pause();
        assert_eq!(playback.state, PlaybackState::Paused);

        playback.stop();
        assert_eq!(playback.state, PlaybackState::Idle);
        assert_eq!(playback.current_tick, 0);
    }

    #[test]
    fn test_playback_seek() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let recording = ReplayRecording::new(header, vec![], vec![]);
        let mut playback = ReplayPlayback::new(&recording);

        playback.seek(500);
        assert_eq!(playback.current_tick, 500);
        assert_eq!(playback.state, PlaybackState::Seeking);
    }

    #[test]
    fn test_playback_speed() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let recording = ReplayRecording::new(header, vec![], vec![]);
        let mut playback = ReplayPlayback::new(&recording);

        playback.set_speed(2.0);
        assert_eq!(playback.speed, 2.0);

        playback.set_speed(0.5);
        assert_eq!(playback.speed, 0.5);

        playback.set_speed(20.0);
        assert_eq!(playback.speed, 10.0); // clamped to max
    }

    #[test]
    fn test_playback_progress() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![
            InputFrame::new(0, InputType::Move, "{}"),
            InputFrame::new(1, InputType::Attack, "{}"),
        ];
        let recording = ReplayRecording::new(header, frames, vec![]);
        let mut playback = ReplayPlayback::new(&recording);

        assert_eq!(playback.progress(), 0.0);

        playback.current_frame_idx = 1;
        assert!((playback.progress() - 0.5).abs() < 0.01);

        playback.current_frame_idx = 2;
        assert_eq!(playback.progress(), 1.0);
    }

    #[test]
    fn test_playback_advance() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![
            InputFrame::new(0, InputType::Move, "{}"),
            InputFrame::new(1, InputType::Attack, "{}"),
        ];
        let recording = ReplayRecording::new(header, frames, vec![]);
        let mut playback = ReplayPlayback::new(&recording);

        playback.play();
        let frame1 = playback.advance(&recording);
        assert!(frame1.is_some());
        assert_eq!(frame1.unwrap().tick, 0);

        let frame2 = playback.advance(&recording);
        assert!(frame2.is_some());
        assert_eq!(frame2.unwrap().tick, 1);

        let frame3 = playback.advance(&recording);
        assert!(frame3.is_none());
        assert_eq!(playback.state, PlaybackState::Finished);
    }

    #[test]
    fn test_playback_loop() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![InputFrame::new(0, InputType::Move, "{}")];
        let recording = ReplayRecording::new(header, frames, vec![]);
        let mut playback = ReplayPlayback::new(&recording);
        playback.loop_playback = true;

        playback.play();
        playback.advance(&recording);
        assert_eq!(playback.state, PlaybackState::Finished);

        let frame = playback.advance(&recording);
        assert!(frame.is_some()); // looped back to start
        assert_eq!(playback.current_frame_idx, 1);
        assert_eq!(playback.state, PlaybackState::Playing);
    }

    #[test]
    fn test_recorder_start_stop() {
        let mut recorder = ReplayRecorder::default();
        let seed = TowerSeed { seed: 42 };

        recorder.start_recording(&seed, 10, "Player1", r#"{"weapon":"Sword"}"#, 0);
        assert!(recorder.is_recording());

        recorder.record_frame(0, InputType::Move, "{}");
        recorder.record_frame(1, InputType::Attack, "{}");

        let recording = recorder.stop_recording(ReplayOutcome::Victory, vec![], 100);
        assert!(recording.is_some());

        let rec = recording.unwrap();
        assert_eq!(rec.frames.len(), 2);
        assert_eq!(rec.header.outcome, ReplayOutcome::Victory);
        assert_eq!(rec.header.duration_ticks, 100);
    }

    #[test]
    fn test_recorder_cancel() {
        let mut recorder = ReplayRecorder::default();
        let seed = TowerSeed { seed: 42 };

        recorder.start_recording(&seed, 10, "Player1", "{}", 0);
        recorder.record_frame(0, InputType::Move, "{}");

        recorder.cancel_recording();
        assert!(!recorder.is_recording());
        assert!(recorder.current_header.is_none());
        assert!(recorder.frames.is_empty());
    }

    #[test]
    fn test_recorder_not_recording() {
        let mut recorder = ReplayRecorder::default();
        recorder.record_frame(0, InputType::Move, "{}");
        assert_eq!(
            recorder.frames.len(),
            0,
            "Should not record when not recording"
        );
    }

    #[test]
    fn test_replay_snapshot() {
        let mut recorder = ReplayRecorder::default();
        let seed = TowerSeed { seed: 42 };

        recorder.start_recording(&seed, 10, "Player1", "{}", 0);
        recorder.record_frame(0, InputType::Move, "{}");
        recorder.record_frame(1, InputType::Attack, "{}");

        let snapshot = ReplaySnapshot::capture(&recorder);
        assert!(snapshot.is_recording);
        assert!(snapshot.current_replay_id.is_some());
        assert_eq!(snapshot.recorded_frames, 2);
        assert_eq!(snapshot.available_input_types.len(), 8);

        let json = snapshot.to_json();
        assert!(json.contains("is_recording"));
    }

    #[test]
    fn test_all_input_types() {
        let types = vec![
            InputType::Move,
            InputType::Attack,
            InputType::Parry,
            InputType::Dodge,
            InputType::UseAbility,
            InputType::Interact,
            InputType::Jump,
            InputType::ChangeWeapon,
        ];

        for (i, input_type) in types.iter().enumerate() {
            let frame = InputFrame::new(i as u64, *input_type, "{}");
            assert_eq!(frame.tick, i as u64);
            assert!(frame.hash() > 0);
        }
    }

    #[test]
    fn test_all_replay_outcomes() {
        let outcomes = vec![
            ReplayOutcome::InProgress,
            ReplayOutcome::Victory,
            ReplayOutcome::Death,
            ReplayOutcome::Abandoned,
        ];

        for outcome in outcomes {
            let mut header = ReplayHeader::new("test", 42, 1, "P1", "{}");
            header.outcome = outcome;
            assert_eq!(header.outcome, outcome);
        }
    }

    #[test]
    fn test_estimated_size() {
        let header = ReplayHeader::new("test", 42, 1, "P1", "{}");
        let frames = vec![
            InputFrame::new(0, InputType::Move, "{}"),
            InputFrame::new(1, InputType::Attack, "{}"),
        ];
        let recording = ReplayRecording::new(header, frames, vec![]);
        let size = recording.estimated_size();
        assert!(size > 200); // header + frames
    }
}
