/// Current state of the AI.
pub(crate) enum AiState {
    Pursue,          // Entity to target.
    Wander(f32, u8), // Range to wander.
    Idle,            // Do nothing.
}

/// Basic AI that can be modified.
pub(crate) struct BasicAi {
    pub state: AiState,
}

impl BasicAi {
    pub fn new() -> Self {
        Self {
            state: AiState::Wander(3.0, 1),
            // state: AiState::Pursue,
        }
    }

    pub fn set_state(&mut self, state: AiState) {
        self.state = state;
    }
}
