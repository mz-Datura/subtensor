#![allow(clippy::indexing_slicing)]
use crate::mock::*;
use frame_support::{assert_err, assert_noop, assert_ok};
mod mock;
use pallet_subtensor::*;
use sp_core::U256;
use sp_runtime::traits::BadOrigin;

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_success --exact --nocapture
#[test]
fn test_do_set_child_singular_success() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, child)]
        ));

        // Verify child assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion, child)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_network_does_not_exist --exact --nocapture
#[test]
fn test_do_set_child_singular_network_does_not_exist() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 999; // Non-existent network
        let proportion: u64 = 1000;

        // Attempt to set child
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, child)]
            ),
            Error::<Test>::SubNetworkDoesNotExist
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_invalid_child --exact --nocapture
#[test]
fn test_do_set_child_singular_invalid_child() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Attempt to set child as the same hotkey
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![
                    (proportion, hotkey) // Invalid child
                ]
            ),
            Error::<Test>::InvalidChild
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_non_associated_coldkey --exact --nocapture
#[test]
fn test_do_set_child_singular_non_associated_coldkey() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey with a different coldkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, U256::from(999), 0);

        // Attempt to set child
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, child)]
            ),
            Error::<Test>::NonAssociatedColdKey
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_root_network --exact --nocapture
#[test]
fn test_do_set_child_singular_root_network() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = SubtensorModule::get_root_netuid(); // Root network
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);

        // Attempt to set child
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, child)]
            ),
            Error::<Test>::RegistrationNotPermittedOnRootSubnet
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_old_children_cleanup --exact --nocapture
#[test]
fn test_do_set_child_singular_old_children_cleanup() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let old_child = U256::from(3);
        let new_child = U256::from(4);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set old child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, old_child)]
        ));

        // Set new child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, new_child)]
        ));

        // Verify old child is removed
        let old_child_parents = SubtensorModule::get_parents(&old_child, netuid);
        assert!(old_child_parents.is_empty());

        // Verify new child assignment
        let new_child_parents = SubtensorModule::get_parents(&new_child, netuid);
        assert_eq!(new_child_parents, vec![(proportion, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_old_children_cleanup --exact --nocapture
#[test]
fn test_do_set_child_singular_new_children_assignment() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, child)]
        ));

        // Verify child assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion, child)]);

        // Verify parent assignment
        let parents = SubtensorModule::get_parents(&child, netuid);
        assert_eq!(parents, vec![(proportion, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_proportion_edge_cases --exact --nocapture
#[test]
fn test_do_set_child_singular_proportion_edge_cases() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set child with minimum proportion
        let min_proportion: u64 = 0;
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(min_proportion, child)]
        ));

        // Verify child assignment with minimum proportion
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(min_proportion, child)]);

        // Set child with maximum proportion
        let max_proportion: u64 = u64::MAX;
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(max_proportion, child)]
        ));

        // Verify child assignment with maximum proportion
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(max_proportion, child)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_child_singular_multiple_children --exact --nocapture
#[test]
fn test_do_set_child_singular_multiple_children() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 1;
        let proportion1: u64 = 500;
        let proportion2: u64 = 500;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set first child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion1, child1)]
        ));

        // Set second child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion2, child2)]
        ));

        // Verify children assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion2, child2)]);

        // Verify parent assignment for both children
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert!(parents1.is_empty()); // Old child should be removed

        let parents2 = SubtensorModule::get_parents(&child2, netuid);
        assert_eq!(parents2, vec![(proportion2, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_add_singular_child --exact --nocapture
#[test]
#[cfg(not(tarpaulin))]
fn test_add_singular_child() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let child = U256::from(1);
        let hotkey = U256::from(1);
        let coldkey = U256::from(2);
        assert_eq!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(u64::MAX, child)]
            ),
            Err(Error::<Test>::SubNetworkDoesNotExist.into())
        );
        add_network(netuid, 0, 0);
        assert_eq!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(u64::MAX, child)]
            ),
            Err(Error::<Test>::NonAssociatedColdKey.into())
        );
        SubtensorModule::create_account_if_non_existent(&coldkey, &hotkey);
        assert_eq!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(u64::MAX, child)]
            ),
            Err(Error::<Test>::InvalidChild.into())
        );
        let child = U256::from(3);
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(u64::MAX, child)]
        ));
    })
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_get_stake_for_hotkey_on_subnet --exact --nocapture
#[test]
fn test_get_stake_for_hotkey_on_subnet() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let hotkey0 = U256::from(1);
        let hotkey1 = U256::from(2);
        let coldkey0 = U256::from(3);
        let coldkey1 = U256::from(4);

        add_network(netuid, 0, 0);

        let max_stake: u64 = 3000;
        SubtensorModule::set_network_max_stake(netuid, max_stake);

        SubtensorModule::create_account_if_non_existent(&coldkey0, &hotkey0);
        SubtensorModule::create_account_if_non_existent(&coldkey1, &hotkey1);

        SubtensorModule::increase_stake_on_coldkey_hotkey_account(&coldkey0, &hotkey0, 1000);
        SubtensorModule::increase_stake_on_coldkey_hotkey_account(&coldkey0, &hotkey1, 1000);
        SubtensorModule::increase_stake_on_coldkey_hotkey_account(&coldkey1, &hotkey0, 1000);
        SubtensorModule::increase_stake_on_coldkey_hotkey_account(&coldkey1, &hotkey1, 1000);

        assert_eq!(SubtensorModule::get_total_stake_for_hotkey(&hotkey0), 2000);
        assert_eq!(SubtensorModule::get_total_stake_for_hotkey(&hotkey1), 2000);

        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey0, netuid),
            2000
        );
        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey1, netuid),
            2000
        );

        // Set child relationship
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey0),
            hotkey0,
            netuid,
            vec![(u64::MAX, hotkey1)]
        ));

        // Check stakes after setting child
        let stake0 = SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey0, netuid);
        let stake1 = SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey1, netuid);

        assert_eq!(stake0, 0);
        assert_eq!(stake1, max_stake);

        // Change child relationship to 50%
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey0),
            hotkey0,
            netuid,
            vec![(u64::MAX / 2, hotkey1)]
        ));

        // Check stakes after changing child relationship
        let stake0 = SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey0, netuid);
        let stake1 = SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey1, netuid);

        assert_eq!(stake0, 1001);
        assert!(stake1 >= max_stake - 1 && stake1 <= max_stake);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_child_singular_success --exact --nocapture
