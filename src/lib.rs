pub mod signature_generator;
mod delta_generator;

type StrongHash = String;

// TODO: addler32 from  https://docs.rs/adler32/1.2.0/adler32/struct.RollingAdler32.html
// + a std::collections::VecDeque to keep track of the bytes
