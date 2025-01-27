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

//! Query Tests History
//!

use std::collections::{BTreeMap, HashMap};
use std::fmt::{Debug, Formatter};
use std::option::Option::None;

use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_json::json;

use drive::common;
use drive::common::helpers::setup::setup_drive;
use drive::contract::{document::Document, Contract};
use drive::drive::batch::GroveDbOpBatch;
use drive::drive::config::DriveConfig;
use drive::drive::contract::add_init_contracts_structure_operations;
use drive::drive::flags::StorageFlags;
use drive::drive::object_size_info::DocumentInfo::DocumentRefAndSerialization;
use drive::drive::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::drive::Drive;
use drive::error::{query::QueryError, Error};
use drive::query::DriveQuery;

use dpp::data_contract::extra::DriveContractExt;
use drive::drive::block_info::BlockInfo;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Person {
    #[serde(rename = "$id")]
    id: Vec<u8>,
    #[serde(rename = "$ownerId")]
    owner_id: Vec<u8>,
    first_name: String,
    middle_name: String,
    last_name: String,
    message: Option<String>,
    age: u8,
}

impl Debug for Person {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Person")
            .field("id", &String::from_utf8_lossy(&self.id))
            .field("owner_id", &String::from_utf8_lossy(&self.owner_id))
            .field("first_name", &self.first_name)
            .field("middle_name", &self.middle_name)
            .field("last_name", &self.last_name)
            .field("age", &self.age)
            .field("message", &self.message)
            .finish()
    }
}

impl Person {
    fn random_people_for_block_times(
        count: usize,
        seed: u64,
        block_times: Vec<u64>,
    ) -> BTreeMap<u64, Vec<Self>> {
        let first_names =
            common::text_file_strings("tests/supporting_files/contract/family/first-names.txt");
        let middle_names =
            common::text_file_strings("tests/supporting_files/contract/family/middle-names.txt");
        let last_names =
            common::text_file_strings("tests/supporting_files/contract/family/last-names.txt");
        let quotes = common::text_file_strings("tests/supporting_files/contract/family/quotes.txt");
        let mut people: Vec<Person> = Vec::with_capacity(count as usize);

        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _ in 0..count {
            let person = Person {
                id: Vec::from(rng.gen::<[u8; 32]>()),
                owner_id: Vec::from(rng.gen::<[u8; 32]>()),
                first_name: first_names.choose(&mut rng).unwrap().clone(),
                middle_name: middle_names.choose(&mut rng).unwrap().clone(),
                last_name: last_names.choose(&mut rng).unwrap().clone(),
                message: None,
                age: rng.gen_range(0..85),
            };
            people.push(person);
        }

        let mut people_for_blocks: BTreeMap<u64, Vec<Person>> = BTreeMap::new();

        for block_time in block_times {
            let block_vec: Vec<Person> = people
                .iter()
                .map(|person| {
                    let mut quote = quotes.choose(&mut rng).unwrap().clone();
                    if quote.len() > 128 {
                        let quote_str = quote.as_str();
                        let mut end: usize = 0;
                        quote
                            .chars()
                            .into_iter()
                            .take(128)
                            .for_each(|x| end += x.len_utf8());
                        let sub_quote = &quote_str[..end];
                        quote = String::from(sub_quote);
                    }
                    Person {
                        id: person.id.clone(),
                        owner_id: person.owner_id.clone(),
                        first_name: person.first_name.clone(),
                        middle_name: person.middle_name.clone(),
                        last_name: person.last_name.clone(),
                        message: Some(quote),
                        age: person.age + ((block_time / 100) as u8),
                    }
                })
                .collect();
            people_for_blocks.insert(block_time, block_vec);
        }
        people_for_blocks
    }
}