#[test]
fn test_do_revoke_child_singular_success() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, child)]
        ));

        // Verify child assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion, child)]);

        // Revoke child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![]
        ));

        // Verify child removal
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert!(children.is_empty());

        // Verify parent removal
        let parents = SubtensorModule::get_parents(&child, netuid);
        assert!(parents.is_empty());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_child_singular_network_does_not_exist --exact --nocapture
#[test]
fn test_do_revoke_child_singular_network_does_not_exist() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 999; // Non-existent network

        // Attempt to revoke child
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![]
            ),
            Error::<Test>::SubNetworkDoesNotExist
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_child_singular_non_associated_coldkey --exact --nocapture
#[test]
fn test_do_revoke_child_singular_non_associated_coldkey() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;

        // Add network and register hotkey with a different coldkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, U256::from(999), 0);

        // Attempt to revoke child
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![]
            ),
            Error::<Test>::NonAssociatedColdKey
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_child_singular_child_not_associated --exact --nocapture
#[test]
fn test_do_revoke_child_singular_child_not_associated() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        // Attempt to revoke child that is not associated
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(u64::MAX, child)]
            ),
            Error::<Test>::NonAssociatedColdKey
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_success --exact --nocapture
#[test]
fn test_do_set_children_multiple_success() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 1;
        let proportion1: u64 = 1000;
        let proportion2: u64 = 2000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set multiple children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion1, child1), (proportion2, child2)]
        ));

        // Verify children assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion1, child1), (proportion2, child2)]);

        // Verify parent assignment for both children
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert_eq!(parents1, vec![(proportion1, hotkey)]);

        let parents2 = SubtensorModule::get_parents(&child2, netuid);
        assert_eq!(parents2, vec![(proportion2, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_network_does_not_exist --exact --nocapture
#[test]
fn test_do_set_children_multiple_network_does_not_exist() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let netuid: u16 = 999; // Non-existent network
        let proportion: u64 = 1000;

        // Attempt to set children
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, child1)]
            ),
            Error::<Test>::SubNetworkDoesNotExist
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_invalid_child --exact --nocapture
#[test]
fn test_do_set_children_multiple_invalid_child() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Attempt to set child as the same hotkey
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, hotkey)]
            ),
            Error::<Test>::InvalidChild
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_non_associated_coldkey --exact --nocapture
#[test]
fn test_do_set_children_multiple_non_associated_coldkey() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey with a different coldkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, U256::from(999), 0);

        // Attempt to set children
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, child)]
            ),
            Error::<Test>::NonAssociatedColdKey
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_root_network --exact --nocapture
#[test]
fn test_do_set_children_multiple_root_network() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = SubtensorModule::get_root_netuid(); // Root network
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);

        // Attempt to set children
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(proportion, child)]
            ),
            Error::<Test>::RegistrationNotPermittedOnRootSubnet
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_old_children_cleanup --exact --nocapture
#[test]
fn test_do_set_children_multiple_old_children_cleanup() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let old_child = U256::from(3);
        let new_child1 = U256::from(4);
        let new_child2 = U256::from(5);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set old child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, old_child)]
        ));

        // Set new children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, new_child1), (proportion, new_child2)]
        ));

        // Verify old child is removed
        let old_child_parents = SubtensorModule::get_parents(&old_child, netuid);
        assert!(old_child_parents.is_empty());

        // Verify new children assignment
        let new_child1_parents = SubtensorModule::get_parents(&new_child1, netuid);
        assert_eq!(new_child1_parents, vec![(proportion, hotkey)]);

        let new_child2_parents = SubtensorModule::get_parents(&new_child2, netuid);
        assert_eq!(new_child2_parents, vec![(proportion, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_proportion_edge_cases --exact --nocapture
#[test]
fn test_do_set_children_multiple_proportion_edge_cases() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set children with minimum and maximum proportions
        let min_proportion: u64 = 0;
        let max_proportion: u64 = u64::MAX;
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(min_proportion, child1), (max_proportion, child2)]
        ));

        // Verify children assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(
            children,
            vec![(min_proportion, child1), (max_proportion, child2)]
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_overwrite_existing --exact --nocapture
#[test]
fn test_do_set_children_multiple_overwrite_existing() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let child3 = U256::from(5);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set initial children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, child1), (proportion, child2)]
        ));

        // Overwrite with new children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion * 2, child2), (proportion * 3, child3)]
        ));

        // Verify final children assignment
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(
            children,
            vec![(proportion * 2, child2), (proportion * 3, child3)]
        );

        // Verify parent assignment for all children
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert!(parents1.is_empty());

        let parents2 = SubtensorModule::get_parents(&child2, netuid);
        assert_eq!(parents2, vec![(proportion * 2, hotkey)]);

        let parents3 = SubtensorModule::get_parents(&child3, netuid);
        assert_eq!(parents3, vec![(proportion * 3, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_set_children_multiple_empty_list --exact --nocapture
#[test]
fn test_do_set_children_multiple_empty_list() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set empty children list
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![]
        ));

        // Verify children assignment is empty
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert!(children.is_empty());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_success --exact --nocapture
#[test]
fn test_do_revoke_children_multiple_success() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 1;
        let proportion1: u64 = 1000;
        let proportion2: u64 = 2000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set multiple children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion1, child1), (proportion2, child2)]
        ));

        // Revoke multiple children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![]
        ));

        // Verify children removal
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert!(children.is_empty());

        // Verify parent removal for both children
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert!(parents1.is_empty());

        let parents2 = SubtensorModule::get_parents(&child2, netuid);
        assert!(parents2.is_empty());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_network_does_not_exist --exact --nocapture
