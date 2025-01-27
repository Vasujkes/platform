use std::fmt::Debug;

use dashcore::signer;
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::{
    identity::KeyType,
    prelude::ProtocolError,
    util::{
        hash,
        json_value::{JsonValueExt, ReplaceWith},
        serializer,
    },
    BlsModule,
};

use super::{
    fee::calculate_state_transition_fee::calculate_state_transition_fee,
    state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
    StateTransitionType,
};

const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 2] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
];

pub const DATA_CONTRACT_TRANSITION_TYPES: [StateTransitionType; 2] = [
    StateTransitionType::DataContractCreate,
    StateTransitionType::DataContractUpdate,
];

/// The StateTransitionLike represents set of methods that are shared for all types of State Transition.
/// Every type of state transition should also implement Debug, Clone, and support conversion to compounded [`StateTransition`]
pub trait StateTransitionLike:
    StateTransitionConvert + Clone + Debug + Into<StateTransition>
{
    /// returns the protocol version
    fn get_protocol_version(&self) -> u32;
    /// returns the type of State Transition
    fn get_type(&self) -> StateTransitionType;
    /// returns the signature as a byte-array
    fn get_signature(&self) -> &Vec<u8>;
    /// set a new signature
    fn set_signature(&mut self, signature: Vec<u8>);
    /// Calculates the ST fee in credits
    fn calculate_fee(&self) -> i64 {
        calculate_state_transition_fee(self)
    }

    /// Signs data with the private key
    fn sign_by_private_key(
        &mut self,
        private_key: &[u8],
        key_type: KeyType,
        bls: &impl BlsModule,
    ) -> Result<(), ProtocolError> {
        let data = self.to_buffer(true)?;
        match key_type {
            KeyType::BLS12_381 => self.set_signature(bls.sign(&data, private_key)?),

            // https://github.com/dashevo/platform/blob/9c8e6a3b6afbc330a6ab551a689de8ccd63f9120/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L169
            KeyType::ECDSA_SECP256K1 | KeyType::ECDSA_HASH160 => {
                let signature = signer::sign(&data, private_key)?;
                self.set_signature(signature.to_vec());
            }

            // the default behavior from
            // https://github.com/dashevo/platform/blob/6b02b26e5cd3a7c877c5fdfe40c4a4385a8dda15/packages/js-dpp/lib/stateTransition/AbstractStateTransition.js#L187
            // is to return the error for the BIP13_SCRIPT_HASH
            KeyType::BIP13_SCRIPT_HASH => {
                return Err(ProtocolError::InvalidIdentityPublicKeyTypeError {
                    public_key_type: key_type,
                })
            }
        };
        Ok(())
    }

    fn verify_by_public_key<T: BlsModule>(
        &self,
        public_key: &[u8],
        public_key_type: KeyType,
        bls: &T,
    ) -> Result<(), ProtocolError> {
        match public_key_type {
            KeyType::ECDSA_SECP256K1 => self.verify_ecdsa_signature_by_public_key(public_key),
            KeyType::ECDSA_HASH160 => {
                self.verify_ecdsa_hash_160_signature_by_public_key_hash(public_key)
            }
            KeyType::BLS12_381 => self.verify_bls_signature_by_public_key(public_key, bls),
            KeyType::BIP13_SCRIPT_HASH => {
                Err(ProtocolError::InvalidIdentityPublicKeyTypeError { public_key_type })
            }
        }
    }

    fn verify_ecdsa_hash_160_signature_by_public_key_hash(
        &self,
        public_key_hash: &[u8],
    ) -> Result<(), ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }
        let data_hash = self.hash(true)?;
        Ok(signer::verify_hash_signature(
            &data_hash,
            self.get_signature(),
            public_key_hash,
        )?)
    }

    /// Verifies an ECDSA signature with the public key
    fn verify_ecdsa_signature_by_public_key(&self, public_key: &[u8]) -> Result<(), ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }
        let data = self.to_buffer(true)?;
        Ok(signer::verify_data_signature(
            &data,
            self.get_signature(),
            public_key,
        )?)
    }

    /// Verifies a BLS signature with the public key
    fn verify_bls_signature_by_public_key<T: BlsModule>(
        &self,
        public_key: &[u8],
        bls: &T,
    ) -> Result<(), ProtocolError> {
        if self.get_signature().is_empty() {
            return Err(ProtocolError::StateTransitionIsNotIsSignedError {
                state_transition: self.clone().into(),
            });
        }

        let data = self.to_buffer(true)?;

        bls.verify_signature(self.get_signature(), &data, public_key)
            .map(|_| ())
    }

    /// returns true if state transition is a document state transition
    fn is_document_state_transition(&self) -> bool {
        DOCUMENT_TRANSITION_TYPES.contains(&self.get_type())
    }
    /// returns true if state transition is a data contract state transition
    fn is_data_contract_state_transition(&self) -> bool {
        DATA_CONTRACT_TRANSITION_TYPES.contains(&self.get_type())
    }
    /// return true if state transition is an identity state transition
    fn is_identity_state_transition(&self) -> bool {
        IDENTITY_TRANSITION_TYPE.contains(&self.get_type())
    }

    fn get_execution_context(&self) -> &StateTransitionExecutionContext;
    fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext;
    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext);
}

