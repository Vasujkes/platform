// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Deterministic Root Hash Tests
//!

use std::option::Option::None;

use drive::common;
use drive::common::setup_contract;
use drive::contract::document::Document;
use drive::contract::Contract;
use grovedb::{Element, Transaction, TransactionArg};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use drive::drive::config::DriveConfig;
use drive::drive::flags::StorageFlags;
use drive::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::drive::{Drive, RootTree};

use dpp::data_contract::extra::DriveContractExt;
use drive::drive::block_info::BlockInfo;

/// Contains the unique ID for a Dash identity.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Records {
    dash_unique_identity_id: Vec<u8>,
}

/// Info about a DPNS name.
// In the real dpns label is required, we make it optional here for a test
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Domain {
    #[serde(rename = "$id")]
    id: Vec<u8>,
    #[serde(rename = "$ownerId")]
    owner_id: Vec<u8>,
    label: Option<String>,
    normalized_label: Option<String>,
    normalized_parent_domain_name: String,
    records: Records,
    preorder_salt: Vec<u8>,
    subdomain_rules: bool,
}

impl Domain {
    /// Creates domains with random data for a given normalized parent domain name.
    fn random_domains_in_parent(
        count: u32,
        seed: u64,
        normalized_parent_domain_name: &str,
    ) -> Vec<Self> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let mut vec: Vec<Domain> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _i in 0..count {
            let label = first_names.choose(&mut rng).unwrap();
            let domain = Domain {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                label: Some(label.clone()),
                normalized_label: Some(label.to_lowercase()),
                normalized_parent_domain_name: normalized_parent_domain_name.to_string(),
                records: Records {
                    dash_unique_identity_id: Vec::from(rng.gen::<[u8; 32]>()),
                },
                preorder_salt: Vec::from(rng.gen::<[u8; 32]>()),
                subdomain_rules: false,
            };
            vec.push(domain);
        }
        vec
    }
}

/// Creates and adds to a contract domains with random data.
pub fn add_domains_to_contract(
    drive: &Drive,
    contract: &Contract,
    transaction: TransactionArg,
    count: u32,
    seed: u64,
) {
    let domains = Domain::random_domains_in_parent(count, seed, "dash");
    for domain in domains {
        let value = serde_json::to_value(&domain).expect("serialized domain");
        let document_cbor =
            common::value_to_cbor(value, Some(drive::drive::defaults::PROTOCOL_VERSION));
        let document = Document::from_cbor(document_cbor.as_slice(), None, None)
            .expect("document should be properly deserialized");
        let document_type = contract
            .document_type_for_name("domain")
            .expect("expected to get document type");

        let storage_flags = Some(StorageFlags::SingleEpoch(0));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefAndSerialization((
                            &document,
                            &document_cbor,
                            storage_flags.as_ref(),
                        )),
                        owner_id: None,
                    },
                    contract,
                    document_type,
                },
                true,
                BlockInfo::genesis(),
                true,
                transaction,
            )
            .expect("document should be inserted");
    }
}