#[test]
fn test_do_revoke_children_multiple_network_does_not_exist() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 999; // Non-existent network
                               // Attempt to revoke children
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(u64::MAX / 2, child1), (u64::MAX / 2, child2)]
            ),
            Error::<Test>::SubNetworkDoesNotExist
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_non_associated_coldkey --exact --nocapture
#[test]
fn test_do_revoke_children_multiple_non_associated_coldkey() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 1;

        // Add network and register hotkey with a different coldkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, U256::from(999), 0);

        // Attempt to revoke children
        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                vec![(u64::MAX / 2, child1), (u64::MAX / 2, child2)]
            ),
            Error::<Test>::NonAssociatedColdKey
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_partial_revocation --exact --nocapture
#[test]
fn test_do_revoke_children_multiple_partial_revocation() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let child3 = U256::from(5);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set multiple children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![
                (proportion, child1),
                (proportion, child2),
                (proportion, child3)
            ]
        ));

        // Revoke only child3
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, child1), (proportion, child2)]
        ));

        // Verify children removal
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion, child1), (proportion, child2)]);

        // Verify parents.
        let parents1 = SubtensorModule::get_parents(&child3, netuid);
        assert!(parents1.is_empty());
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert_eq!(parents1, vec![(proportion, hotkey)]);
        let parents2 = SubtensorModule::get_parents(&child2, netuid);
        assert_eq!(parents2, vec![(proportion, hotkey)]);
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_non_existent_children --exact --nocapture

