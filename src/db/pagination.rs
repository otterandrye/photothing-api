use diesel::prelude::*;
use diesel::query_dsl::methods::LoadQuery;
use diesel::query_builder::*;
use diesel::pg::Pg;
use diesel::sql_types::{BigInt, Integer};

use rocket::request::{FromForm, FormItems};

// An implementation of keyset paging against postgres. For more info, see
// https://www.citusdata.com/blog/2016/03/30/five-ways-to-paginate/
//
// NB: this _will not work_ on tables that don't have an id column. Still working on
// how to make this restriction a compile-time failure...
//
// Code adapted from http://diesel.rs/guides/extending-diesel/ and
// https://github.com/diesel-rs/diesel/blob/v1.3.0/examples/postgres/advanced-blog-cli/src/
//
// Types to keep track of in here (all named similarly...)
// - Pagination: user-supplied & validated set of pagination params, uses FromForm
// - Page<T>: a page of results, including the params that generated it
// - Paginate/Paginated<T>: diesel glue, shouldn't need to call directly except to bring into scope

const DEFAULT_PER_PAGE: i64 = 30;

// Struct for collecting a user's pagination params
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub key: Option<i32>,
    pub per_page: i64,
}

impl Pagination {
    pub fn new(key: Option<i32>, per_page: Option<i64>) -> Pagination {
        let per_page = match per_page {
            Some(per_page) if per_page >= 1 => per_page,
            _ => DEFAULT_PER_PAGE,
        };
        Pagination { key, per_page }
    }

    pub fn first() -> Pagination {
        Pagination::new(None, None)
    }

    pub fn page(key: i32) -> Pagination {
        Pagination::new(Some(key), None)
    }

    pub fn limit(&self) -> i64 {
        self.per_page
    }
}

impl<'f> FromForm<'f> for Pagination {
    type Error = ();

    fn from_form(items: &mut FormItems<'f>, _strict: bool) -> Result<Pagination, ()> {
        let mut max_key = None;
        let mut per_page = None;

        for item in items {
            // TODO: right now this will ignore bad user-supplied parameters
            // would a bad request be more appropriate?
            match item.key.as_str() {
                "key" => max_key = item.value.parse::<i32>().ok(),
                "page_size" => per_page = item.value.parse::<i64>().ok(),
                _ => { /* always allow extra params */ },
            }
        }
        Ok(Pagination::new(max_key, per_page))
    }
}

// Struct for returning a page of results from the DB
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[cfg_attr(test, derive(Clone))]
pub struct Page<T> {
    pub key: Option<i32>,
    pub next_key: Option<i32>,
    pub remaining: i64,
    pub items: Vec<T>,
}

impl<T> Page<T> {
    pub fn empty() -> Self {
        Page { key: None, next_key: None, remaining: 0, items: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn map<F, U>(self, mapper: F) -> Page<U>
        where F: Fn(T) -> U
    {
        self.map_items(|items| items.into_iter().map(mapper).collect())
    }

    pub fn map_items<F, U>(self, item_mapper: F) -> Page<U>
        where F: FnOnce(Vec<T>) -> Vec<U>
    {
        Page {
            items: item_mapper(self.items),
            key: self.key,
            remaining: self.remaining,
            next_key: self.next_key
        }
    }
}

pub trait Paginate: Sized {
    fn paginate(self, page: Pagination) -> Paginated<Self>;
}

impl<T> Paginate for T {
    fn paginate(self, page: Pagination) -> Paginated<Self> {
        Paginated {
            query: self,
            page,
        }
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct Paginated<T> {
    query: T,
    page: Pagination,
}

impl<T> Paginated<T> {
    pub fn load_and_count_pages<U>(self, conn: &PgConnection) -> QueryResult<Page<U>>
    where
        Self: LoadQuery<PgConnection, (U, i64, i32)>,
    {
        let key = self.page.key;
        let results = self.load::<(U, i64, i32)>(conn)?;
        let (total, next_key) = results.get(0).map(|x| (x.1, Some(x.2))).unwrap_or((0, None));
        let items = results.into_iter().map(|x| x.0).collect::<Vec<_>>();
        let remaining = total - items.len() as i64;
        let page = Page { items, key, next_key, remaining };
        Ok(page)
    }
}

impl<T: Query> Query for Paginated<T> {
    type SqlType = (T::SqlType, BigInt, Integer);
}

impl<T> RunQueryDsl<PgConnection> for Paginated<T> {}

impl<T> QueryFragment<Pg> for Paginated<T>
where
    T: QueryFragment<Pg>,
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        // TODO: make id column configurable. Identifiable?
        out.push_sql("SELECT *, COUNT(*) OVER (), MAX(id) OVER () FROM (");
        self.query.walk_ast(out.reborrow())?;
        out.push_sql(") t ");
        out.push_sql("WHERE id > ");
        out.push_bind_param::<Integer, _>(&self.page.key.unwrap_or(0))?;
        out.push_sql("ORDER BY id ASC LIMIT ");
        out.push_bind_param::<BigInt, _>(&self.page.limit())?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    #[ignore]
    fn page_map_items() {
        let pg = Page { items: vec![1, 2, 3], key: Some(11), next_key: Some(55), remaining: 1};
        let mapped = pg.clone()
            .map_items(|items| items.into_iter().map(|i: i32| i.to_string()).collect());
        assert_eq!(mapped.items, vec!["1", "2", "3"]);
        assert_eq!(pg.key, mapped.key);
        assert_eq!(pg.next_key, mapped.next_key);
        assert_eq!(pg.remaining, mapped.remaining);
    }

    #[test]
    #[ignore]
    fn pagination_from_request() {
        let mut form = FormItems::from("key=3&page_size=21");
        let parsed = Pagination::from_form(&mut form, false)
            .expect("parsing from items failed");
        assert_eq!(parsed.key, Some(3), "got key");
        assert_eq!(parsed.per_page, 21, "got page size");
    }

    #[test]
    #[ignore]
    fn pagination_defaults() {
        let mut form = FormItems::from("?wrong=foo&args=only");
        let parsed = Pagination::from_form(&mut form, false)
            .expect("parsing from items failed");
        assert_eq!(parsed.key, None, "got default key");
        assert_eq!(parsed.per_page, DEFAULT_PER_PAGE, "got default page size");
    }

    #[test]
    #[ignore]
    fn pagination_wrong_types() {
        let mut form = FormItems::from("key=beep&page_size=bop");
        let parsed = Pagination::from_form(&mut form, false)
            .expect("parsing from items failed");
        assert_eq!(parsed.key, None, "got key");
        assert_eq!(parsed.per_page, DEFAULT_PER_PAGE, "got page size");
    }
}