/// Tests that the root hash is being calculated correctly after inserting empty subtrees into
/// the root tree and the DPNS contract.
fn test_root_hash_with_batches(drive: &Drive, db_transaction: &Transaction) {
    // [1644293142180] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "00"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "0000000000000000000000000000000000000000000000000000000000000000"

    drive
        .grove
        .insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::Identities),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "43704df09958f09576708d0770f6ddcb4e155b96539893f4202eebefb4b0355d"
    );

    //[1644293142181] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "02"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b"

    drive
        .grove
        .insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::PublicKeyHashesToIdentities),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "64cb5491af7bea172ffd49f92ca1f4e2205c219522c12e614f45d4ab7243b48d"
    );

    // [1644293142181] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "01"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "7a0501f5957bdf9cb3a8ff4966f02265f968658b7a9c62642cba1165e86642f5"

    drive
        .grove
        .insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::ContractDocuments),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "6cd486f8384d17bb61d52305732e59e1eac3cc96a4b67fa35694fc80f7eb8d5b"
    );

    // [1644293142182] INFO (35 on bf3bb2a2796a): createTree
    //     path: []
    //     pathHash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    //     key: "03"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "db56114e00fdd4c1f85c892bf35ac9a89289aaecb1ebd0a96cde606a748b5d71"

    drive
        .grove
        .insert(
            [],
            Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions),
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "e9db255a3a50e80ffee4dffe6ac601f9b55f293e87c2548ebbf718ba740f1362"
    );

    // [1644293142182] INFO (35 on bf3bb2a2796a): createTree
    //     path: [
    //       "03"
    //     ]
    //     pathHash: "084fed08b978af4d7d196a7446a86b58009e636b611db16211b65a9aadff29c5"
    //     key: "00"
    //     value: "0000000000000000000000000000000000000000000000000000000000000000"
    //     valueHash: "66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925"
    //     useTransaction: true
    //     type: "tree"
    //     method: "insert"
    //     appHash: "2bca13b0f7e68d9c0be5c7352484f8bfba5be6c78f094551c1a0f849f4d7cde0"

    drive
        .grove
        .insert(
            [Into::<&[u8; 1]>::into(RootTree::SpentAssetLockTransactions).as_slice()],
            &[0],
            Element::empty_tree(),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert tree");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "20b009a6f16c5b7e6748017d2c70109ad0395346a7eb749c6a786d723cdfd4f6"
    );

    // [1644295643055] INFO (36 on a5bc48c228d6): put
    //     path: [
    //       "00"
    //     ]
    //     pathHash: "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"
    //     key: "f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715"
    //     value: "01000000a46269645820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a7156762616c616e636500687265766973696f6e006a7075626c69634b65797381a662696400646461746158210328f474ce2d61d6fdb45c1fb437ddbf167924e6af3303c167f64d8c8857e39ca564747970650067707572706f73650068726561644f6e6c79f76d73656375726974794c6576656c00"
    //     valueHash: "d7fef03318e2db119a9f5a2d6bcbf9a03fc280b4f4a3f94307736be193c320d4"
    //     useTransaction: true
    //     type: "item"
    //     method: "insert"
    //     appHash: "53fa0a40055ee0930964751a9ef39a3f7cc43dee0fe03142af421cf29f2483ab"

    drive.grove.insert(
        [Into::<&[u8; 1]>::into(RootTree::Identities).as_slice()],
        hex::decode("f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715").unwrap().as_slice(),
        Element::new_item(hex::decode("01000000a46269645820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a7156762616c616e636500687265766973696f6e006a7075626c69634b65797381a662696400646461746158210328f474ce2d61d6fdb45c1fb437ddbf167924e6af3303c167f64d8c8857e39ca564747970650067707572706f73650068726561644f6e6c79f76d73656375726974794c6576656c00").unwrap()),
        None,
        Some(db_transaction),
    ).unwrap().expect("should insert");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "f1e35551a41455af65c7f07eb84777cf3c9204608ac84dce96382a20c72767a7"
    );

    // [1644295643057] INFO (36 on a5bc48c228d6): put
    //     path: [
    //       "02"
    //     ]
    //     pathHash: "dbc1b4c900ffe48d575b5da5c638040125f65db0fe3e24494b76ea986457d986"
    //     key: "6198bae2a577044d7975f4d1a06a8d13a9eab9b0"
    //     value: "815820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715"
    //     valueHash: "d8c99c5e59a7c1a1cd47aeeef820585df42a21070d0ece24f316061328212636"
    //     useTransaction: true
    //     type: "item"
    //     method: "insert"
    //     appHash: "09532362c92495bf45f2b3c44dc9b8a078d73aa5066d8104f8900900db7a4d17"

    drive
        .grove
        .insert(
            [Into::<&[u8; 1]>::into(RootTree::PublicKeyHashesToIdentities).as_slice()],
            hex::decode("6198bae2a577044d7975f4d1a06a8d13a9eab9b0")
                .unwrap()
                .as_slice(),
            Element::new_item(
                hex::decode(
                    "815820f00100b0c1e3762b8bc1421e113c76b2a635c5930b9abf2b336583be5987a715",
                )
                .unwrap(),
            ),
            None,
            Some(db_transaction),
        )
        .unwrap()
        .expect("should insert");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    assert_eq!(
        hex::encode(app_hash),
        "30a50a76ee0623c030c52db557447fe67dbeb7b814b615003b75de7e96c6b0e6"
    );

    let encoded_dpns_contract = hex::decode("01000000a5632469645820e668c659af66aee1e72c186dde7b5b7e0a1d712a09c40d5721f622bf53c531556724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458203012c19b98ec0033addb36cd64b7f510670f2a351a4304b5f6994144286efdac6776657273696f6e0169646f63756d656e7473a266646f6d61696ea66474797065666f626a65637467696e646963657383a3646e616d6572706172656e744e616d65416e644c6162656c66756e69717565f56a70726f7065727469657382a1781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d6563617363a16f6e6f726d616c697a65644c6162656c63617363a3646e616d656e646173684964656e74697479496466756e69717565f56a70726f7065727469657381a1781c7265636f7264732e64617368556e697175654964656e74697479496463617363a2646e616d656964617368416c6961736a70726f7065727469657381a1781b7265636f7264732e64617368416c6961734964656e746974794964636173636824636f6d6d656e74790137496e206f7264657220746f207265676973746572206120646f6d61696e20796f75206e65656420746f206372656174652061207072656f726465722e20546865207072656f726465722073746570206973206e656564656420746f2070726576656e74206d616e2d696e2d7468652d6d6964646c652061747461636b732e206e6f726d616c697a65644c6162656c202b20272e27202b206e6f726d616c697a6564506172656e74446f6d61696e206d757374206e6f74206265206c6f6e676572207468616e20323533206368617273206c656e67746820617320646566696e65642062792052464320313033352e20446f6d61696e20646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e20617265207265737472696374656468726571756972656486656c6162656c6f6e6f726d616c697a65644c6162656c781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d656c7072656f7264657253616c74677265636f7264736e737562646f6d61696e52756c65736a70726f70657274696573a6656c6162656ca5647479706566737472696e67677061747465726e782a5e5b612d7a412d5a302d395d5b612d7a412d5a302d392d5d7b302c36317d5b612d7a412d5a302d395d24696d61784c656e677468183f696d696e4c656e677468036b6465736372697074696f6e7819446f6d61696e206c6162656c2e20652e672e2027426f62272e677265636f726473a66474797065666f626a6563746824636f6d6d656e747890436f6e73747261696e742077697468206d617820616e64206d696e2070726f7065727469657320656e737572652074686174206f6e6c79206f6e65206964656e74697479207265636f72642069732075736564202d206569746865722061206064617368556e697175654964656e74697479496460206f722061206064617368416c6961734964656e746974794964606a70726f70657274696573a27364617368416c6961734964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783d4964656e7469747920494420746f206265207573656420746f2063726561746520616c696173206e616d657320666f7220746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965727464617368556e697175654964656e746974794964a764747970656561727261796824636f6d6d656e7478234d75737420626520657175616c20746f2074686520646f63756d656e74206f776e6572686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e783e4964656e7469747920494420746f206265207573656420746f2063726561746520746865207072696d617279206e616d6520746865204964656e7469747970636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726d6d617850726f70657274696573016d6d696e50726f7065727469657301746164646974696f6e616c50726f70657274696573f46c7072656f7264657253616c74a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e782253616c74207573656420696e20746865207072656f7264657220646f63756d656e746e737562646f6d61696e52756c6573a56474797065666f626a656374687265717569726564816f616c6c6f77537562646f6d61696e736a70726f70657274696573a16f616c6c6f77537562646f6d61696e73a3647479706567626f6f6c65616e6824636f6d6d656e74784f4f6e6c792074686520646f6d61696e206f776e657220697320616c6c6f77656420746f2063726561746520737562646f6d61696e7320666f72206e6f6e20746f702d6c6576656c20646f6d61696e736b6465736372697074696f6e785b54686973206f7074696f6e20646566696e65732077686f2063616e2063726561746520737562646f6d61696e733a2074727565202d20616e796f6e653b2066616c7365202d206f6e6c792074686520646f6d61696e206f776e65726b6465736372697074696f6e7842537562646f6d61696e2072756c657320616c6c6f7720646f6d61696e206f776e65727320746f20646566696e652072756c657320666f7220737562646f6d61696e73746164646974696f6e616c50726f70657274696573f46f6e6f726d616c697a65644c6162656ca5647479706566737472696e67677061747465726e78215e5b612d7a302d395d5b612d7a302d392d5d7b302c36317d5b612d7a302d395d246824636f6d6d656e7478694d75737420626520657175616c20746f20746865206c6162656c20696e206c6f776572636173652e20546869732070726f70657274792077696c6c20626520646570726563617465642064756520746f206361736520696e73656e73697469766520696e6469636573696d61784c656e677468183f6b6465736372697074696f6e7850446f6d61696e206c6162656c20696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e2027626f6227781a6e6f726d616c697a6564506172656e74446f6d61696e4e616d65a6647479706566737472696e67677061747465726e78285e247c5e5b5b612d7a302d395d5b612d7a302d392d5c2e5d7b302c3138387d5b612d7a302d395d246824636f6d6d656e74788c4d7573742065697468657220626520657175616c20746f20616e206578697374696e6720646f6d61696e206f7220656d70747920746f20637265617465206120746f70206c6576656c20646f6d61696e2e204f6e6c7920746865206461746120636f6e7472616374206f776e65722063616e2063726561746520746f70206c6576656c20646f6d61696e732e696d61784c656e67746818be696d696e4c656e677468006b6465736372697074696f6e785e412066756c6c20706172656e7420646f6d61696e206e616d6520696e206c6f7765726361736520666f7220636173652d696e73656e73697469766520756e697175656e6573732076616c69646174696f6e2e20652e672e20276461736827746164646974696f6e616c50726f70657274696573f4687072656f72646572a66474797065666f626a65637467696e646963657381a3646e616d656a73616c7465644861736866756e69717565f56a70726f7065727469657381a17073616c746564446f6d61696e48617368636173636824636f6d6d656e74784a5072656f7264657220646f63756d656e74732061726520696d6d757461626c653a206d6f64696669636174696f6e20616e642064656c6574696f6e206172652072657374726963746564687265717569726564817073616c746564446f6d61696e486173686a70726f70657274696573a17073616c746564446f6d61696e48617368a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e7859446f75626c65207368612d323536206f662074686520636f6e636174656e6174696f6e206f66206120333220627974652072616e646f6d2073616c7420616e642061206e6f726d616c697a656420646f6d61696e206e616d65746164646974696f6e616c50726f70657274696573f4").unwrap();

    drive
        .apply_contract_cbor(
            encoded_dpns_contract,
            None,
            BlockInfo::genesis(),
            true,
            StorageFlags::optional_default_as_ref(),
            Some(db_transaction),
        )
        .expect("apply contract");

    let app_hash = drive
        .grove
        .root_hash(Some(db_transaction))
        .unwrap()
        .expect("should return app hash");

    let expected_app_hash = "7eb40b3af9e6a78915fb0d4a8819c7ae5cb98575c77f1d587661857666682c51";

    assert_eq!(hex::encode(app_hash), expected_app_hash);
}

