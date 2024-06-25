# Library/crate choices
(see https://blessed.rs/crates#section-networking-subsection-http-foundations)

# Inner design (WIP)
- (OPTIONAL: collector: runs on single node, generates a timeline of data)
- backend: aggregates collectors, serves HTML frontend
- (OPTIONAL: frontend: serves as more complex logic, when HTML + CSS + nice pictures don't suffice anymore)