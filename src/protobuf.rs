pub mod apex {
    pub mod proto {
        include!(concat!(env!("OUT_DIR"), "/apex.proto.rs"));
    }
}

pub mod build {
    pub mod tools {
        pub mod releasetools {
            include!(concat!(env!("OUT_DIR"), "/build.tools.releasetools.rs"));
        }
    }
}
