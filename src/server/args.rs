//! Typed tool argument structs for the rmcp server (schemars + serde).

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::hevy::{
    CreateExerciseTemplateInput, Equipment, ExerciseType, MuscleGroup, RepRange,
    RoutineExerciseInput, RoutineFolderInput, RoutineInput, RoutineSetInput, SetInput, SetType,
    WorkoutExerciseInput, WorkoutInput,
};

/// Deserializes `null` as the type's default (matches the legacy lenient parsing).
fn default_on_null<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

/// Arguments for paginated list tools.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[schemars(extend("type" = "number"))]
    pub page: Option<u32>,
    /// Number of items per page (max 10)
    #[schemars(extend("type" = "number"))]
    pub page_size: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkoutIdParams {
    /// The workout ID
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkoutEventsParams {
    /// Page number (1-indexed)
    #[schemars(extend("type" = "number"))]
    pub page: Option<u32>,
    /// Number of items per page (max 10)
    #[schemars(extend("type" = "number"))]
    pub page_size: Option<u32>,
    /// ISO 8601 timestamp to start from
    pub since: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExerciseHistoryParams {
    /// Exercise template ID
    pub exercise_template_id: String,
    /// Optional ISO 8601 start date
    pub start_date: Option<String>,
    /// Optional ISO 8601 end date
    pub end_date: Option<String>,
}

/// Type of set.
#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SetTypeParam {
    Normal,
    Warmup,
    Dropset,
    Failure,
}

impl From<SetTypeParam> for SetType {
    fn from(value: SetTypeParam) -> Self {
        match value {
            SetTypeParam::Normal => SetType::Normal,
            SetTypeParam::Warmup => SetType::Warmup,
            SetTypeParam::Dropset => SetType::Dropset,
            SetTypeParam::Failure => SetType::Failure,
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetParams {
    /// Type of set
    #[schemars(required)]
    pub set_type: Option<SetTypeParam>,
    /// Weight in kg
    pub weight_kg: Option<f64>,
    /// Number of reps
    #[schemars(extend("type" = "number"))]
    pub reps: Option<u32>,
    /// Duration in seconds
    #[schemars(extend("type" = "number"))]
    pub duration_seconds: Option<u32>,
    /// Distance in meters
    pub distance_meters: Option<f64>,
    /// Rate of perceived exertion (1-10)
    pub rpe: Option<f64>,
    /// Custom metric value
    pub custom_metric: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkoutExerciseParams {
    /// Exercise template ID
    pub exercise_template_id: String,
    /// Superset group ID
    #[schemars(extend("type" = "number"))]
    pub superset_id: Option<u32>,
    /// Exercise notes
    pub notes: Option<String>,
    /// Sets in the exercise
    #[serde(default, deserialize_with = "default_on_null")]
    #[schemars(with = "Vec<SetParams>")]
    pub sets: Vec<SetParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateWorkoutParams {
    /// Workout title
    pub title: String,
    /// Whether the workout is private
    #[serde(default)]
    pub is_private: bool,
    /// Start time in ISO 8601 format
    pub start_time: DateTime<Utc>,
    /// End time in ISO 8601 format
    #[schemars(required)]
    pub end_time: Option<DateTime<Utc>>,
    /// Optional description
    pub description: Option<String>,
    /// Exercises in the workout
    #[serde(default, deserialize_with = "default_on_null")]
    #[schemars(with = "Vec<WorkoutExerciseParams>")]
    pub exercises: Vec<WorkoutExerciseParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateWorkoutParams {
    /// The workout ID to update
    pub id: String,
    /// Workout title
    pub title: String,
    /// Whether the workout is private
    #[serde(default)]
    pub is_private: bool,
    /// Start time in ISO 8601 format
    pub start_time: DateTime<Utc>,
    /// End time in ISO 8601 format
    #[schemars(required)]
    pub end_time: Option<DateTime<Utc>>,
    /// Optional description
    pub description: Option<String>,
    /// Exercises in the workout
    #[serde(default, deserialize_with = "default_on_null")]
    #[schemars(with = "Vec<WorkoutExerciseParams>")]
    pub exercises: Vec<WorkoutExerciseParams>,
}

impl From<SetParams> for SetInput {
    fn from(v: SetParams) -> Self {
        Self {
            set_type: v.set_type.map(Into::into),
            weight_kg: v.weight_kg,
            reps: v.reps,
            duration_seconds: v.duration_seconds,
            distance_meters: v.distance_meters,
            rpe: v.rpe,
            custom_metric: v.custom_metric,
        }
    }
}

impl From<WorkoutExerciseParams> for WorkoutExerciseInput {
    fn from(v: WorkoutExerciseParams) -> Self {
        Self {
            exercise_template_id: v.exercise_template_id,
            superset_id: v.superset_id,
            notes: v.notes,
            sets: v.sets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CreateWorkoutParams> for WorkoutInput {
    fn from(v: CreateWorkoutParams) -> Self {
        Self {
            title: v.title,
            description: v.description,
            is_private: v.is_private,
            start_time: v.start_time,
            end_time: v.end_time,
            exercises: v.exercises.into_iter().map(Into::into).collect(),
        }
    }
}

impl UpdateWorkoutParams {
    /// Splits into the workout ID and the replacement workout data.
    pub fn into_parts(self) -> (String, WorkoutInput) {
        let input = WorkoutInput {
            title: self.title,
            description: self.description,
            is_private: self.is_private,
            start_time: self.start_time,
            end_time: self.end_time,
            exercises: self.exercises.into_iter().map(Into::into).collect(),
        };
        (self.id, input)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RoutineIdParams {
    /// The routine ID
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FolderIdParams {
    /// The routine folder ID
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TemplateIdParams {
    /// The exercise template ID
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RepRangeParams {
    /// Min reps
    #[schemars(extend("type" = "number"))]
    pub start: u32,
    /// Max reps
    #[schemars(extend("type" = "number"))]
    pub end: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RoutineSetParams {
    /// Type of set
    #[schemars(required)]
    pub set_type: Option<SetTypeParam>,
    /// Weight in kg
    pub weight_kg: Option<f64>,
    /// Number of reps
    #[schemars(extend("type" = "number"))]
    pub reps: Option<u32>,
    /// Rep range for the set
    pub rep_range: Option<RepRangeParams>,
    /// Duration in seconds
    #[schemars(extend("type" = "number"))]
    pub duration_seconds: Option<u32>,
    /// Distance in meters
    pub distance_meters: Option<f64>,
    /// Custom metric value
    pub custom_metric: Option<f64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RoutineExerciseParams {
    /// Exercise template ID
    pub exercise_template_id: String,
    /// Superset group ID
    #[schemars(extend("type" = "number"))]
    pub superset_id: Option<u32>,
    /// Rest time between sets in seconds
    #[schemars(extend("type" = "number"))]
    pub rest_seconds: Option<u32>,
    /// Exercise notes
    pub notes: Option<String>,
    /// Set templates in the exercise
    #[serde(default, deserialize_with = "default_on_null")]
    #[schemars(with = "Vec<RoutineSetParams>")]
    pub sets: Vec<RoutineSetParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateRoutineParams {
    /// Routine title
    pub title: String,
    /// Folder ID to place the routine in (use get_routine_folders to list available folders)
    #[schemars(required)]
    pub folder_id: Option<String>,
    /// Optional notes
    pub notes: Option<String>,
    /// Exercises in the routine
    #[serde(default, deserialize_with = "default_on_null")]
    #[schemars(with = "Vec<RoutineExerciseParams>")]
    pub exercises: Vec<RoutineExerciseParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateRoutineParams {
    /// The routine ID to update
    pub id: String,
    /// Routine title
    pub title: String,
    /// Optional notes
    pub notes: Option<String>,
    /// Exercises in the routine
    #[serde(default, deserialize_with = "default_on_null")]
    #[schemars(with = "Vec<RoutineExerciseParams>")]
    pub exercises: Vec<RoutineExerciseParams>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateRoutineFolderParams {
    /// Folder title
    pub title: String,
    /// Folder position index (accepted but not advertised in the schema)
    #[serde(default)]
    #[schemars(skip)]
    pub index: Option<u32>,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseTypeParam {
    WeightReps,
    Reps,
    Duration,
    WeightDuration,
    DistanceDuration,
    Weight,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentParam {
    Barbell,
    Dumbbell,
    Machine,
    Cable,
    Bodyweight,
    Band,
    Kettlebell,
    TrapBar,
    SmithMachine,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MuscleGroupParam {
    Abdominals,
    Abductors,
    Adductors,
    Biceps,
    Calves,
    Cardio,
    Chest,
    Forearms,
    FullBody,
    Glutes,
    Hamstrings,
    Lats,
    LowerBack,
    Neck,
    Quadriceps,
    Shoulders,
    Traps,
    Triceps,
    UpperBack,
    Other,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateExerciseTemplateParams {
    /// Exercise title
    pub title: String,
    /// Exercise type
    pub exercise_type: ExerciseTypeParam,
    /// Equipment category
    pub equipment_category: EquipmentParam,
    /// Primary muscle group
    pub muscle_group: MuscleGroupParam,
    /// Optional additional muscle groups
    pub other_muscles: Option<Vec<MuscleGroupParam>>,
}

impl From<ExerciseTypeParam> for ExerciseType {
    fn from(v: ExerciseTypeParam) -> Self {
        match v {
            ExerciseTypeParam::WeightReps => Self::WeightReps,
            ExerciseTypeParam::Reps => Self::Reps,
            ExerciseTypeParam::Duration => Self::Duration,
            ExerciseTypeParam::WeightDuration => Self::WeightDuration,
            ExerciseTypeParam::DistanceDuration => Self::DistanceDuration,
            ExerciseTypeParam::Weight => Self::Weight,
        }
    }
}

impl From<EquipmentParam> for Equipment {
    fn from(v: EquipmentParam) -> Self {
        match v {
            EquipmentParam::Barbell => Self::Barbell,
            EquipmentParam::Dumbbell => Self::Dumbbell,
            EquipmentParam::Machine => Self::Machine,
            EquipmentParam::Cable => Self::Cable,
            EquipmentParam::Bodyweight => Self::Bodyweight,
            EquipmentParam::Band => Self::Band,
            EquipmentParam::Kettlebell => Self::Kettlebell,
            EquipmentParam::TrapBar => Self::TrapBar,
            EquipmentParam::SmithMachine => Self::SmithMachine,
            EquipmentParam::Other => Self::Other,
        }
    }
}

impl From<MuscleGroupParam> for MuscleGroup {
    fn from(v: MuscleGroupParam) -> Self {
        match v {
            MuscleGroupParam::Abdominals => Self::Abdominals,
            MuscleGroupParam::Abductors => Self::Abductors,
            MuscleGroupParam::Adductors => Self::Adductors,
            MuscleGroupParam::Biceps => Self::Biceps,
            MuscleGroupParam::Calves => Self::Calves,
            MuscleGroupParam::Cardio => Self::Cardio,
            MuscleGroupParam::Chest => Self::Chest,
            MuscleGroupParam::Forearms => Self::Forearms,
            MuscleGroupParam::FullBody => Self::FullBody,
            MuscleGroupParam::Glutes => Self::Glutes,
            MuscleGroupParam::Hamstrings => Self::Hamstrings,
            MuscleGroupParam::Lats => Self::Lats,
            MuscleGroupParam::LowerBack => Self::LowerBack,
            MuscleGroupParam::Neck => Self::Neck,
            MuscleGroupParam::Quadriceps => Self::Quadriceps,
            MuscleGroupParam::Shoulders => Self::Shoulders,
            MuscleGroupParam::Traps => Self::Traps,
            MuscleGroupParam::Triceps => Self::Triceps,
            MuscleGroupParam::UpperBack => Self::UpperBack,
            MuscleGroupParam::Other => Self::Other,
        }
    }
}

impl From<CreateExerciseTemplateParams> for CreateExerciseTemplateInput {
    fn from(v: CreateExerciseTemplateParams) -> Self {
        Self {
            title: v.title,
            exercise_type: v.exercise_type.into(),
            equipment_category: v.equipment_category.into(),
            muscle_group: v.muscle_group.into(),
            other_muscles: v
                .other_muscles
                .map(|m| m.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<CreateRoutineFolderParams> for RoutineFolderInput {
    fn from(v: CreateRoutineFolderParams) -> Self {
        Self {
            title: v.title,
            index: v.index,
        }
    }
}

impl From<RepRangeParams> for RepRange {
    fn from(v: RepRangeParams) -> Self {
        Self {
            start: v.start,
            end: v.end,
        }
    }
}

impl From<RoutineSetParams> for RoutineSetInput {
    fn from(v: RoutineSetParams) -> Self {
        Self {
            set_type: v.set_type.map(Into::into),
            weight_kg: v.weight_kg,
            reps: v.reps,
            rep_range: v.rep_range.map(Into::into),
            duration_seconds: v.duration_seconds,
            distance_meters: v.distance_meters,
            custom_metric: v.custom_metric,
        }
    }
}

impl From<RoutineExerciseParams> for RoutineExerciseInput {
    fn from(v: RoutineExerciseParams) -> Self {
        Self {
            exercise_template_id: v.exercise_template_id,
            superset_id: v.superset_id,
            rest_seconds: v.rest_seconds,
            notes: v.notes,
            sets: v.sets.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CreateRoutineParams> for RoutineInput {
    fn from(v: CreateRoutineParams) -> Self {
        Self {
            title: v.title,
            folder_id: v.folder_id,
            notes: v.notes,
            exercises: v.exercises.into_iter().map(Into::into).collect(),
        }
    }
}

impl UpdateRoutineParams {
    /// Splits into the routine ID and the replacement routine data.
    pub fn into_parts(self) -> (String, RoutineInput) {
        let input = RoutineInput {
            title: self.title,
            folder_id: None,
            notes: self.notes,
            exercises: self.exercises.into_iter().map(Into::into).collect(),
        };
        (self.id, input)
    }
}
