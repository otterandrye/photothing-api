table! {
    users (id) {
        id -> Int4,
        email -> Varchar,
        uuid -> Varchar,
        password -> Varchar,
        name -> Nullable<Varchar>,
        subscription_expires -> Nullable<Date>,
    }
}
