use clorinde::{Error, config::Config};

// This script will generate a new clorinde crate every time your schema or queries change.
// In this example, we generate the module in our project, but we could also generate it elsewhere.

#[allow(clippy::result_large_err)]
fn main() -> Result<(), Error> {
    let queries_path = "../queries";
    let schema_file = "../../schema.sql";

    let cfg = Config::builder()
        .queries(queries_path)
        .name("database")
        .destination("../crates/database")
        .build();

    println!("cargo:rerun-if-changed={queries_path}");
    println!("cargo:rerun-if-changed={schema_file}");
    clorinde::gen_managed(&[schema_file], cfg)?;

    Ok(())
}
