use crate::buffer::Buffer;
use std::iter::FromIterator;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicateDocumentTransitionsWithIdsError)]
pub struct DuplicateDocumentTransitionsWithIdsErrorWasm {
    references: Vec<(String, Vec<u8>)>,
    code: u32,
}

impl DuplicateDocumentTransitionsWithIdsErrorWasm {
    pub fn new(references: Vec<(String, Vec<u8>)>, code: u32) -> Self {
        DuplicateDocumentTransitionsWithIdsErrorWasm { references, code }
    }
}

#[wasm_bindgen(js_class=DuplicateDocumentTransitionsWithIdsError)]
impl DuplicateDocumentTransitionsWithIdsErrorWasm {
    #[wasm_bindgen(js_name=getReferences)]
    pub fn get_references(&self) -> js_sys::Array {
        self.references
            .iter()
            .map(|v| {
                js_sys::Array::from_iter(vec![
                    JsValue::from(v.0.clone()),
                    JsValue::from(Buffer::from_bytes(&v.1)),
                ])
            })
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
