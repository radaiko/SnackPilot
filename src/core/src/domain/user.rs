#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct GourmetUserInfo {
    pub username: String,
    pub shop_model_id: String,
    pub eater_id: String,
    pub staff_group_id: String,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}