/// Sets up the `family-contract-with-history` contract to test queries on.
pub fn setup(
    count: usize,
    restrict_to_inserts: Option<Vec<usize>>,
    with_batching: bool,
    seed: u64,
) -> (Drive, Contract) {
    let drive_config = if with_batching {
        DriveConfig::default_with_batches()
    } else {
        DriveConfig::default_without_batches()
    };

    let drive = setup_drive(Some(drive_config));

    let db_transaction = drive.grove.start_transaction();

    // Create contracts tree
    let mut batch = GroveDbOpBatch::new();

    add_init_contracts_structure_operations(&mut batch);

    drive
        .grove_apply_batch(batch, false, Some(&db_transaction))
        .expect("expected to create contracts tree successfully");

    // setup code
    let contract = common::setup_contract(
        &drive,
        "tests/supporting_files/contract/family/family-contract-with-history.json",
        None,
        Some(&db_transaction),
    );

    let block_times: Vec<u64> = vec![0, 15, 100, 1000];

    let people_at_block_times = Person::random_people_for_block_times(count, seed, block_times);

    for (block_time, people) in people_at_block_times {
        for (i, person) in people.iter().enumerate() {
            if let Some(range_insert) = &restrict_to_inserts {
                if !range_insert.contains(&i) {
                    continue;
                }
            }
            let value = serde_json::to_value(person).expect("serialized person");
            let document_cbor =
                common::value_to_cbor(value, Some(drive::drive::defaults::PROTOCOL_VERSION));
            let document = Document::from_cbor(document_cbor.as_slice(), None, None)
                .expect("document should be properly deserialized");
            let document_type = contract
                .document_type_for_name("person")
                .expect("expected to get document type");

            let storage_flags = Some(StorageFlags::SingleEpoch(0));

            // if block_time == 100 && i == 9 {
            //     dbg!("block time {} {} {:#?}",block_time, i, person);
            // }

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
                        contract: &contract,
                        document_type,
                    },
                    true,
                    BlockInfo::default_with_time(block_time),
                    true,
                    Some(&db_transaction),
                )
                .expect("expected to add document");
        }
    }
    drive
        .grove
        .commit_transaction(db_transaction)
        .unwrap()
        .expect("transaction should be committed");

    (drive, contract)
}

#[test]
fn test_setup() {
    let range_inserts = vec![0, 2];
    setup(10, Some(range_inserts), true, 73509);
}

