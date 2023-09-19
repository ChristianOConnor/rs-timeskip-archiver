// @generated automatically by Diesel CLI.

diesel::table! {
    files (id) {
        id -> Integer,
        file_name -> Text,
        sha256 -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        profile_id -> Integer,
    }
}

diesel::table! {
    profiles (id) {
        id -> Integer,
        profile_name -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(files -> profiles (profile_id));

diesel::allow_tables_to_appear_in_same_query!(
    files,
    profiles,
);
