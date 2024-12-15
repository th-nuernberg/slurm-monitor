
use chrono::{DateTime, Utc};
use derive_more::derive::{Add, Deref, Display, FromStr, Into, Sub};
use poem_openapi::{
    NewType, Object,
};

/*
NOTE HashMaps don't seem to work with poem_openapi ._________.
I tried deriving Object (nope b/c newtype struct), NewType (weird messages about missing FromJSON, FromMultipart etc.).
I even tried switching off various trait derives with
`#[oai(from_json = false, from_parameter = false, from_multipart = false, to_header = false)]` but that made no difference.

Other data structures seem to work (except tuples), and I can give a HashMap directly as a response via
`Json(HashMap<Foo, Bar>)` but _not_ derive any [`poem_openapi::Type`] from it. Maybe it's a bug, maybe not. The only
reason I really tried that anyways was hoping that Swagger would display my response types nicely. Moving on…
*/

#[derive(Debug, Clone, PartialEq, Eq, Hash, NewType, Deref, Into, FromStr, Display)]
pub struct SlurmUser(pub String);

#[derive(Debug, Clone, Copy, Add, Sub, PartialEq, Eq, Hash, NewType, Deref, Into, FromStr, Display)]
pub struct GpuHours(pub usize);

#[derive(Debug, Clone, Object)]
pub struct GpuHoursPerUser {
    user: SlurmUser,
    hours: GpuHours,
}

//#[derive(Debug, Clone, PartialEq, Eq, NewType)]
//pub struct GpuHoursPerUser2(#[oai(from_json = false, from_parameter = false, from_multipart = false, to_header = false)] pub HashMap<String, usize>);

/// TODO find use
#[derive(Debug, Clone, PartialEq, Eq, Object)]
pub struct GpuReservedTimeframe {
    num_cores: usize,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}