#[test]
fn test_query_historical() {
    let (drive, contract) = setup(10, None, true, 73509);

    let db_transaction = drive.grove.start_transaction();

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");
    assert_eq!(
        root_hash.as_slice(),
        vec![
            49, 205, 177, 218, 169, 224, 236, 206, 112, 34, 163, 112, 222, 73, 92, 82, 189, 120,
            135, 32, 13, 65, 253, 139, 167, 209, 146, 1, 81, 127, 38, 61
        ]
    );

    let all_names = [
        "Adey".to_string(),
        "Briney".to_string(),
        "Cammi".to_string(),
        "Celinda".to_string(),
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Kevina".to_string(),
        "Meta".to_string(),
        "Noellyn".to_string(),
        "Prissie".to_string(),
    ];

    // A query getting all elements by firstName

    let query_value = json!({
        "where": [
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, Some(&db_transaction))
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    assert_eq!(names, all_names);

    // A query getting all people who's first name is Chris (which should exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // A query getting all people who's first name is Adey and lastName Randolf

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"],
            ["lastName", "==", "Randolf"]
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let document = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");
    let last_name = document
        .properties
        .get("lastName")
        .expect("we should be able to get the last name")
        .as_text()
        .expect("last name must be a string");

    assert_eq!(last_name, "Randolf");

    // A query getting all people who's first name is in a range with a single element Adey,
    // order by lastName (this should exist)

    let query_value = json!({
        "where": [
            ["firstName", "in", ["Adey"]]
        ],
        "orderBy": [
            ["firstName", "asc"],
            ["lastName", "asc"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // A query getting all people who's first name is Adey, order by lastName (which should exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Adey"]
        ],
        "orderBy": [
            ["lastName", "asc"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let document = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");
    let last_name = document
        .properties
        .get("lastName")
        .expect("we should be able to get the last name")
        .as_text()
        .expect("last name must be a string");

    assert_eq!(last_name, "Randolf");

    // A query getting all people who's first name is Chris (which is not exist)

    let query_value = json!({
        "where": [
            ["firstName", "==", "Chris"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 0);

    // A query getting a middle name

    let query_value = json!({
        "where": [
            ["middleName", "==", "Briggs"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            None,
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // A query getting all people who's first name is before Chris

    let query_value = json!({
        "where": [
            ["firstName", "<", "Chris"]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_before_chris = [
        "Adey".to_string(),
        "Briney".to_string(),
        "Cammi".to_string(),
        "Celinda".to_string(),
    ];
    assert_eq!(names, expected_names_before_chris);

    // A query getting all people who's first name is before Chris

    let query_value = json!({
        "where": [
            ["firstName", "StartsWith", "C"]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_names_before_chris = ["Cammi".to_string(), "Celinda".to_string()];
    assert_eq!(names, expected_names_before_chris);

    // A query getting all people who's first name is between Chris and Noellyn included

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 5);

    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_between_names = [
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Kevina".to_string(),
        "Meta".to_string(),
        "Noellyn".to_string(),
    ];

    assert_eq!(names, expected_between_names);

    // A query getting all people who's first name is between Chris and Noellyn included
    // However here there will be a startAt of the ID of Kevina

    // Let's first get the ID of Kevina
    let ids: HashMap<String, Vec<u8>> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let name = name_value
                .as_text()
                .expect("the first name should be a string")
                .to_string();
            (name, Vec::from(document.id))
        })
        .collect();

    let kevina_id = ids
        .get("Kevina")
        .expect("We should be able to get back Kevina's Id");
    let kevina_encoded_id = bs58::encode(kevina_id).into_string();

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAt": kevina_encoded_id, //Kevina
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 3);

    let reduced_names_after: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_reduced_names = [
        "Kevina".to_string(),
        "Meta".to_string(),
        "Noellyn".to_string(),
    ];

    assert_eq!(reduced_names_after, expected_reduced_names);

    // Now lets try startsAfter

    let query_value = json!({
        "where": [
            ["firstName", ">", "Chris"],
            ["firstName", "<=", "Noellyn"]
        ],
        "startAfter": kevina_encoded_id, //Kevina
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    assert_eq!(results.len(), 2);

    let reduced_names_after: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let expected_reduced_names = ["Meta".to_string(), "Noellyn".to_string()];

    assert_eq!(reduced_names_after, expected_reduced_names);

    // A query getting back elements having specific names

    let query_value = json!({
        "where": [
            ["firstName", "in", names]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    assert_eq!(names, expected_between_names);

    let query_value = json!({
        "where": [
            ["firstName", "in", names]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .clone()
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    let ages: Vec<u64> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let age_value = document
                .properties
                .get("age")
                .expect("we should be able to get the age");
            let age: u64 = age_value
                .as_integer()
                .expect("the age should be an integer")
                .try_into()
                .expect("the age should be put in an u64");
            age
        })
        .collect();

    let expected_reversed_between_names = [
        "Noellyn".to_string(),  // 40
        "Meta".to_string(),     // 69
        "Kevina".to_string(),   // 58
        "Gilligan".to_string(), // 59
        "Dalia".to_string(),    // 78
    ];

    assert_eq!(names, expected_reversed_between_names);

    let expected_ages = [40, 69, 58, 59, 78];

    assert_eq!(ages, expected_ages);

    // A query getting back elements having specific names and over a certain age

    let query_value = json!({
        "where": [
            ["firstName", "in", names],
            ["age", ">=", 45]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"],
            ["age", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    // Kevina is 55, and is excluded from this test
    let expected_names_45_over = [
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Kevina".to_string(),
        "Meta".to_string(),
    ];

    assert_eq!(names, expected_names_45_over);

    // A query getting back elements having specific names and over a certain age

    let query_value = json!({
        "where": [
            ["firstName", "in", names],
            ["age", ">", 58]
        ],
        "limit": 100,
        "orderBy": [
            ["firstName", "asc"],
            ["age", "desc"]
        ]
    });
    let where_cbor = common::value_to_cbor(query_value, None);
    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");
    let query = DriveQuery::from_cbor(where_cbor.as_slice(), &contract, person_document_type)
        .expect("query should be built");
    let (results, _, _) = query
        .execute_no_proof(&drive, None, None)
        .expect("proof should be executed");
    let names: Vec<String> = results
        .iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let first_name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let first_name = first_name_value
                .as_text()
                .expect("the first name should be a string");
            String::from(first_name)
        })
        .collect();

    // Kevina is 48 so she should be now excluded, Dalia is 68, Gilligan is 49 and Meta is 59

    let expected_names_over_48 = [
        "Dalia".to_string(),
        "Gilligan".to_string(),
        "Meta".to_string(),
    ];

    assert_eq!(names, expected_names_over_48);

    let ages: HashMap<String, u8> = results
        .into_iter()
        .map(|result| {
            let document = Document::from_cbor(result.as_slice(), None, None)
                .expect("we should be able to deserialize the cbor");
            let name_value = document
                .properties
                .get("firstName")
                .expect("we should be able to get the first name");
            let name = name_value
                .as_text()
                .expect("the first name should be a string")
                .to_string();
            let age_value = document
                .properties
                .get("age")
                .expect("we should be able to get the age");
            let age_integer = age_value.as_integer().expect("age should be an integer");
            let age: u8 = age_integer.try_into().expect("expected u8 value");
            (name, age)
        })
        .collect();

    let meta_age = ages
        .get("Meta")
        .expect("we should be able to get Kevina as she is 48");

    assert_eq!(*meta_age, 69);

    // fetching by $id
    let mut rng = rand::rngs::StdRng::seed_from_u64(84594);
    let id_bytes = bs58::decode("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD")
        .into_vec()
        .expect("this should decode");

    let owner_id_bytes = bs58::decode("BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj")
        .into_vec()
        .expect("this should decode");

    let fixed_person = Person {
        id: id_bytes,
        owner_id: owner_id_bytes,
        first_name: String::from("Wisdom"),
        middle_name: String::from("Madabuchukwu"),
        last_name: String::from("Ogwu"),
        message: Some(String::from("Oh no")),
        age: rng.gen_range(0..85),
    };
    let serialized_person = serde_json::to_value(&fixed_person).expect("serialized person");
    let person_cbor = common::value_to_cbor(
        serialized_person,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    );
    let document = Document::from_cbor(person_cbor.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let document_type = contract
        .document_type_for_name("person")
        .expect("expected to get document type");

    let storage_flags = Some(StorageFlags::SingleEpoch(0));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefAndSerialization((
                        &document,
                        &person_cbor,
                        storage_flags.as_ref(),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let id_two_bytes = bs58::decode("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")
        .into_vec()
        .expect("should decode");
    let owner_id_bytes = bs58::decode("Di8dtJXv3L2YnzDNUN4w5rWLPSsSAzv6hLMMQbg3eyVA")
        .into_vec()
        .expect("this should decode");
    let next_person = Person {
        id: id_two_bytes,
        owner_id: owner_id_bytes,
        first_name: String::from("Wdskdfslgjfdlj"),
        middle_name: String::from("Mdsfdsgsdl"),
        last_name: String::from("dkfjghfdk"),
        message: Some(String::from("Bad name")),
        age: rng.gen_range(0..85),
    };
    let serialized_person = serde_json::to_value(&next_person).expect("serialized person");
    let person_cbor = common::value_to_cbor(
        serialized_person,
        Some(drive::drive::defaults::PROTOCOL_VERSION),
    );
    let document = Document::from_cbor(person_cbor.as_slice(), None, None)
        .expect("document should be properly deserialized");

    let document_type = contract
        .document_type_for_name("person")
        .expect("expected to get document type");

    let storage_flags = Some(StorageFlags::SingleEpoch(0));

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefAndSerialization((
                        &document,
                        &person_cbor,
                        storage_flags.as_ref(),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::genesis(),
            true,
            Some(&db_transaction),
        )
        .expect("document should be inserted");

    let query_value = json!({
        "where": [
            ["$id", "in", vec![String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let query_value = json!({
        "where": [
            ["$id", "==", "6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179"]
        ]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    let query_value = json!({
        "where": [
            ["$id", "==", "6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179"]
        ],
        "blockTime": 300
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // fetching by $id with order by

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");

    assert_eq!(
        last_person.id,
        vec![
            76, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198, 189,
            116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20
        ]
        .as_slice()
    );

    // fetching by $id with order by desc

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "orderBy": [["$id", "desc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");

    assert_eq!(
        last_person.id,
        vec![
            140, 161, 17, 201, 152, 232, 129, 48, 168, 13, 49, 10, 218, 53, 118, 136, 165, 198,
            189, 116, 116, 22, 133, 92, 104, 165, 186, 249, 94, 81, 45, 20
        ]
        .as_slice()
    );

    //
    // // fetching with empty where and orderBy
    //
    let query_value = json!({});

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 12);

    //
    // // fetching with empty where and orderBy $id desc
    //
    let query_value = json!({
        "orderBy": [["$id", "desc"]]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 12);

    let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
        .expect("we should be able to deserialize the cbor");

    assert_eq!(
        last_person.id,
        vec![
            249, 170, 70, 122, 181, 31, 35, 176, 175, 131, 70, 150, 250, 223, 194, 203, 175, 200,
            107, 252, 199, 227, 154, 105, 89, 57, 38, 85, 236, 192, 254, 88
        ]
        .as_slice()
    );

    let message_value = last_person.properties.get("message").unwrap();

    let message = message_value
        .as_text()
        .expect("the message should be a string")
        .to_string();

    assert_eq!(
        message,
        String::from("“Since it’s the customer that pays our salary, our responsibility is to make the product they want, when they want it, and deliv")
    );

    //
    // // fetching with empty where and orderBy $id desc with a blockTime
    //
    let query_value = json!({
        "orderBy": [["$id", "desc"]],
        "blockTime": 300
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect_err("not yet implemented");

    // assert_eq!(results.len(), 12);
    //
    // let last_person = Document::from_cbor(results.first().unwrap().as_slice(), None, None)
    //     .expect("we should be able to deserialize the cbor");
    //
    // assert_eq!(
    //     last_person.id,
    //     vec![
    //         249, 170, 70, 122, 181, 31, 35, 176, 175, 131, 70, 150, 250, 223, 194, 203, 175, 200,
    //         107, 252, 199, 227, 154, 105, 89, 57, 38, 85, 236, 192, 254, 88
    //     ]
    //         .as_slice()
    // );
    //
    // let message_value = last_person.properties.get("message").unwrap();
    //
    // let message = message_value
    //     .as_text()
    //     .expect("the message should be a string")
    //     .to_string();
    //
    // assert_eq!(
    //     message,
    //     String::from("“Since it’s the customer that pays our salary, our responsibility is to make the product they want, when they want it, and deliver quality that satisfies them.” Retired factory worker, Kiyoshi Tsutsumi (Osono et al 2008, 136)")
    // );

    //
    // // fetching with ownerId in a set of values
    //
    let query_value = json!({
        "where": [
            ["$ownerId", "in", ["BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj", "Di8dtJXv3L2YnzDNUN4w5rWLPSsSAzv6hLMMQbg3eyVA"]]
        ],
        "orderBy": [["$ownerId", "desc"]]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 2);

    //
    // // fetching with ownerId equal and orderBy
    //
    let query_value = json!({
        "where": [
            ["$ownerId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"]
        ],
        "orderBy": [["$ownerId", "asc"]]
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let (results, _, _) = drive
        .query_documents_from_contract(
            &contract,
            person_document_type,
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 1);

    // query empty contract with nested path queries

    let contract_cbor = hex::decode("01000000a5632469645820b0248cd9a27f86d05badf475dd9ff574d63219cd60c52e2be1e540c2fdd713336724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e6572496458204c9bf0db6ae315c85465e9ef26e6a006de9673731d08d14881945ddef1b5c5f26776657273696f6e0169646f63756d656e7473a267636f6e74616374a56474797065666f626a65637467696e646963657381a3646e616d656f6f6e7765724964546f55736572496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a168746f557365724964636173636872657175697265648268746f557365724964697075626c69634b65796a70726f70657274696573a268746f557365724964a56474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f570636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e746966696572697075626c69634b6579a36474797065656172726179686d61784974656d73182169627974654172726179f5746164646974696f6e616c50726f70657274696573f46770726f66696c65a56474797065666f626a65637467696e646963657381a3646e616d65676f776e6572496466756e69717565f56a70726f7065727469657381a168246f776e6572496463617363687265717569726564826961766174617255726c6561626f75746a70726f70657274696573a26561626f7574a2647479706566737472696e67696d61784c656e67746818ff6961766174617255726ca3647479706566737472696e6766666f726d61746375726c696d61784c656e67746818ff746164646974696f6e616c50726f70657274696573f4").unwrap();

    drive
        .apply_contract_cbor(
            contract_cbor.clone(),
            None,
            BlockInfo::genesis(),
            true,
            StorageFlags::optional_default_as_ref(),
            Some(&db_transaction),
        )
        .expect("expected to apply contract successfully");

    let query_value = json!({
        "where": [
            ["$ownerId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"],
            ["toUserId", "==", "BYR3zJgXDuz1BYAkEagwSjVqTcE1gbqEojd6RwAGuMzj"],
        ],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let (results, _, _) = drive
        .query_documents_from_contract_cbor(
            contract_cbor.as_slice(),
            String::from("contact"),
            query_cbor.as_slice(),
            None,
            Some(&db_transaction),
        )
        .expect("query should be executed");

    assert_eq!(results.len(), 0);

    // using non existing document in startAt

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "startAt": String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF178"),
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        None,
        Some(&db_transaction),
    );

    assert!(
        matches!(result, Err(Error::Query(QueryError::StartDocumentNotFound(message))) if message == "startAt document not found")
    );

    // using non existing document in startAfter

    let query_value = json!({
        "where": [
            ["$id", "in", [String::from("ATxXeP5AvY4aeUFA6WRo7uaBKTBgPQCjTrgtNpCMNVRD"), String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF179")]],
        ],
        "startAfter": String::from("6A8SGgdmj2NtWCYoYDPDpbsYkq2MCbgi6Lx4ALLfF178"),
        "orderBy": [["$id", "asc"]],
    });

    let query_cbor = common::value_to_cbor(query_value, None);

    let person_document_type = contract
        .document_types()
        .get("person")
        .expect("contract should have a person document type");

    let result = drive.query_documents_from_contract(
        &contract,
        person_document_type,
        query_cbor.as_slice(),
        None,
        Some(&db_transaction),
    );

    assert!(
        matches!(result, Err(Error::Query(QueryError::StartDocumentNotFound(message))) if message == "startAfter document not found")
    );

    // validate eventual root hash

    let root_hash = drive
        .grove
        .root_hash(Some(&db_transaction))
        .unwrap()
        .expect("there is always a root hash");
    assert_eq!(
        root_hash.as_slice(),
        vec![
            200, 234, 81, 179, 120, 70, 117, 20, 202, 219, 197, 168, 20, 96, 55, 130, 62, 243, 181,
            198, 88, 50, 225, 68, 205, 54, 191, 136, 37, 65, 113, 200
        ]
    );
}