/// Runs `test_root_hash_with_batches` 10 times.
#[test]
fn test_deterministic_root_hash_with_batches() {
    let tmp_dir = TempDir::new().unwrap();
    let drive: Drive = Drive::open(tmp_dir, Some(DriveConfig::default_with_batches()))
        .expect("expected to open Drive successfully");

    let db_transaction = drive.grove.start_transaction();

    for _ in 0..10 {
        test_root_hash_with_batches(&drive, &db_transaction);

        drive
            .grove
            .rollback_transaction(&db_transaction)
            .expect("transaction should be rolled back");
    }
}

/// Tests that the root hashes are the same between a Drive with and without batches.
/// Employs the empty root tree with the DPNS contract.
#[ignore]
#[test]
fn test_root_hash_matches_with_batching_just_contract() {
    let tmp_dir_1 = TempDir::new().unwrap();
    let tmp_dir_2 = TempDir::new().unwrap();
    let drive_with_batches: Drive =
        Drive::open(&tmp_dir_1, Some(DriveConfig::default_with_batches()))
            .expect("expected to open Drive successfully");
    let drive_without_batches: Drive =
        Drive::open(&tmp_dir_2, Some(DriveConfig::default_without_batches()))
            .expect("expected to open Drive successfully");

    let db_transaction_with_batches = drive_with_batches.grove.start_transaction();
    let db_transaction_without_batches = drive_without_batches.grove.start_transaction();

    drive_with_batches
        .create_initial_state_structure(Some(&db_transaction_with_batches))
        .expect("expected to create root tree successfully");

    drive_without_batches
        .create_initial_state_structure(Some(&db_transaction_without_batches))
        .expect("expected to create root tree successfully");

    // setup code
    setup_contract(
        &drive_with_batches,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction_with_batches),
    );

    setup_contract(
        &drive_without_batches,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction_without_batches),
    );

    let root_hash_with_batches = drive_with_batches
        .grove
        .root_hash(Some(&db_transaction_with_batches))
        .unwrap()
        .expect("there is always a root hash");

    let root_hash_without_batches = drive_without_batches
        .grove
        .root_hash(Some(&db_transaction_without_batches))
        .unwrap()
        .expect("there is always a root hash");

    assert_eq!(root_hash_with_batches, root_hash_without_batches);
}

