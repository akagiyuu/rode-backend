// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct UpdateSubmissionStatusParams<T1: crate::StringSql> {
    pub score: f32,
    pub error: T1,
    pub failed_test_case: i32,
    pub id: uuid::Uuid,
}
#[derive(Debug, Clone, PartialEq)]
pub struct GetSubmission {
    pub question_id: uuid::Uuid,
    pub language: i16,
    pub code: String,
}
pub struct GetSubmissionBorrowed<'a> {
    pub question_id: uuid::Uuid,
    pub language: i16,
    pub code: &'a str,
}
impl<'a> From<GetSubmissionBorrowed<'a>> for GetSubmission {
    fn from(
        GetSubmissionBorrowed {
            question_id,
            language,
            code,
        }: GetSubmissionBorrowed<'a>,
    ) -> Self {
        Self {
            question_id,
            language,
            code: code.into(),
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct GetSubmissionQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    stmt: &'s mut crate::client::async_::Stmt,
    extractor: fn(&tokio_postgres::Row) -> Result<GetSubmissionBorrowed, tokio_postgres::Error>,
    mapper: fn(GetSubmissionBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> GetSubmissionQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(GetSubmissionBorrowed) -> R,
    ) -> GetSubmissionQuery<'c, 'a, 's, C, R, N> {
        GetSubmissionQuery {
            client: self.client,
            params: self.params,
            stmt: self.stmt,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let stmt = self.stmt.prepare(self.client).await?;
        let row = self.client.query_one(stmt, &self.params).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let stmt = self.stmt.prepare(self.client).await?;
        Ok(self
            .client
            .query_opt(stmt, &self.params)
            .await?
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stmt = self.stmt.prepare(self.client).await?;
        let it = self
            .client
            .query_raw(stmt, crate::slice_iter(&self.params))
            .await?
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(it)
    }
}
pub fn get_submission() -> GetSubmissionStmt {
    GetSubmissionStmt(crate::client::async_::Stmt::new(
        "SELECT question_id, language, code FROM submissions WHERE id = $1",
    ))
}
pub struct GetSubmissionStmt(crate::client::async_::Stmt);
impl GetSubmissionStmt {
    pub fn bind<'c, 'a, 's, C: GenericClient>(
        &'s mut self,
        client: &'c C,
        id: &'a uuid::Uuid,
    ) -> GetSubmissionQuery<'c, 'a, 's, C, GetSubmission, 1> {
        GetSubmissionQuery {
            client,
            params: [id],
            stmt: &mut self.0,
            extractor:
                |row: &tokio_postgres::Row| -> Result<GetSubmissionBorrowed, tokio_postgres::Error> {
                    Ok(GetSubmissionBorrowed {
                        question_id: row.try_get(0)?,
                        language: row.try_get(1)?,
                        code: row.try_get(2)?,
                    })
                },
            mapper: |it| GetSubmission::from(it),
        }
    }
}
pub fn update_submission_status() -> UpdateSubmissionStatusStmt {
    UpdateSubmissionStatusStmt(crate::client::async_::Stmt::new(
        "UPDATE submissions SET score = $1, error = $2, failed_test_case = $3 WHERE id = $4",
    ))
}
pub struct UpdateSubmissionStatusStmt(crate::client::async_::Stmt);
impl UpdateSubmissionStatusStmt {
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s mut self,
        client: &'c C,
        score: &'a f32,
        error: &'a T1,
        failed_test_case: &'a i32,
        id: &'a uuid::Uuid,
    ) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.0.prepare(client).await?;
        client
            .execute(stmt, &[score, error, failed_test_case, id])
            .await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpdateSubmissionStatusParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpdateSubmissionStatusStmt
{
    fn params(
        &'a mut self,
        client: &'a C,
        params: &'a UpdateSubmissionStatusParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.score,
            &params.error,
            &params.failed_test_case,
            &params.id,
        ))
    }
}
