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

pub struct UpdateSubmissionHistory {
    pub id: Uuid,
    pub score: f32,
    pub compilation_error: Option<String>,
}

impl UpdateSubmissionHistory {
    pub async fn update(self, connection: &mut AsyncPgConnection) -> QueryResult<()> {
        use schema::submission_histories::dsl as s;

        let affected_row_count = diesel::update(s::submission_histories.filter(s::id.eq(self.id)))
            .set((
                s::score.eq(self.score),
                s::compilation_error.eq(self.compilation_error),
            ))
            .execute(connection)
            .await?;
        if affected_row_count != 1 {
            Err(diesel::result::Error::NotFound)
        } else {
            Ok(())
        }
    }
}
