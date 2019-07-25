pub(crate) struct Context {
    pub(crate) pool: r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::pg::PgConnection>>,
    pub(crate) cfg: cfg::Config,
}


impl juniper::Context for Context {}