use chrono::prelude::*;
use diesel::{self, PgConnection, result::Error};
use diesel::prelude::*;
use diesel_derives::{belongs_to, Queryable, Identifiable, Associations};

use crate::db::pagination::{Paginate, Pagination, Page};
use crate::db::schema::{photos, photo_attrs};
use crate::db::user::User;
use crate::util::uuid;

// Main Photo object, keeps track of whether the file exists on S3 and who uploaded it
#[derive(Queryable, Associations, Identifiable)]
#[cfg_attr(test, derive(Debug))]
#[belongs_to(User, foreign_key = "owner")]
pub struct Photo {
    pub id: i32,
    pub uuid: String,
    pub owner: i32, // users.id
    pub present: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

type PhotoWithAttrs = (Photo, Vec<PhotoAttr>);

impl Photo {
    pub fn by_user(db: &PgConnection, user: &User, page: Pagination) -> Result<Page<PhotoWithAttrs>, Error> {
        let photos = Photo::belonging_to(user)
             .paginate(page)
             .load_and_count_pages::<Photo>(db)?;
        let attributes = PhotoAttr::belonging_to(&photos.items)
            .load(db)?
            .grouped_by(&photos.items);
        Ok(photos.map_items(|items| items.into_iter()
            .zip(attributes)
            .collect()))
    }
}

#[derive(Insertable)]
#[table_name="photos"]
pub struct NewPhoto {
    uuid: String,
    owner: i32,
}

impl NewPhoto {
    pub fn new(owner: &User) -> NewPhoto {
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
#[derive(Queryable, Associations, Identifiable)]
#[belongs_to(Photo)]
#[primary_key(photo_id, key)]
pub struct PhotoAttr {
    photo_id: i32,
    pub key: String,
    pub value: String,
    pub updated_at: NaiveDateTime,
}

// Validation for an attribute's string key & value
#[derive(Debug, PartialEq)]
pub struct AttributeKeyValue {
    key: String,
    value: String,
}

static ERR_TAG_LEN_30: &str = "TAG_TOO_LONG_MAX_30";
static ERR_TAG_EMPTY: &str = "TAG_EMPTY";
static ERR_VALUE_LEN_100: &str = "VALUE_TOO_LONG_MAX_100";
static ERR_VALUE_EMPTY: &str = "VALUE_EMPTY";

impl AttributeKeyValue {
    // This function verifies the database constraints on the attributes table for you
    // NB: keys are downcased
    pub fn new(key: &str, value: &str) -> Result<AttributeKeyValue, &'static str> {
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
        Ok(AttributeKeyValue {
            key: key.to_lowercase().into(),
            value: value.into()
        })
    }
}

#[derive(Insertable, Debug)]
#[table_name="photo_attrs"]
pub struct NewPhotoAttr {
    photo_id: i32,
    key: String,
    value: String
}

impl NewPhotoAttr {
    pub fn new(photo: &Photo, value: AttributeKeyValue) -> NewPhotoAttr {
        NewPhotoAttr {
            photo_id: photo.id, key: value.key, value: value.value
        }
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn new_photo_attr() {
        let photo = photo();
        match AttributeKeyValue::new("FOO", "BAR") {
            Ok(attr) => {
                assert_eq!(attr.key, "foo");
                assert_eq!(attr.value, "BAR");
                let new = NewPhotoAttr::new(&photo, attr);
                assert_eq!(new.photo_id, photo.id);
            },
            Err(e) => assert!(false, format!("Got error making attr: {}", e))
        }
    }

    #[test]
    fn attr_db_constraints() {
        assert_eq!(AttributeKeyValue::new("", "foo"), Err(ERR_TAG_EMPTY));
        assert_eq!(AttributeKeyValue::new("asdflkjghasdfljasflkjaslfdjaslfdjalsdfj", "foo"),
                   Err(ERR_TAG_LEN_30));
        assert_eq!(AttributeKeyValue::new("f", ""), Err(ERR_VALUE_EMPTY));
        assert_eq!(AttributeKeyValue::new("f", "fooagwiuerpqoiweu¨zoiueraux,n,mnwqueihaohsdkjaklsdfjaklsjfklasjfklasjdflkajsfkljaslfdjasldfjalsjdflajsdfl;ajsdfl;ajdsf;lajsdfkla"),
                   Err(ERR_VALUE_LEN_100));
    }
}
