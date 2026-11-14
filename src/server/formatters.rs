use crate::hevy::types::RoutineSet;
use crate::hevy::{ExerciseTemplate, Routine, RoutineFolder, Workout};

pub(crate) fn format_workout(workout: &Workout) -> String {
    let mut output = format!(
        "Workout: {}\nID: {}\nStart: {}\n",
        workout.title, workout.id, workout.start_time
    );

    if let Some(routine_id) = &workout.routine_id {
        output.push_str(&format!("Routine ID: {}\n", routine_id));
    }
    if let Some(end) = &workout.end_time {
        output.push_str(&format!("End: {}\n", end));
    }
    if let Some(desc) = &workout.description {
        output.push_str(&format!("Description: {}\n", desc));
    }

    output.push_str(&format!("\nExercises ({}):\n", workout.exercises.len()));

    for exercise in &workout.exercises {
        output.push_str(&format!(
            "  {}. Exercise Template ID: {}\n",
            exercise.index + 1,
            exercise.exercise_template_id
        ));
        if let Some(notes) = &exercise.notes {
            output.push_str(&format!("     Notes: {}\n", notes));
        }
        for set in &exercise.sets {
            let mut set_info = format!("     Set {}: ", set.index + 1);
            if let Some(w) = set.weight_kg {
                set_info.push_str(&format!("{:.1}kg ", w));
            }
            if let Some(r) = set.reps {
                set_info.push_str(&format!("x{} reps ", r));
            }
            if let Some(d) = set.duration_seconds {
                set_info.push_str(&format!("{}s ", d));
            }
            output.push_str(&format!("{}\n", set_info.trim()));
        }
    }

    output
}

pub(crate) fn format_routine(routine: &Routine) -> String {
    let mut output = format!("Routine: {}\nID: {}\n", routine.title, routine.id);

    if let Some(folder_id) = &routine.folder_id {
        output.push_str(&format!("Folder ID: {}\n", folder_id));
    }
    if let Some(notes) = &routine.notes {
        output.push_str(&format!("Notes: {}\n", notes));
    }
    if let Some(created) = &routine.created_at {
        output.push_str(&format!("Created: {}\n", created));
    }

    output.push_str(&format!("\nExercises ({}):\n", routine.exercises.len()));

    for exercise in &routine.exercises {
        output.push_str(&format!(
            "  {}. Exercise Template ID: {}\n",
            exercise.index + 1,
            exercise.exercise_template_id
        ));
        if let Some(notes) = &exercise.notes {
            output.push_str(&format!("     Notes: {}\n", notes));
        }
        if let Some(superset_id) = exercise.superset_id {
            output.push_str(&format!("     Superset ID: {}\n", superset_id));
        }
        if let Some(rest) = exercise.rest_seconds {
            output.push_str(&format!("     Rest: {}s\n", rest));
        }
        output.push_str(&format!("     Sets: {}\n", exercise.sets.len()));
        for set in &exercise.sets {
            output.push_str(&format!("     {}\n", format_routine_set(set)));
        }
    }

    output
}

fn format_routine_set(set: &RoutineSet) -> String {
    let mut parts = vec![format!("Set {}:", set.index + 1)];
    if let Some(t) = &set.set_type {
        parts.push(format!("[{:?}]", t));
    }
    if let Some(w) = set.weight_kg {
        parts.push(format!("{:.1}kg", w));
    }
    if let Some(r) = set.reps {
        parts.push(format!("x{} reps", r));
    }
    if let Some(rr) = &set.rep_range {
        parts.push(format!("{}-{} reps", rr.start, rr.end));
    }
    if let Some(d) = set.duration_seconds {
        parts.push(format!("{}s", d));
    }
    if let Some(d) = set.distance_meters {
        parts.push(format!("{}m", d));
    }
    if let Some(c) = set.custom_metric {
        parts.push(format!("custom: {}", c));
    }
    parts.join(" ")
}

pub(crate) fn format_exercise_template(template: &ExerciseTemplate) -> String {
    let mut output = format!(
        "Exercise: {}\nID: {}\nType: {:?}\nPrimary Muscle: {:?}\n",
        template.title, template.id, template.exercise_type, template.primary_muscle_group
    );

    if !template.secondary_muscle_groups.is_empty() {
        output.push_str(&format!(
            "Secondary Muscles: {:?}\n",
            template.secondary_muscle_groups
        ));
    }

    if let Some(equipment) = &template.equipment {
        output.push_str(&format!("Equipment: {:?}\n", equipment));
    }

    output.push_str(&format!(
        "Custom Exercise: {}\n",
        if template.is_custom { "Yes" } else { "No" }
    ));

    output
}

pub(crate) fn format_folder(folder: &RoutineFolder) -> String {
    let mut output = format!(
        "Folder: {}\nID: {}\nIndex: {}\n",
        folder.title, folder.id, folder.index
    );

    if let Some(created) = &folder.created_at {
        output.push_str(&format!("Created: {}\n", created));
    }
    if let Some(updated) = &folder.updated_at {
        output.push_str(&format!("Updated: {}\n", updated));
    }

    output
}

