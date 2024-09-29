# Library/crate choices
(see https://blessed.rs/crates#section-networking-subsection-http-foundations)

## Serialization / Cache
### Alternatives
- Sled (maybe abandoned, not stable enough)
- serde +
    - bincode: seems very popular, but breaks with reordering. Should be a minor nuisance for our use case if we only use it for cache, but a possible way to f\*\*k things up
    - bson (https://lib.rs/crates/bson): seems doable
    - protobuf (https://lib.rs/crates/protobuf): field tested and widely supported. Unfortunately you have to maintain extra type definitions
        - maybe that's not quite the case? see https://docs.rs/protobuf-codegen/latest/protobuf_codegen/ or https://docs.rs/protobuf-json-mapping/latest/protobuf_json_mapping/
    - capnproto (https://lib.rs/crates/protobuf): fast, but seems a bit complicated
- Databases (SQL, NoSQL): seems doable (and a standard choice), but maintaining and syncing the data structures + migrations, even with ORM, seems like too much complexity without good reasons
    - I suggest optimizing + profiling before making the switch

# Inner design (WIP)
- (OPTIONAL: collector: runs on single node, generates a timeline of data)
- backend: aggregates collectors, serves HTML frontend
- (OPTIONAL: frontend: serves as more complex logic, when HTML + CSS + nice pictures don't suffice anymore)

## Backend
### Async

With the current poll interval of 30s, and the number of collectors being 3-5, async would have not been necessary to ensure fluid operations. But since I am learning async for the frontend webserver anyways, and this will scale more nicely when there are problems with connections or interior tasks on the backend, I am going to do it async anyways.

#### `ClientMap` as `Mutex<HashMap<SocketAddr, Arc<Mutex<Client>>>>`
This really doesn't look performant at all, but the access will be very infrequent, as mentioned above.
- A possible fix would be to only have the HashMap in `main()` and use channels to receive events and update/respond with data accordingly. But unneccessary optimization at this point.