#[test]
fn test_do_revoke_children_multiple_non_existent_children() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let netuid: u16 = 1;
        let proportion: u64 = 1000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set one child
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion, child1)]
        ));

        // Attempt to revoke existing and non-existent children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![]
        ));

        // Verify all children are removed
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert!(children.is_empty());

        // Verify parent removal for the existing child
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert!(parents1.is_empty());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_empty_list --exact --nocapture
#[test]
fn test_do_revoke_children_multiple_empty_list() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Attempt to revoke with an empty list
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![]
        ));

        // Verify no changes in children
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert!(children.is_empty());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_do_revoke_children_multiple_complex_scenario --exact --nocapture
#[test]
fn test_do_revoke_children_multiple_complex_scenario() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let child3 = U256::from(5);
        let netuid: u16 = 1;
        let proportion1: u64 = 1000;
        let proportion2: u64 = 2000;
        let proportion3: u64 = 3000;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set multiple children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![
                (proportion1, child1),
                (proportion2, child2),
                (proportion3, child3)
            ]
        ));

        // Revoke child2
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(proportion1, child1), (proportion3, child3)]
        ));

        // Verify remaining children
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert_eq!(children, vec![(proportion1, child1), (proportion3, child3)]);

        // Verify parent removal for child2
        let parents2 = SubtensorModule::get_parents(&child2, netuid);
        assert!(parents2.is_empty());

        // Revoke remaining children
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![]
        ));

        // Verify all children are removed
        let children = SubtensorModule::get_children(&hotkey, netuid);
        assert!(children.is_empty());

        // Verify parent removal for all children
        let parents1 = SubtensorModule::get_parents(&child1, netuid);
        assert!(parents1.is_empty());
        let parents3 = SubtensorModule::get_parents(&child3, netuid);
        assert!(parents3.is_empty());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_get_network_max_stake --exact --nocapture
