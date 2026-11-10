//! AppState enum and state transition queries.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadTarget {
    Overworld,
    Nether,
    TheEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    LoadingLevel,
    BuildingTerrain,
    GeneratingStructures,
    Lighting,
    BuildingMeshes,
}

impl LoadStage {
    pub fn description(&self) -> &'static str {
        match self {
            LoadStage::LoadingLevel => "Loading level",
            LoadStage::BuildingTerrain => "Building terrain",
            LoadStage::GeneratingStructures => "Generating structures",
            LoadStage::Lighting => "Calculating lighting",
            LoadStage::BuildingMeshes => "Building meshes",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Title,
    SelectWorld,
    CreateWorld,
    Options,
    VideoSettings,
    Controls,
    Loading {
        target: LoadTarget,
        stage: LoadStage,
        done: usize,
        total: usize,
    },
    InGame,
    Paused,
    Dead,
}

impl AppState {
    /// Check whether the cursor must be unlocked/visible for GUI interaction
    pub fn requires_mouse_cursor(&self) -> bool {
        !matches!(self, AppState::InGame)
    }

    /// Check whether the underlying 3D world should be rendered behind the GUI
    pub fn renders_world(&self) -> bool {
        matches!(self, AppState::InGame | AppState::Paused | AppState::Dead)
    }

    /// Check whether the simulation is currently active (advancing player/physics)
    pub fn is_simulating(&self) -> bool {
        matches!(self, AppState::InGame)
    }
}
