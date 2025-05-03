// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Uuid,
        team_id -> Int4,
        #[max_length = 64]
        full_name -> Varchar,
        #[max_length = 16]
        student_id -> Varchar,
        #[max_length = 64]
        email -> Varchar,
        #[max_length = 128]
        password -> Varchar,
        #[max_length = 12]
        phone -> Varchar,
        #[max_length = 128]
        school -> Varchar,
        dob -> Date,
        is_banned -> Bool,
        created_at -> Timestamp,
    }
}

diesel::table! {
    questions (id) {
        id -> Uuid,
        room_id -> Int4,
        score -> Int4,
        time_limit -> Int4,
        memory_limit -> Int4,
    }
}

diesel::table! {
    rooms (id) {
        id -> Int4,
        #[max_length = 12]
        code -> Bpchar,
        kind -> Int4,
        open_time -> Timestamp,
        close_time -> Timestamp,
        created_at -> Timestamp,
    }
}

diesel::table! {
    submission_details (submission_history_id, test_case_id) {
        submission_history_id -> Uuid,
        test_case_id -> Uuid,
        status -> Int4,
        run_time -> Int4,
        stdout -> Nullable<Text>,
        stderr -> Nullable<Text>,
    }
}

diesel::table! {
    submission_histories (id) {
        id -> Uuid,
        question_id -> Uuid,
        team_id -> Int4,
        language -> Int4,
        code -> Text,
        score -> Nullable<Float4>,
        compilation_error -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    teams (id) {
        id -> Int4,
        room_id -> Int4,
        #[max_length = 128]
        name -> Varchar,
        total_score -> Float4,
        penalty -> Int4,
    }
}

diesel::table! {
    test_cases (id) {
        id -> Uuid,
        question_id -> Uuid,
        #[max_length = 64]
        input_path -> Varchar,
        #[max_length = 64]
        output_path -> Varchar,
        is_hidden -> Bool,
    }
}

diesel::joinable!(accounts -> teams (team_id));
diesel::joinable!(questions -> rooms (room_id));
diesel::joinable!(submission_details -> submission_histories (submission_history_id));
diesel::joinable!(submission_details -> test_cases (test_case_id));
diesel::joinable!(submission_histories -> questions (question_id));
diesel::joinable!(submission_histories -> teams (team_id));
diesel::joinable!(teams -> rooms (room_id));
diesel::joinable!(test_cases -> questions (question_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    questions,
    rooms,
    submission_details,
    submission_histories,
    teams,
    test_cases,
);