#[test]
fn test_get_network_max_stake() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let default_max_stake = SubtensorModule::get_network_max_stake(netuid);

        // Check that the default value is set correctly
        assert_eq!(default_max_stake, 500_000_000_000_000);

        // Set a new max stake value
        let new_max_stake: u64 = 1_000_000;
        SubtensorModule::set_network_max_stake(netuid, new_max_stake);

        // Check that the new value is retrieved correctly
        assert_eq!(
            SubtensorModule::get_network_max_stake(netuid),
            new_max_stake
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_set_network_max_stake --exact --nocapture
#[test]
fn test_set_network_max_stake() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let initial_max_stake = SubtensorModule::get_network_max_stake(netuid);

        // Set a new max stake value
        let new_max_stake: u64 = 500_000;
        SubtensorModule::set_network_max_stake(netuid, new_max_stake);

        // Check that the new value is set correctly
        assert_eq!(
            SubtensorModule::get_network_max_stake(netuid),
            new_max_stake
        );
        assert_ne!(
            SubtensorModule::get_network_max_stake(netuid),
            initial_max_stake
        );

        // Check that the event is emitted
        System::assert_last_event(Event::NetworkMaxStakeSet(netuid, new_max_stake).into());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_set_network_max_stake_multiple_networks --exact --nocapture
#[test]
fn test_set_network_max_stake_multiple_networks() {
    new_test_ext(1).execute_with(|| {
        let netuid1: u16 = 1;
        let netuid2: u16 = 2;

        // Set different max stake values for two networks
        let max_stake1: u64 = 1_000_000;
        let max_stake2: u64 = 2_000_000;
        SubtensorModule::set_network_max_stake(netuid1, max_stake1);
        SubtensorModule::set_network_max_stake(netuid2, max_stake2);

        // Check that the values are set correctly for each network
        assert_eq!(SubtensorModule::get_network_max_stake(netuid1), max_stake1);
        assert_eq!(SubtensorModule::get_network_max_stake(netuid2), max_stake2);
        assert_ne!(
            SubtensorModule::get_network_max_stake(netuid1),
            SubtensorModule::get_network_max_stake(netuid2)
        );
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_set_network_max_stake_update --exact --nocapture
#[test]
fn test_set_network_max_stake_update() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;

        // Set an initial max stake value
        let initial_max_stake: u64 = 1_000_000;
        SubtensorModule::set_network_max_stake(netuid, initial_max_stake);

        // Update the max stake value
        let updated_max_stake: u64 = 1_500_000;
        SubtensorModule::set_network_max_stake(netuid, updated_max_stake);

        // Check that the value is updated correctly
        assert_eq!(
            SubtensorModule::get_network_max_stake(netuid),
            updated_max_stake
        );
        assert_ne!(
            SubtensorModule::get_network_max_stake(netuid),
            initial_max_stake
        );

        // Check that the event is emitted for the update
        System::assert_last_event(Event::NetworkMaxStakeSet(netuid, updated_max_stake).into());
    });
}

// SKIP_WASM_BUILD=1 RUST_LOG=info cargo test --test children -- test_children_stake_values --exact --nocapture
#[test]
fn test_children_stake_values() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let child3 = U256::from(5);
        let netuid: u16 = 1;
        let proportion1: u64 = u64::MAX / 4;
        let proportion2: u64 = u64::MAX / 4;
        let proportion3: u64 = u64::MAX / 4;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        SubtensorModule::set_max_registrations_per_block(netuid, 4);
        SubtensorModule::set_target_registrations_per_interval(netuid, 4);
        register_ok_neuron(netuid, hotkey, coldkey, 0);
        register_ok_neuron(netuid, child1, coldkey, 0);
        register_ok_neuron(netuid, child2, coldkey, 0);
        register_ok_neuron(netuid, child3, coldkey, 0);
        SubtensorModule::increase_stake_on_coldkey_hotkey_account(
            &coldkey,
            &hotkey,
            100_000_000_000_000,
        );

        // Set multiple children with proportions.
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![
                (proportion1, child1),
                (proportion2, child2),
                (proportion3, child3)
            ]
        ));
        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey, netuid),
            25_000_000_069_852
        );
        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&child1, netuid),
            24_999_999_976_716
        );
        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&child2, netuid),
            24_999_999_976_716
        );
        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&child3, netuid),
            24_999_999_976_716
        );
        assert_eq!(
            SubtensorModule::get_stake_for_hotkey_on_subnet(&child3, netuid)
                + SubtensorModule::get_stake_for_hotkey_on_subnet(&child2, netuid)
                + SubtensorModule::get_stake_for_hotkey_on_subnet(&child1, netuid)
                + SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey, netuid),
            100_000_000_000_000
        );
    });
}

/// Tests that setting children with a total proportion exceeding u64::MAX fails.
#[test]
fn test_set_children_proportion_overflow() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Attempt to set children with total proportion exceeding u64::MAX
        let children = vec![(u64::MAX, U256::from(3)), (1, U256::from(4))];

        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                children
            ),
            Error::<Test>::ProportionOverflow
        );
    });
}

/// Tests that setting duplicate children fails.
#[test]
fn test_set_duplicate_children() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child = U256::from(3);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Attempt to set duplicate children
        let children = vec![(1000, child), (2000, child)];

        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                children
            ),
            Error::<Test>::DuplicateChild
        );
    });
}

/// Tests the correct distribution of stake among children.
#[test]
fn test_stake_distribution_among_children() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let child1 = U256::from(3);
        let child2 = U256::from(4);
        let netuid: u16 = 1;

        // Add network and register neurons
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);
        register_ok_neuron(netuid, child1, coldkey, 0);
        register_ok_neuron(netuid, child2, coldkey, 0);

        // Set initial stake
        let initial_stake: u64 = 1_000_000_000_000;
        SubtensorModule::increase_stake_on_coldkey_hotkey_account(&coldkey, &hotkey, initial_stake);

        // Set children with different proportions
        let children = vec![(u64::MAX / 2, child1), (u64::MAX / 4, child2)];

        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            children
        ));

        // Check stake distribution
        let hotkey_stake = SubtensorModule::get_stake_for_hotkey_on_subnet(&hotkey, netuid);
        let child1_stake = SubtensorModule::get_stake_for_hotkey_on_subnet(&child1, netuid);
        let child2_stake = SubtensorModule::get_stake_for_hotkey_on_subnet(&child2, netuid);

        assert!(hotkey_stake < initial_stake, "Hotkey stake should decrease");
        assert!(
            child1_stake > child2_stake,
            "Child1 should have more stake than Child2"
        );
        assert_eq!(
            hotkey_stake + child1_stake + child2_stake,
            initial_stake,
            "Total stake should remain constant"
        );
    });
}

