#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::saccts, check_for_backend(diesel::sqlite::Sqlite))]
struct SAcct {
    jobid: String,
    jobidraw: String,
    jobname: String,
    user: String,

    elapsed: String, // TODO parse time
    state: String,   // TODO maybe parse enum?

    partition: String,
    ntasks: u32,
    alloccpus: u32,

    maxrss: String, // TODO parse units
    averss: String, // TODO parse units
    avecpu: String, // TODO parse time

    consumedenergy: f64,
}

