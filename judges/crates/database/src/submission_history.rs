use diesel::{ExpressionMethods, QueryDsl, QueryResult, Queryable, Selectable};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::schema;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::submission_histories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SubmissionHistory {
    pub question_id: Uuid,
    pub language: i32,
    pub code: String,
}

impl SubmissionHistory {
    pub async fn get(id: Uuid, connection: &mut AsyncPgConnection) -> QueryResult<Self> {
        use schema::submission_histories::dsl as s;

        s::submission_histories
            .select((s::question_id, s::language, s::code))
            .filter(s::id.eq(id))
            .first(connection)
            .await
    }
}
