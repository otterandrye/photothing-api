use chrono::NaiveDateTime;
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;

use db::schema::{photos, photo_attrs};
use db::user::User;
use ::util::uuid;

// Main Photo object, keeps track of whether the file exists on S3 and who uploaded it
#[derive(Queryable)]
pub struct Photo {
    pub id: i32,
    pub uuid: String,
    pub owner: i32, // users.id
    pub present: Option<bool>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name="photos"]
pub struct NewPhoto {
    uuid: String,
    owner: i32,
}

impl NewPhoto {
    pub fn new(owner: User) -> NewPhoto {
        NewPhoto {
            owner: owner.id,
            uuid: uuid().0,
        }
    }

    pub fn insert(self, db: &PgConnection) -> Result<Photo, Error> {
        use db::schema::photos::dsl::*;
        let created = diesel::insert_into(photos).values(&self).get_result(db)?;
        Ok(created)
    }
}

// Attributes object for storing metadata about a photo
#[derive(Queryable)]
pub struct PhotoAttr {
    photo_id: i32,
    pub key: String,
    pub value: String,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable, Debug, PartialEq)]
#[table_name="photo_attrs"]
pub struct NewPhotoAttr {
    photo_id: i32,
    key: String,
    value: String
}

static ERR_TAG_LEN_30: &'static str = "TAG_TOO_LONG_MAX_30";
static ERR_TAG_EMPTY: &'static str = "TAG_EMPTY";
static ERR_VALUE_LEN_100: &'static str = "VALUE_TOO_LONG_MAX_100";
static ERR_VALUE_EMPTY: &'static str = "VALUE_EMPTY";

impl NewPhotoAttr {
    // This function verifies the database constraints on the attributes table for you
    // NB: keys are downcased
    pub fn new(photo: &Photo, key: &str, value: &str) -> Result<NewPhotoAttr, &'static str> {
        if key.is_empty() {
            return Err(ERR_TAG_EMPTY);
        }
        if key.len() > 30 {
            return Err(ERR_TAG_LEN_30);
        }
        if value.is_empty() {
            return Err(ERR_VALUE_EMPTY);
        }
        if value.len() > 100 {
            return Err(ERR_VALUE_LEN_100);
        }
        Ok(NewPhotoAttr {
            photo_id: photo.id, key: key.to_lowercase().into(), value: value.into()
        })
    }

    pub fn insert(self, db: &PgConnection) -> Result<PhotoAttr, Error> {
        use db::schema::photo_attrs::dsl::*;
        let created = diesel::insert_into(photo_attrs).values(&self).get_result(db)?;
        Ok(created)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn photo() -> Photo {
        Photo {
            id: 1,
            uuid: "fake".into(),
            owner: 2,
            present: Some(false),
            created_at: NaiveDateTime::from_timestamp(0, 42_000_000),
            updated_at: NaiveDateTime::from_timestamp(0, 42_000_000),
        }
    }

    #[test]
    fn new_photo_attr() {
        let photo = photo();
        match NewPhotoAttr::new(&photo, "FOO", "BAR") {
            Ok(attr) => {
                assert_eq!(attr.key, "foo");
                assert_eq!(attr.value, "BAR");
                assert_eq!(attr.photo_id, photo.id);
            },
            Err(e) => assert!(false, format!("Got error making attr: {}", e))
        }
    }

    #[test]
    fn attr_db_constraints() {
        let photo = photo();
        assert_eq!(NewPhotoAttr::new(&photo, "", "foo"), Err(ERR_TAG_EMPTY));
        assert_eq!(NewPhotoAttr::new(&photo, "asdflkjghasdfljasflkjaslfdjaslfdjalsdfj", "foo"),
                   Err(ERR_TAG_LEN_30));
        assert_eq!(NewPhotoAttr::new(&photo, "f", ""), Err(ERR_VALUE_EMPTY));
        assert_eq!(NewPhotoAttr::new(&photo, "f", "fooagwiuerpqoiweu¨zoiueraux,n,mnwqueihaohsdkjaklsdfjaklsjfklasjfklasjdflkajsfkljaslfdjasldfjalsjdflajsdfl;ajsdfl;ajdsf;lajsdfkla"),
                   Err(ERR_VALUE_LEN_100));
    }
}