/// Tests that setting more than the maximum allowed number of children (5) fails.
#[test]
fn test_set_children_exceeds_maximum() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey = U256::from(2);
        let netuid: u16 = 1;

        // Add network and register hotkey
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Attempt to set 6 children (exceeding the maximum of 5)
        let children: Vec<(u64, U256)> = (3..9).map(|i| (1000, U256::from(i))).collect();

        assert_err!(
            SubtensorModule::do_set_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                netuid,
                children
            ),
            Error::<Test>::TooManyChildren
        );
    });
}

#[test]
fn test_set_child_with_multiple_parents() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey1 = U256::from(2);
        let hotkey2 = U256::from(3);
        let child = U256::from(4);
        let netuid: u16 = 1;
        let proportion1: u64 = 1000;
        let proportion2: u64 = 2000;

        // Add network and register hotkeys and child
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey1, coldkey, 0);
        register_ok_neuron(netuid, hotkey2, coldkey, 0);
        register_ok_neuron(netuid, child, coldkey, 0);

        // Set child for first parent
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey1,
            netuid,
            vec![(proportion1, child)]
        ));

        // Set child for second parent
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey2,
            netuid,
            vec![(proportion2, child)]
        ));

        // Check that child's parent list contains both parents
        let parents = SubtensorModule::get_parents(&child, netuid);
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&(proportion1, hotkey1)));
        assert!(parents.contains(&(proportion2, hotkey2)));
    });
}

#[test]
fn test_remove_child_with_multiple_parents() {
    new_test_ext(1).execute_with(|| {
        let coldkey = U256::from(1);
        let hotkey1 = U256::from(2);
        let hotkey2 = U256::from(3);
        let child = U256::from(4);
        let netuid: u16 = 1;
        let proportion1: u64 = 1000;
        let proportion2: u64 = 2000;

        // Add network and register hotkeys and child
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey1, coldkey, 0);
        register_ok_neuron(netuid, hotkey2, coldkey, 0);
        register_ok_neuron(netuid, child, coldkey, 0);

        // Set child for both parents
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey1,
            netuid,
            vec![(proportion1, child)]
        ));
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey2,
            netuid,
            vec![(proportion2, child)]
        ));

        // Remove child from first parent
        assert_ok!(SubtensorModule::do_set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey1,
            netuid,
            vec![]
        ));

        // Check that child's parent list only contains the second parent
        let parents = SubtensorModule::get_parents(&child, netuid);
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0], (proportion2, hotkey2));

        // Check that first parent's child list is empty
        let children1 = SubtensorModule::get_children(&hotkey1, netuid);
        assert!(children1.is_empty());

        // Check that second parent's child list still contains the child
        let children2 = SubtensorModule::get_children(&hotkey2, netuid);
        assert_eq!(children2.len(), 1);
        assert_eq!(children2[0], (proportion2, child));
    });
}

#[test]
fn test_revoke_children_success() {
    new_test_ext(1).execute_with(|| {
        let netuid = 1;
        let hotkey = U256::from(1);
        let coldkey = U256::from(1);
        let child1 = U256::from(2);
        let child2 = U256::from(3);

        // Add network
        add_network(netuid, 13, 0);

        // Register neurons
        register_ok_neuron(netuid, hotkey, coldkey, 0);
        register_ok_neuron(netuid, child1, coldkey, 0);
        register_ok_neuron(netuid, child2, coldkey, 0);

        // Set up the initial state
        assert_ok!(SubtensorModule::set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(100, child1), (200, child2)]
        ));

        // Revoke children
        assert_ok!(SubtensorModule::revoke_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid
        ));

        // Check that children are removed
        assert_eq!(SubtensorModule::get_children(&hotkey, netuid), vec![]);

        // Check that parent relationships are removed
        assert_eq!(SubtensorModule::get_parents(&child1, netuid), vec![]);
        assert_eq!(SubtensorModule::get_parents(&child2, netuid), vec![]);
        // Check that the event was emitted
        System::assert_last_event(RuntimeEvent::SubtensorModule(Event::ChildrenRevoked {
            hotkey,
            netuid,
        }));
    });
}

