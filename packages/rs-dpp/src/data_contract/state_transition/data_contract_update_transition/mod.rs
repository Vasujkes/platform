use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{
    data_contract::DataContract,
    identity::KeyID,
    prelude::Identifier,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    util::json_value::{JsonValueExt, ReplaceWith},
    ProtocolError,
};

use super::property_names::*;

pub mod apply_data_contract_update_transition_factory;
pub mod validation;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataContractUpdateTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    // we want to skip serialization of transitions, as we does it manually in `to_object()`  and `to_json()`
    #[serde(skip_serializing)]
    pub data_contract: DataContract,
    pub signature_public_key_id: KeyID,
    pub signature: Vec<u8>,
    #[serde(skip)]
    pub execution_context: StateTransitionExecutionContext,
}

impl std::default::Default for DataContractUpdateTransition {
    fn default() -> Self {
        DataContractUpdateTransition {
            protocol_version: Default::default(),
            transition_type: StateTransitionType::DataContractUpdate,
            signature_public_key_id: 0,
            signature: vec![],
            data_contract: Default::default(),
            execution_context: Default::default(),
        }
    }
}

impl DataContractUpdateTransition {
    pub fn from_raw_object(
        mut raw_data_contract_update_transition: JsonValue,
    ) -> Result<DataContractUpdateTransition, ProtocolError> {
        Ok(DataContractUpdateTransition {
            protocol_version: raw_data_contract_update_transition.get_u64(PROTOCOL_VERSION)? as u32,
            signature: raw_data_contract_update_transition
                .remove_into(SIGNATURE)
                .unwrap_or_default(),
            signature_public_key_id: raw_data_contract_update_transition
                .get_u64(SIGNATURE_PUBLIC_KEY_ID)
                .unwrap_or_default(),
            data_contract: DataContract::from_raw_object(
                raw_data_contract_update_transition.remove(DATA_CONTRACT)?,
            )?,
            ..Default::default()
        })
    }

    pub fn get_data_contract(&self) -> &DataContract {
        &self.data_contract
    }

    pub fn set_data_contract(&mut self, data_contract: DataContract) {
        self.data_contract = data_contract;
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<&Identifier> {
        vec![&self.data_contract.id]
    }
}

impl StateTransitionIdentitySigned for DataContractUpdateTransition {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.data_contract.owner_id
    }

    fn get_signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }
}

impl StateTransitionLike for DataContractUpdateTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8> {
        &self.signature
    }
    /// set a new signature
    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature
    }

    fn get_execution_context(&self) -> &StateTransitionExecutionContext {
        &self.execution_context
    }

    fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext {
        &mut self.execution_context
    }

    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
        self.execution_context = execution_context
    }
}

impl StateTransitionConvert for DataContractUpdateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![SIGNATURE, ENTROPY]
    }

    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }

        json_value.replace_binary_paths(Self::binary_property_paths(), ReplaceWith::Base64)?;
        json_value
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Base58)?;

        json_value.insert(DATA_CONTRACT.to_string(), self.data_contract.to_json()?)?;

        Ok(json_value)
    }

    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_object: JsonValue = serde_json::to_value(self)?;
        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_object {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }
        json_object.insert(
            String::from(DATA_CONTRACT),
            self.data_contract.to_object(false)?,
        )?;
        Ok(json_object)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::tests::fixtures::get_data_contract_fixture;
    use crate::{util::deserializer::get_protocol_version, version};

    use super::*;

    struct TestData {
        state_transition: DataContractUpdateTransition,
        data_contract: DataContract,
    }

    fn get_test_data() -> TestData {
        let data_contract = get_data_contract_fixture(None);

        let state_transition = DataContractUpdateTransition::from_raw_object(json!({
                    PROTOCOL_VERSION: version::LATEST_VERSION,
                    DATA_CONTRACT : data_contract.to_object(false).unwrap(),
        }))
        .expect("state transition should be created without errors");

        TestData {
            data_contract,
            state_transition,
        }
    }

    #[test]
    fn should_return_protocol_version() {
        let data = get_test_data();
        assert_eq!(
            version::LATEST_VERSION,
            data.state_transition.get_protocol_version()
        )
    }

    #[test]
    fn should_return_transition_type() {
        let data = get_test_data();
        assert_eq!(
            StateTransitionType::DataContractUpdate,
            data.state_transition.get_type()
        );
    }

    #[test]
    fn should_return_data_contract() {
        let data = get_test_data();

        assert_eq!(
            data.state_transition
                .get_data_contract()
                .to_object(false)
                .expect("conversion to object shouldn't fail"),
            data.data_contract
                .to_object(false)
                .expect("conversion to object shouldn't fail")
        );
    }

    #[test]
    fn should_return_state_transition_in_json_format() {
        let data = get_test_data();
        let mut json_object = data
            .state_transition
            .to_json(false)
            .expect("conversion to JSON shouldn't fail");

        assert_eq!(
            version::LATEST_VERSION,
            json_object
                .get_u64(PROTOCOL_VERSION)
                .expect("the protocol version should be present") as u32
        );

        assert_eq!(
            4,
            json_object
                .get_u64(TRANSITION_TYPE)
                .expect("the transition type should be present") as u8
        );
        assert_eq!(
            0,
            json_object
                .get_u64(SIGNATURE_PUBLIC_KEY_ID)
                .expect("default public key id should be defined"),
        );
        assert_eq!(
            "",
            json_object
                .remove_into::<String>(SIGNATURE)
                .expect("default string value for signature should be present")
        );
    }

    #[test]
    fn should_return_serialized_state_transition_to_buffer() {
        let data = get_test_data();
        let state_transition_bytes = data
            .state_transition
            .to_buffer(false)
            .expect("state transition should be converted to buffer");
        let (protocol_bytes, _) = state_transition_bytes.split_at(4);
        assert_eq!(
            version::LATEST_VERSION,
            get_protocol_version(protocol_bytes).expect("version should be valid")
        )
    }

    #[test]
    fn should_return_owner_id() {
        let data = get_test_data();
        assert_eq!(
            &data.data_contract.owner_id,
            data.state_transition.get_owner_id()
        );
    }

    #[test]
    fn is_data_contract_state_transition() {
        let data = get_test_data();
        assert!(data.state_transition.is_data_contract_state_transition());
        assert!(!data.state_transition.is_document_state_transition());
        assert!(!data.state_transition.is_identity_state_transition());
    }
}
