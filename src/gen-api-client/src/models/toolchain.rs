/*
 * JJS main API
 *
 * No description provided (generated by Openapi Generator https://github.com/openapitools/openapi-generator)
 *
 * The version of the OpenAPI document: 1.0.0
 *
 * Generated by: https://openapi-generator.tech
 */

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Toolchain {
    /// Internal name, e.g. \"cpp.san.9.1\"
    #[serde(rename = "id")]
    pub id: String,
    /// Human readable name, e.g. \"GCC C++ v9.1 with sanitizers enables\"
    #[serde(rename = "name")]
    pub name: String,
}

impl Toolchain {
    pub fn new(id: String, name: String) -> Toolchain {
        Toolchain { id, name }
    }
}