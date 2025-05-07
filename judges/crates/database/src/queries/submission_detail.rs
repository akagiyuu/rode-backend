// This file was generated with `clorinde`. Do not modify.

#[derive(Debug)]
pub struct InsertParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub submission_id: uuid::Uuid,
    pub index: i32,
    pub status: i32,
    pub run_time: i32,
    pub stdout: T1,
    pub stderr: T2,
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub fn insert() -> InsertStmt {
    InsertStmt(crate::client::async_::Stmt::new(
        "INSERT INTO submission_details( submission_id, index, status, run_time, stdout, stderr ) VALUES ( $1, $2, $3, $4, $5, $6 )",
    ))
}
pub struct InsertStmt(crate::client::async_::Stmt);
impl InsertStmt {
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s mut self,
        client: &'c C,
        submission_id: &'a uuid::Uuid,
        index: &'a i32,
        status: &'a i32,
        run_time: &'a i32,
        stdout: &'a T1,
        stderr: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        let stmt = self.0.prepare(client).await?;
        client
            .execute(
                stmt,
                &[submission_id, index, status, run_time, stdout, stderr],
            )
            .await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        InsertParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for InsertStmt
{
    fn params(
        &'a mut self,
        client: &'a C,
        params: &'a InsertParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.submission_id,
            &params.index,
            &params.status,
            &params.run_time,
            &params.stdout,
            &params.stderr,
        ))
    }
}
