use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{
    identity::{core_script::CoreScript, KeyID},
    prelude::Identifier,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    util::{
        json_value::{JsonValueExt, ReplaceWith},
        string_encoding::{self, Encoding},
    },
    ProtocolError,
};

use super::properties::{
    PROPERTY_IDENTITY_ID, PROPERTY_OUTPUT_SCRIPT, PROPERTY_SIGNATURE,
    PROPERTY_SIGNATURE_PUBLIC_KEY_ID,
};

pub mod apply_identity_credit_withdrawal_transition_factory;
pub mod validation;

#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy, Debug)]
pub enum Pooling {
    Never = 0,
    IfAvailable = 1,
    Standard = 2,
}

impl std::default::Default for Pooling {
    fn default() -> Self {
        Pooling::Never
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditWithdrawalTransition {
    pub protocol_version: u32,
    #[serde(rename = "type")]
    pub transition_type: StateTransitionType,
    pub identity_id: Identifier,
    pub amount: u64,
    pub core_fee: u32,
    pub pooling: Pooling,
    pub output_script: CoreScript,
    pub signature_public_key_id: KeyID,
    pub signature: Vec<u8>,
    #[serde(skip)]
    pub execution_context: StateTransitionExecutionContext,
}

impl std::default::Default for IdentityCreditWithdrawalTransition {
    fn default() -> Self {
        IdentityCreditWithdrawalTransition {
            protocol_version: Default::default(),
            transition_type: StateTransitionType::IdentityCreditWithdrawal,
            identity_id: Default::default(),
            amount: Default::default(),
            core_fee: Default::default(),
            pooling: Default::default(),
            output_script: Default::default(),
            signature_public_key_id: Default::default(),
            signature: Default::default(),
            execution_context: Default::default(),
        }
    }
}

impl IdentityCreditWithdrawalTransition {
    pub fn from_value(value: JsonValue) -> Result<Self, ProtocolError> {
        let transition: IdentityCreditWithdrawalTransition = serde_json::from_value(value)?;

        Ok(transition)
    }

    pub fn from_json(mut value: JsonValue) -> Result<Self, ProtocolError> {
        value.replace_binary_paths(Self::binary_property_paths(), ReplaceWith::Bytes)?;

        Self::from_value(value)
    }

    pub fn from_raw_object(
        mut raw_object: JsonValue,
    ) -> Result<IdentityCreditWithdrawalTransition, ProtocolError> {
        let output_script_option = raw_object.get(PROPERTY_OUTPUT_SCRIPT);

        let output_script_string = output_script_option
            .ok_or_else(|| anyhow!("uanble to get outputScript"))
            .and_then(|value| serde_json::from_value(value.clone()).map_err(|e| anyhow!(e)))
            .map(|bytes: Vec<u8>| string_encoding::encode(&bytes, Encoding::Base64))?;

        raw_object.insert(
            PROPERTY_OUTPUT_SCRIPT.to_owned(),
            JsonValue::String(output_script_string),
        )?;

        raw_object
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Base58)?;

        Self::from_value(raw_object)
    }

    /// Returns ID of the created contract
    pub fn get_modified_data_ids(&self) -> Vec<&Identifier> {
        vec![&self.identity_id]
    }
}

impl StateTransitionIdentitySigned for IdentityCreditWithdrawalTransition {
    /// Get owner ID
    fn get_owner_id(&self) -> &Identifier {
        &self.identity_id
    }

    fn get_signature_public_key_id(&self) -> KeyID {
        self.signature_public_key_id
    }

    fn set_signature_public_key_id(&mut self, key_id: crate::identity::KeyID) {
        self.signature_public_key_id = key_id
    }
}

impl StateTransitionLike for IdentityCreditWithdrawalTransition {
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

impl StateTransitionConvert for IdentityCreditWithdrawalTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE, PROPERTY_SIGNATURE_PUBLIC_KEY_ID]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_IDENTITY_ID]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![PROPERTY_SIGNATURE]
    }

    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;

        let output_script_option = json_value.get(PROPERTY_OUTPUT_SCRIPT);

        let output_script_bytes = output_script_option
            .ok_or_else(|| anyhow!("uanble to get outputScript"))
            .and_then(|value| serde_json::from_value(value.clone()).map_err(|e| anyhow!(e)))
            .and_then(|string: String| {
                string_encoding::decode(&string, Encoding::Base64).map_err(|e| anyhow!(e))
            })?;

        json_value.insert(
            PROPERTY_OUTPUT_SCRIPT.to_owned(),
            JsonValue::Array(
                output_script_bytes
                    .into_iter()
                    .map(JsonValue::from)
                    .collect(),
            ),
        )?;

        json_value
            .replace_identifier_paths(Self::identifiers_property_paths(), ReplaceWith::Bytes)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in Self::signature_property_paths() {
                    o.remove(path);
                }
            }
        }

        Ok(json_value)
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

        Ok(json_value)
    }
}
