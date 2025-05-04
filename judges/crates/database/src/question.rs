use diesel::{ExpressionMethods, QueryDsl, QueryResult, Queryable, Selectable};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::schema;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::questions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Question {
    pub score: f32,
    pub time_limit: i32,
    pub memory_limit: i32,
}

impl Question {
    pub async fn get(id: Uuid, connection: &mut AsyncPgConnection) -> QueryResult<Self> {
        use schema::questions::dsl as q;

        q::questions
            .select((q::score, q::time_limit, q::memory_limit))
            .filter(q::id.eq(id))
            .first(connection)
            .await
    }
}
