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
        created_at -> Timestamp,
        updated_at -> Timestamp,
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
        updated_at -> Timestamp,
        joined -> Timestamp,
    }
}

joinable!(photo_attrs -> photos (photo_id));
joinable!(photos -> users (owner));

allow_tables_to_appear_in_same_query!(
    photo_attrs,
    photos,
    users,
);
