use diesel::{ExpressionMethods, QueryDsl, QueryResult, Queryable, Selectable};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::schema;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::test_cases)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TestCase {
    pub id: Uuid,
    pub input_path: String,
    pub output_path: String,
    pub is_hidden: bool,
}

impl TestCase {
    pub async fn get_by_question_id(
        question_id: Uuid,
        connection: &mut AsyncPgConnection,
    ) -> QueryResult<Vec<Self>> {
        use schema::test_cases::dsl as t;

        t::test_cases
            .select((t::id, t::input_path, t::output_path, t::is_hidden))
            .filter(t::question_id.eq(question_id))
            .load(connection)
            .await
    }
}
