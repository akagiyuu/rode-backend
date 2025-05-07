// This file was generated with `clorinde`. Do not modify.

#[derive(Debug, Clone, PartialEq)]
pub struct GetByQuestionId {
    pub index: i32,
    pub input_path: Option<String>,
    pub output_path: String,
    pub is_hidden: bool,
}
pub struct GetByQuestionIdBorrowed<'a> {
    pub index: i32,
    pub input_path: Option<&'a str>,
    pub output_path: &'a str,
    pub is_hidden: bool,
}
impl<'a> From<GetByQuestionIdBorrowed<'a>> for GetByQuestionId {
    fn from(
        GetByQuestionIdBorrowed {
            index,
            input_path,
            output_path,
            is_hidden,
        }: GetByQuestionIdBorrowed<'a>,
    ) -> Self {
        Self {
            index,
            input_path: input_path.map(|v| v.into()),
            output_path: output_path.into(),
            is_hidden,
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct GetByQuestionIdQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    stmt: &'s mut crate::client::async_::Stmt,
    extractor: fn(&tokio_postgres::Row) -> Result<GetByQuestionIdBorrowed, tokio_postgres::Error>,
    mapper: fn(GetByQuestionIdBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> GetByQuestionIdQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(GetByQuestionIdBorrowed) -> R,
    ) -> GetByQuestionIdQuery<'c, 'a, 's, C, R, N> {
        GetByQuestionIdQuery {
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
pub fn get_by_question_id() -> GetByQuestionIdStmt {
    GetByQuestionIdStmt(crate::client::async_::Stmt::new(
        "SELECT index, input_path, output_path, is_hidden FROM test_cases WHERE question_id = $1 ORDER BY index",
    ))
}
pub struct GetByQuestionIdStmt(crate::client::async_::Stmt);
impl GetByQuestionIdStmt {
    pub fn bind<'c, 'a, 's, C: GenericClient>(
        &'s mut self,
        client: &'c C,
        question_id: &'a uuid::Uuid,
    ) -> GetByQuestionIdQuery<'c, 'a, 's, C, GetByQuestionId, 1> {
        GetByQuestionIdQuery {
            client,
            params: [question_id],
            stmt: &mut self.0,
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<GetByQuestionIdBorrowed, tokio_postgres::Error> {
                Ok(GetByQuestionIdBorrowed {
                    index: row.try_get(0)?,
                    input_path: row.try_get(1)?,
                    output_path: row.try_get(2)?,
                    is_hidden: row.try_get(3)?,
                })
            },
            mapper: |it| GetByQuestionId::from(it),
        }
    }
}
