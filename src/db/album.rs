use chrono::prelude::*;
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;

use db::pagination::{Paginate, Pagination, Page};
use db::user::User;
use db::photo::Photo;
use db::schema::{photo_albums, album_membership};

#[derive(Queryable, Associations, Identifiable)]
#[belongs_to(User)]
#[table_name="photo_albums"]
pub struct Album {
    pub id: i32,
    pub user_id: i32,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name="photo_albums"]
struct NewAlbum<'a> {
    user_id: i32,
    name: Option<&'a str>,
}

type AlbumPhoto = (AlbumMembership, Photo);

impl Album {
    pub fn create(db: &PgConnection, user: &User, album_name: Option<&str>) -> Result<Album, Error> {
        use db::schema::photo_albums::dsl::*;
        let new = NewAlbum { user_id: user.id, name: album_name };
        let created = diesel::insert_into(photo_albums).values(&new).get_result(db)?;
        Ok(created)
    }

    pub fn for_user(db: &PgConnection, user: &User, page: Pagination) -> Result<Page<Album>, Error> {
        let albums = Album::belonging_to(user)
            .paginate(page)
            .load_and_count_pages::<Album>(db)?;
        Ok(albums)
    }

    pub fn by_id(db: &PgConnection, user: &User, album_id: i32) -> Result<Option<Album>, Error> {
        use db::schema::photo_albums::dsl::*;
        photo_albums
            .filter(id.eq(album_id))
            .filter(user_id.eq(user.id))
            .first(db).optional()
    }

    pub fn add_photos(&self, db: &PgConnection, photos: &Vec<i32>) -> Result<usize, Error> {
        use db::schema::album_membership::dsl::*;
        // TODO: additional work here to make sure we don't insert photos which are 'renderings',
        // once that concept & the photo graph both exist
        let id_pairs: Vec<NewAlbumMember> = photos.iter()
            .map(|id| NewAlbumMember { photo_id: *id, album_id: self.id })
            .collect();
        let inserted = diesel::insert_into(album_membership)
            .values(&id_pairs)
            .execute(db)?;
        Ok(inserted)
    }

    pub fn get_photos(&self, db: &PgConnection, page: Pagination) -> Result<Page<AlbumPhoto>, Error> {
        use db::schema::photos::dsl::photos;
        // TODO: figure out how to get photo attributes here too
        // TODO: figure out how to order on the album membership 'ordering' column rather than id
        let album_photos = AlbumMembership::belonging_to(self)
            .inner_join(photos)
            .paginate(page)
            .load_and_count_pages::<AlbumPhoto>(db)?;
        Ok(album_photos)
    }
}

#[derive(Queryable, Associations, Identifiable)]
#[belongs_to(Album)]
#[belongs_to(Photo)]
#[primary_key(photo_id, album_id)]
#[table_name="album_membership"]
pub struct AlbumMembership {
    photo_id: i32,
    album_id: i32,
    pub ordering: Option<i16>,
    pub caption: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name="album_membership"]
struct NewAlbumMember {
    photo_id: i32,
    album_id: i32,
}
