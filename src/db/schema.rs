use diesel::macros::{table, joinable, allow_tables_to_appear_in_same_query};

table! {
    album_membership (photo_id, album_id) {
        photo_id -> Int4,
        album_id -> Int4,
        ordering -> Nullable<Int2>,
        caption -> Nullable<Text>,
        updated_at -> Timestamptz,
    }
}

table! {
    password_resets (uuid) {
        uuid -> Varchar,
        user_id -> Int4,
        created_at -> Timestamptz,
    }
}

table! {
    photo_albums (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Nullable<Text>,
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
    published_albums (id) {
        id -> Int4,
        album_id -> Int4,
        user_id -> Int4,
        active -> Bool,
        created_at -> Timestamptz,
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

joinable!(album_membership -> photo_albums (album_id));
joinable!(album_membership -> photos (photo_id));
joinable!(password_resets -> users (user_id));
joinable!(photo_albums -> users (user_id));
joinable!(photo_attrs -> photos (photo_id));
joinable!(photos -> users (owner));
joinable!(published_albums -> photo_albums (album_id));
joinable!(published_albums -> users (user_id));

allow_tables_to_appear_in_same_query!(
    album_membership,
    password_resets,
    photo_albums,
    photo_attrs,
    photos,
    published_albums,
    users,
);
