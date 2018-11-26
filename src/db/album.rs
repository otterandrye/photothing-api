use chrono::prelude::*;
use diesel;
use diesel::PgConnection;
use diesel::result::Error;
use diesel::prelude::*;
use diesel_derives::{belongs_to, table_name, Queryable, Identifiable, Associations};

use crate::db::{
    pagination::{Paginate, Pagination, Page},
    user::User,
    photo::{Photo, PhotoAttr},
    schema::{photo_albums, album_membership}
};

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
type AlbumPhotoWithAttrs = (Photo, AlbumMembership, Vec<PhotoAttr>);

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
            .on_conflict_do_nothing()
            .execute(db)?;
        Ok(inserted)
    }

    pub fn remove_photos(&self, db: &PgConnection, photos: &Vec<i32>) -> Result<usize, Error> {
        use db::schema::album_membership::dsl::*;
        for p_id in photos.iter() {
            diesel::delete(
                album_membership
                    .filter(album_id.eq(self.id))
                    .filter(photo_id.eq(p_id))
            ).execute(db)?;
        }
        Ok(photos.len())
    }

    pub fn get_photos(&self, db: &PgConnection, page: Pagination) -> Result<Page<AlbumPhotoWithAttrs>, Error> {
        use db::schema::photos::dsl::photos;
        // TODO: figure out how to order on the album membership 'ordering' column rather than id
        let album_photos = AlbumMembership::belonging_to(self)
            .inner_join(photos)
            .paginate(page)
            .load_and_count_pages::<AlbumPhoto>(db)?;

        let (membership, db_photos): (Vec<AlbumMembership>, Vec<Photo>) = album_photos.items.into_iter().unzip();

        // I believe this is running something like a 'SELECT WHERE id IN (...)' to get the attrs
        let attributes: Vec<Vec<PhotoAttr>> = PhotoAttr::belonging_to(&db_photos)
            .load(db)?
            .grouped_by(&db_photos);
        let zipped_photos = izip!(db_photos, membership, attributes).collect();

        // TODO: it feels stupid to have to rebuild the Page because we partially moved photos
        // from it, but the `belonging_to` API seems to need Vec<Photo> not Vec<&Photo> :(
        Ok(Page {
            key: album_photos.key,
            next_key: album_photos.next_key,
            remaining: album_photos.remaining,
            items: zipped_photos })
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
