// Allowed since it's actually better to panic during chain setup when there is an error
#![allow(clippy::unwrap_used)]

use super::*;

pub fn greg_config() -> Result<ChainSpec, String> {
    let wasm_binary = WASM_BINARY.ok_or_else(|| "Development wasm not available".to_string())?;

    // Give front-ends necessary data to present to users
    let mut properties = sc_service::Properties::new();
    properties.insert("tokenSymbol".into(), "TAO".into());
    properties.insert("tokenDecimals".into(), 9.into());
    properties.insert("ss58Format".into(), 13116.into());

    Ok(ChainSpec::builder(
        wasm_binary,
        Extensions {
            bad_blocks: Some(HashSet::from_iter(vec![
                // Example bad block
                H256::from_str(
                    "0xc174d485de4bc3813ac249fe078af605c74ff91d07b0a396cf75fa04f81fa312",
                )
                .unwrap(),
            ])),
            ..Default::default()
        },
    )
    .with_name("Bittensor")
    .with_id("bittensor")
    .with_chain_type(ChainType::Development)
    .with_genesis_config_patch(greg_genesis(
        // Initial PoA authorities (Validators)
        // aura | grandpa
        vec![

            // Carrot 1
            authority_keys_from_ss58(
                "5HBsw86VeEzBQDiMWeJJyLE9dEdF7t1gwLBoVSX1pF7YzfFr",
                "5HSgFsyEMF72xHKvWCjQ6HMj5J3Cy6GQBzQQMS2YrJBxFZk5",
            ),

            // Carrot 2
            authority_keys_from_ss58(
                "5Ccyfzo6GsW9exYfi2hNCbDsN4in6pcVNZ3wv5TVEJWww9nm",
                "5EmuLLKtsP9duyFSwkdRgbizof5BZ6Gm5wHNGLBJzwQhntfz",
            ),

            // Jake 1
            authority_keys_from_ss58(
                "5HmNpArQYoDpLpEV6DBNjxb4dkAz77E6PDYRYSrgKotGZ2PA",
                "5DFHHqfJBtE3rYu6T5vophsiRGJua4nqHrLZn9MBeAbHbgpE",
            ),

            // Jake 2
            authority_keys_from_ss58(
                "5HmNpArQYoDpLpEV6DBNjxb4dkAz77E6PDYRYSrgKotGZ2PA",
                "5DFHHqfJBtE3rYu6T5vophsiRGJua4nqHrLZn9MBeAbHbgpE",
            ),

            // Cole 1
            authority_keys_from_ss58(
                "5Gq4RDoMYSvgvU4mcyDRt17JUpjnbMvW6nEbptmbZzNzuTHR",
                "5DAtZCrpE73sVPdDAcuD3NxB8KikRNkzktRUuCXed964KP3Z",
            ),

            // Cole 2
            authority_keys_from_ss58(
                "5HmNpArQYoDpLpEV6DBNjxb4dkAz77E6PDYRYSrgKotGZ2PA",
                "5DFHHqfJBtE3rYu6T5vophsiRGJua4nqHrLZn9MBeAbHbgpE",
            )
            
        ],
        // Pre-funded accounts
        true,
    ))
    .with_properties(properties)
    .build())
}

fn greg_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    _enable_println: bool,
) -> serde_json::Value {
    let mut balances = vec![
        ( <AccountId32 as Ss58Codec>::from_ss58check("5GbECKBx5pMS77NZNYNq2mpNftJfNMX7bLTgKjqydn95qRkZ").unwrap(), 1_000_000_000u128 )
    ];

    let trimvirate_members: Vec<AccountId> = bounded_vec![
        <AccountId32 as Ss58Codec>::from_ss58check("5Dt7YztrcmjRbVCo1LQEZ3jQDDV4pXQoMu1rafifrn1YyGL7").unwrap(),
        <AccountId32 as Ss58Codec>::from_ss58check("5CPHH6BTxVbxbhuDimaeKsXmTYAKVDf1uxezYNvnvypnxSdp").unwrap(),
        <AccountId32 as Ss58Codec>::from_ss58check("5Gb1TTe3qCWsdwcjTSVrnLmFbvYeLrMSWezYRveJnid1hQiT").unwrap(),
    ];

    let senate_members: Vec<AccountId> = bounded_vec![
        <AccountId32 as Ss58Codec>::from_ss58check("5EtbWmsLCvJbJGCHxeY4tNjWFsVnrynjxawUfN5K9MXcDoxg").unwrap(),
        <AccountId32 as Ss58Codec>::from_ss58check("5FxuRpnN2RnaCo8Eabt4ev3UnpV9LZYUH8YFeU8MgWbNgy2k").unwrap(),
        <AccountId32 as Ss58Codec>::from_ss58check("5Ehj325iDSup1pdUbDFxDsnGzWdP9C4iumsybtdPSWxHyk1t").unwrap(),
    ];

    serde_json::json!({
        "balances": { "balances": balances },
        "aura": {
            "authorities": initial_authorities.iter().map(|x| (x.0.clone())).collect::<Vec<_>>()
        },
        "grandpa": {
            "authorities": initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect::<Vec<_>>()
        },
        "sudo": {
            "key": Some(<AccountId32 as Ss58Codec>::from_ss58check("5GbECKBx5pMS77NZNYNq2mpNftJfNMX7bLTgKjqydn95qRkZ").unwrap())
        },
        "triumvirateMembers": {
            "members": trimvirate_members
        },
        "senateMembers": {
            "members": senate_members,
        },
    })
}