/// The trait contains methods related to conversion of StateTransition into different formats
pub trait StateTransitionConvert: Serialize {
    // TODO remove this as it is not necessary and can be hardcoded
    fn signature_property_paths() -> Vec<&'static str>;
    fn identifiers_property_paths() -> Vec<&'static str>;
    fn binary_property_paths() -> Vec<&'static str>;

    /// Returns the [`serde_json::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        state_transition_helpers::to_object(
            self,
            Self::signature_property_paths(),
            Self::identifiers_property_paths(),
            skip_signature,
        )
    }

    /// Returns the [`serde_json::Value`] instance that encodes:
    ///  - Identifiers  - with base58
    ///  - Binary data  - with base64
    fn to_json(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        state_transition_helpers::to_json(
            self,
            Self::binary_property_paths(),
            Self::signature_property_paths(),
            skip_signature,
        )
    }

    // Returns the cibor-encoded bytes representation of the object. The data is  prefixed by 4 bytes containing the Protocol Version
    fn to_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut json_value = self.to_object(skip_signature)?;
        let protocol_version = json_value.remove_u32(PROPERTY_PROTOCOL_VERSION)?;
        serializer::value_to_cbor(json_value, Some(protocol_version))
    }

    // Returns the hash of cibor-encoded bytes representation of the object
    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash(self.to_buffer(skip_signature)?))
    }
}

pub mod state_transition_helpers {
    use super::*;

    pub fn to_json<'a>(
        serializable: impl Serialize,
        binary_property_paths: impl IntoIterator<Item = &'a str>,
        signature_property_paths: impl IntoIterator<Item = &'a str>,
        skip_signature: bool,
    ) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(serializable)?;

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in signature_property_paths {
                    o.remove(path);
                }
            }
        }

        json_value.replace_binary_paths(binary_property_paths, ReplaceWith::Base64)?;

        Ok(json_value)
    }

    pub fn to_object<'a>(
        serializable: impl Serialize,
        signature_property_paths: impl IntoIterator<Item = &'a str>,
        identifier_property_paths: impl IntoIterator<Item = &'a str>,
        skip_signature: bool,
    ) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(serializable)?;

        // TODO: add error checking to `replace_identifier_paths`
        // `IdentityCreateTransition` has the custom serialization and converts the `Identifier` into the bytes (`String` is default).
        // `replace_identifier_paths()` returns an error because it expects a `String`.
        // When we change the default serialization for `Identifier` to bytes we should bring back the error checking
        json_value.replace_identifier_paths(identifier_property_paths, ReplaceWith::Bytes);

        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                for path in signature_property_paths {
                    o.remove(path);
                }
            }
        }
        Ok(json_value)
    }
}
