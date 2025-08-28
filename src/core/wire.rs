use serde::Deserialize;

#[derive(Deserialize, Clone, Copy)]
pub(crate) struct RawNumI64 {
    pub(crate) raw: Option<i64>,
}