#[test]
fn test_revoke_children_root_network() {
    new_test_ext(1).execute_with(|| {
        let root_netuid = SubtensorModule::get_root_netuid();
        let hotkey = U256::from(1);
        let coldkey = U256::from(1);

        assert_noop!(
            SubtensorModule::revoke_children(RuntimeOrigin::signed(coldkey), hotkey, root_netuid),
            Error::<Test>::RegistrationNotPermittedOnRootSubnet
        );
    });
}

#[test]
fn test_revoke_children_non_existent_network() {
    new_test_ext(1).execute_with(|| {
        let non_existent_netuid = 999;
        let hotkey = U256::from(1);
        let coldkey = U256::from(1);

        assert_noop!(
            SubtensorModule::revoke_children(
                RuntimeOrigin::signed(coldkey),
                hotkey,
                non_existent_netuid
            ),
            Error::<Test>::SubNetworkDoesNotExist
        );
    });
}

#[test]
fn test_revoke_children_non_associated_coldkey() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let hotkey = U256::from(1);
        let coldkey = U256::from(1);
        let non_associated_coldkey = U256::from(2);

        // Add network
        add_network(netuid, 13, 0);

        // Register neuron with hotkey
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Set up the initial state
        assert_ok!(SubtensorModule::set_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid,
            vec![(100, U256::from(2))]
        ));

        assert_noop!(
            SubtensorModule::revoke_children(
                RuntimeOrigin::signed(non_associated_coldkey),
                hotkey,
                netuid
            ),
            Error::<Test>::NonAssociatedColdKey
        );
    });
}

#[test]
fn test_revoke_children_unsigned_origin() {
    new_test_ext(1).execute_with(|| {
        let netuid = 1;
        let hotkey = U256::from(1);

        assert_noop!(
            SubtensorModule::revoke_children(RuntimeOrigin::none(), hotkey, netuid),
            BadOrigin
        );
    });
}

#[test]
fn test_revoke_children_no_existing_children() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let hotkey = U256::from(1);
        let coldkey = U256::from(1);

        // Add network and register neuron
        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey, coldkey, 0);

        // Revoke children when there are no existing children
        assert_ok!(SubtensorModule::revoke_children(
            RuntimeOrigin::signed(coldkey),
            hotkey,
            netuid
        ));

        // Check that the event was still emitted
        System::assert_last_event(RuntimeEvent::SubtensorModule(Event::ChildrenRevoked {
            hotkey,
            netuid,
        }));

        // Verify that the children list is empty
        assert_eq!(SubtensorModule::get_children(&hotkey, netuid), vec![]);
    });
}

#[test]
fn test_revoke_children_multiple_parents() {
    new_test_ext(1).execute_with(|| {
        let netuid: u16 = 1;
        let hotkey1 = U256::from(1);
        let hotkey2 = U256::from(2);
        let coldkey1 = U256::from(1);
        let coldkey2 = U256::from(2);
        let child = U256::from(3);

        add_network(netuid, 13, 0);
        register_ok_neuron(netuid, hotkey1, coldkey1, 0);
        register_ok_neuron(netuid, hotkey2, coldkey2, 0);
        register_ok_neuron(netuid, child, U256::from(4), 0);

        // Set up the initial state with two parents for the child
        assert_ok!(SubtensorModule::set_children(
            RuntimeOrigin::signed(coldkey1),
            hotkey1,
            netuid,
            vec![(100, child)]
        ));
        assert_ok!(SubtensorModule::set_children(
            RuntimeOrigin::signed(coldkey2),
            hotkey2,
            netuid,
            vec![(200, child)]
        ));

        // Revoke children for hotkey1
        assert_ok!(SubtensorModule::revoke_children(
            RuntimeOrigin::signed(coldkey1),
            hotkey1,
            netuid
        ));

        // Check that child's parent list only contains hotkey2
        assert_eq!(
            SubtensorModule::get_parents(&child, netuid),
            vec![(200, hotkey2)]
        );

        // Check that hotkey1's children list is empty
        assert_eq!(SubtensorModule::get_children(&hotkey1, netuid), vec![]);

        // Check that hotkey2's children list is unchanged
        assert_eq!(
            SubtensorModule::get_children(&hotkey2, netuid),
            vec![(200, child)]
        );
    });
}