/// Tests that the root hashes are the same between a Drive with and without batches.
/// Employs the empty root tree with the DPNS contract and one document.
#[ignore]
#[test]
fn test_root_hash_matches_with_batching_contract_and_one_document() {
    let tmp_dir_1 = TempDir::new().unwrap();
    let tmp_dir_2 = TempDir::new().unwrap();
    let drive_with_batches: Drive =
        Drive::open(&tmp_dir_1, Some(DriveConfig::default_with_batches()))
            .expect("expected to open Drive successfully");
    let drive_without_batches: Drive =
        Drive::open(&tmp_dir_2, Some(DriveConfig::default_without_batches()))
            .expect("expected to open Drive successfully");

    let db_transaction_with_batches = drive_with_batches.grove.start_transaction();
    let db_transaction_without_batches = drive_without_batches.grove.start_transaction();

    drive_with_batches
        .create_initial_state_structure(Some(&db_transaction_with_batches))
        .expect("expected to create root tree successfully");

    drive_without_batches
        .create_initial_state_structure(Some(&db_transaction_without_batches))
        .expect("expected to create root tree successfully");

    // setup code
    let contract_with_batches = setup_contract(
        &drive_with_batches,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction_with_batches),
    );

    let contract_without_batches = setup_contract(
        &drive_without_batches,
        "tests/supporting_files/contract/dpns/dpns-contract.json",
        None,
        Some(&db_transaction_without_batches),
    );

    add_domains_to_contract(
        &drive_with_batches,
        &contract_with_batches,
        Some(&db_transaction_with_batches),
        1,
        5,
    );
    add_domains_to_contract(
        &drive_without_batches,
        &contract_without_batches,
        Some(&db_transaction_without_batches),
        1,
        5,
    );

    let root_hash_with_batches = drive_with_batches
        .grove
        .root_hash(Some(&db_transaction_with_batches))
        .unwrap()
        .expect("there is always a root hash");

    let root_hash_without_batches = drive_without_batches
        .grove
        .root_hash(Some(&db_transaction_without_batches))
        .unwrap()
        .expect("there is always a root hash");

    assert_eq!(root_hash_with_batches, root_hash_without_batches);
}
