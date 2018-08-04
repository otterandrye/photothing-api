table! {
    password_resets (uuid) {
        uuid -> Varchar,
        user_id -> Int4,
        created_at -> Timestamptz,
    }
}

table! {
    photo_attrs (photo_id, key) {
        photo_id -> Int4,
        key -> Varchar,
        value -> Varchar,
        updated_at -> Timestamp,
    }
}

table! {
    photos (id) {
        id -> Int4,
        uuid -> Varchar,
        owner -> Int4,
        present -> Nullable<Bool>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    users (id) {
        id -> Int4,
        email -> Varchar,
        uuid -> Varchar,
        password -> Varchar,
        name -> Nullable<Varchar>,
        subscription_expires -> Nullable<Date>,
        updated_at -> Timestamptz,
        joined -> Timestamptz,
    }
}

joinable!(password_resets -> users (user_id));
joinable!(photo_attrs -> photos (photo_id));
joinable!(photos -> users (owner));

allow_tables_to_appear_in_same_query!(
    password_resets,
    photo_attrs,
    photos,
    users,
);
