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
    }
}

joinable!(photos -> users (owner));

allow_tables_to_appear_in_same_query!(
    photos,
    users,
);