pub(crate) fn format_exercise_history(response: &crate::hevy::ExerciseHistoryResponse) -> String {
    if response.exercise_history.is_empty() {
        return "No exercise history found.".to_string();
    }

    let mut output = String::new();
    for entry in &response.exercise_history {
        output.push_str(&format_exercise_history_entry(entry));
        output.push_str("---\n");
    }
    output
}

fn format_exercise_history_entry(entry: &crate::hevy::ExerciseHistoryEntry) -> String {
    let mut output = format!(
        "Workout: {} (ID: {})\n",
        entry.workout_title, entry.workout_id
    );
    output.push_str(&format!("Start: {}\n", entry.workout_start_time));
    if let Some(end) = entry.workout_end_time {
        output.push_str(&format!("End: {}\n", end));
    }
    output.push_str(&format!(
        "Exercise Template ID: {}\n",
        entry.exercise_template_id
    ));
    if let Some(weight) = entry.weight_kg {
        output.push_str(&format!("Weight: {:.1}kg\n", weight));
    }
    if let Some(reps) = entry.reps {
        output.push_str(&format!("Reps: {}\n", reps));
    }
    if let Some(rpe) = entry.rpe {
        output.push_str(&format!("RPE: {}\n", rpe));
    }
    output
}

pub(crate) fn format_workouts_page(
    response: &crate::hevy::PaginatedResponse<crate::hevy::types::WorkoutsResponse>,
) -> String {
    let mut output = format!("Page {} of {}\n\n", response.page, response.page_count);
    if response.data.workouts.is_empty() {
        output.push_str("No workouts found.");
        return output;
    }

    for workout in &response.data.workouts {
        output.push_str(&format_workout(workout));
        output.push_str("\n---\n\n");
    }
    output
}

pub(crate) fn format_workout_events(response: &crate::hevy::WorkoutEventsResponse) -> String {
    let mut output = format!("Page {} of {}\n\n", response.page, response.page_count);
    if response.events.is_empty() {
        output.push_str("No workout events found.");
        return output;
    }

    for event in &response.events {
        output.push_str(&format_workout_event(event));
        output.push_str("---\n");
    }

    output
}

fn format_workout_event(event: &crate::hevy::WorkoutEvent) -> String {
    match event.event_type.as_str() {
        "updated" => format_updated_workout_event(event),
        "deleted" => format_deleted_workout_event(event),
        other => format!("Event: {}\n", other),
    }
}

fn format_updated_workout_event(event: &crate::hevy::WorkoutEvent) -> String {
    match &event.workout {
        Some(workout) => format!("Updated workout: {} (ID: {})\n", workout.title, workout.id),
        None => "Updated workout: missing details\n".to_string(),
    }
}

fn format_deleted_workout_event(event: &crate::hevy::WorkoutEvent) -> String {
    match (&event.id, &event.deleted_at) {
        (Some(id), Some(deleted_at)) => format!("Deleted workout: {} at {}\n", id, deleted_at),
        (Some(id), None) => format!("Deleted workout: {}\n", id),
        (None, _) => "Deleted workout: missing ID\n".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn routine(json: serde_json::Value) -> Routine {
        serde_json::from_value(json).expect("valid routine json")
    }

    #[test]
    fn format_routine_includes_rest_superset_and_sets() {
        let r = routine(serde_json::json!({
            "id": "r1",
            "title": "Push",
            "exercises": [
                {
                    "index": 0,
                    "exercise_template_id": "T1",
                    "superset_id": 2,
                    "rest_seconds": 90,
                    "sets": [
                        {"index": 0, "type": "warmup", "weight_kg": 20.0, "reps": 10},
                        {"index": 1, "type": "normal", "weight_kg": 60.5, "rep_range": {"start": 6, "end": 8}}
                    ]
                },
                {
                    "index": 1,
                    "exercise_template_id": "T2",
                    "sets": [{"index": 0, "type": "normal", "duration_seconds": 45}]
                }
            ]
        }));

        let out = format_routine(&r);

        assert!(out.contains("     Superset ID: 2\n"), "{out}");
        assert!(out.contains("     Rest: 90s\n"), "{out}");
        assert!(out.contains("     Sets: 2\n"), "{out}");
        assert!(out.contains("Set 1: [Warmup] 20.0kg x10 reps"), "{out}");
        assert!(out.contains("Set 2: [Normal] 60.5kg 6-8 reps"), "{out}");
        assert!(out.contains("Set 1: [Normal] 45s"), "{out}");
    }

    #[test]
    fn format_routine_omits_absent_rest_and_superset() {
        let r = routine(serde_json::json!({
            "id": "r2",
            "title": "Pull",
            "exercises": [{"index": 0, "exercise_template_id": "T1", "sets": []}]
        }));

        let out = format_routine(&r);

        assert!(!out.contains("Superset ID"), "{out}");
        assert!(!out.contains("Rest:"), "{out}");
        assert!(out.contains("Sets: 0"), "{out}");
    }
}
