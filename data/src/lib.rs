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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
