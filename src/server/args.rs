//! Typed tool argument structs for the rmcp server (schemars + serde).

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::hevy::{SetInput, SetType, WorkoutExerciseInput, WorkoutInput};

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

/// Arguments for tools that take a single entity ID.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct IdParams {
    /// The ID of the entity
    pub id: String,
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
