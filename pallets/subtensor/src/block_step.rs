use super::*;
use frame_support::inherent::Vec;
use frame_support::storage::IterableStorageDoubleMap;
use frame_support::storage::IterableStorageMap;
use substrate_fixed::types::I110F18;
use substrate_fixed::types::I64F64;
use substrate_fixed::types::I96F32;

impl<T: Config> Pallet<T> {
    /// Executes the necessary operations for each block.
    pub fn block_step() -> Result<(), &'static str> {
        let block_number: u64 = Self::get_current_block_as_u64();
        log::debug!("block_step for block: {:?} ", block_number);
        // --- 1. Adjust difficulties.
        Self::adjust_registration_terms_for_networks();
        // --- 2. Mint and distribute TAO.
        Self::run_coinbase(block_number);
        // Return ok.
        Ok(())
    }

    // Helper function which returns the number of blocks remaining before we will run the epoch on this
    // network. Networks run their epoch when (block_number + netuid + 1 ) % (tempo + 1) = 0
    //
    pub fn blocks_until_next_epoch(netuid: u16, tempo: u16, block_number: u64) -> u64 {
        // tempo | netuid | # first epoch block
        //   1        0               0
        //   1        1               1
        //   2        0               1
        //   2        1               0
        //   100      0              99
        //   100      1              98
        // Special case: tempo = 0, the network never runs.
        if tempo == 0 {
            return 1000;
        }
        return tempo as u64 - (block_number + netuid as u64 + 1) % (tempo as u64 + 1);
    }

    pub fn run_coinbase( block_number:u64 ) {

        let block_emission: u64 = Self::get_block_emission();
        let netuids: Vec<u16> = Self::get_all_subnet_netuids();
        let mut prices = Vec::new();
        let mut total_price:I64F64 = I64F64::from_num(0.0);

        // Compute the uniswap price for each netuid
        for netuid in netuids.iter() {
            let tao_reserve:u64 = DynamicTAOReserve::<T>::get(netuid);
            let sub_reserve:u64 = DynamicSubReserve::<T>::get(netuid);
            if sub_reserve > 0 { // Avoid division by zero
                let price = I64F64::from_num(tao_reserve)/ I64F64::from_num(sub_reserve);
                prices.push((netuid, price));
                total_price += price;
            }
        }

        // Normalize the prices and distribute TAO
        for (netuid, price) in prices.iter() {
            let normalized_price: I64F64 = price / I64F64::from_num(total_price);
            let new_tao_emission: u64 = (normalized_price * I64F64::from_num(block_emission)).to_num::<u64>();
            let new_dynamic_emission: u64 = Self::get_block_emission();
            let new_dynamic_reserve_emission: u64 = Self::get_block_emission();

            let current_tao_reserve: u64 = DynamicTAOReserve::<T>::get(netuid);
            let current_dynamic_reserve: u64 = DynamicSubReserve::<T>::get(netuid);

            let new_tao_reserve: u64 = current_tao_reserve + new_tao_emission;
            let new_dynamic_reserve: u64 = current_dynamic_reserve + new_dynamic_emission;
            let new_dynamic_k: u64 = new_tao_reserve * current_dynamic_reserve;

            DynamicK::<T>::insert( netuid, new_dynamic_k );
            DynamicTAOReserve::<T>::insert( netuid, new_tao_reserve );
            PendingEmission::<T>::mutate( netuid, |emission| *emission += new_dynamic_reserve_emission );
            TotalIssuance::<T>::put(TotalIssuance::<T>::get().saturating_add( new_tao_emission ));
        }

        // Iterate over network and run epochs.
        for netuid in netuids.iter() {

            // Check to see if this network has reached tempo.
            let tempo: u16 = Self::get_tempo( *netuid );
            if Self::blocks_until_next_epoch( *netuid, tempo, block_number ) == 0 {

                // Get the emission to distribute for this subnet.
                let emission_to_drain: u64 = PendingEmission::<T>::get(netuid);
                PendingEmission::<T>::insert(netuid, 0);

                // Run the epoch mechanism and return emission tuples for hotkeys in the network.
                let emission_tuples: Vec<(T::AccountId, u64, u64)> = Self::epoch( *netuid, emission_to_drain );

                // --- Emit the tuples through the hotkeys.
                for (hotkey, server_amount, validator_amount) in emission_tuples.iter() {
                    Self::emit_inflation_through_hotkey_account(
                        &hotkey,
                        *netuid,
                        *server_amount,
                        *validator_amount,
                    );
                }

                // Update counters.
                Self::set_blocks_since_last_step(*netuid, 0);
                Self::set_last_mechanism_step_block(*netuid, block_number);
            } 
            else {
                Self::set_blocks_since_last_step(
                    *netuid,
                    Self::get_blocks_since_last_step(*netuid) + 1,
                );
                continue;
            }
        }
    }

    // Distributes token inflation through the hotkey based on emission. The call ensures that the inflation
    // is distributed onto the accounts in proportion of the stake delegated minus the take. This function
    // is called after an epoch to distribute the newly minted stake according to delegation.
    pub fn emit_inflation_through_hotkey_account(
        delegate: &T::AccountId,
        netuid: u16,
        server_emission: u64,
        validator_emission: u64,
    ) {
        // 1. Check if the hotkey is not a delegate and thus the emission is entirely owed to them.
        if !Self::hotkey_is_delegate( delegate ) {
            let total_delegate_emission: u64 = server_emission + validator_emission;
            Self::increase_stake_on_hotkey_account( 
                delegate, 
                netuid, 
                total_delegate_emission
            );
            return;
        }
        // 2. Else the key is a delegate, first compute the delegate take from the emission.
        let take_proportion: I64F64 = I64F64::from_num(Delegates::<T>::get( delegate )) / I64F64::from_num(u16::MAX);
        let delegate_take: I64F64 = take_proportion * I64F64::from_num( validator_emission );
        let delegate_take_u64: u64 = delegate_take.to_num::<u64>();
        let remaining_validator_emission: u64 = validator_emission - delegate_take_u64;
        let mut residual: u64 = remaining_validator_emission;

        // 3. For each nominator compute its proportion of stake weight and distribute the remaining emission to them.
        let global_stake_weight: I64F64 = Self::get_global_stake_weight_float();
        let delegate_local_stake: u64 = Self::get_total_stake_for_hotkey_and_subnet( delegate, netuid );
        let delegate_global_stake: u64 = Self::get_total_stake_for_hotkey( delegate );
        log::debug!("global_stake_weight: {:?}, delegate_local_stake: {:?}, delegate_global_stake: {:?}", global_stake_weight, delegate_local_stake, delegate_global_stake);

        if delegate_local_stake + delegate_global_stake != 0 {
            for (nominator_i, _) in <Stake<T> as IterableStorageDoubleMap<T::AccountId, T::AccountId, u64>>::iter_prefix( delegate ) {

                // 3.a Compute the stake weight percentage for the nominatore weight.
                let nominator_local_stake: u64 = Self::get_subnet_stake_for_coldkey_and_hotkey( &nominator_i, delegate, netuid );
                let nominator_local_emission_i: I64F64 = if delegate_local_stake == 0 {
                    I64F64::from_num(0)
                } else {
                    let nominator_local_percentage: I64F64 = I64F64::from_num( nominator_local_stake ) / I64F64::from_num( delegate_local_stake );
                    nominator_local_percentage * I64F64::from_num(remaining_validator_emission) * ( I64F64::from_num(1.0) - global_stake_weight )
                };
                log::debug!("nominator_local_emission_i: {:?}", nominator_local_emission_i);

                let nominator_global_stake: u64 = Self::get_subnet_stake_for_coldkey_and_hotkey( delegate, &nominator_i, 0); // Get Root stake.
                let nominator_global_emission_i: I64F64 = if delegate_global_stake == 0 {
                    I64F64::from_num(0)
                } else {
                    let nominator_global_percentage: I64F64 = I64F64::from_num( nominator_global_stake ) / I64F64::from_num( delegate_global_stake );
                    nominator_global_percentage * I64F64::from_num( remaining_validator_emission ) * global_stake_weight
                };
                log::debug!("nominator_global_emission_i: {:?}", nominator_global_emission_i);
                let nominator_emission_u64: u64 = (nominator_global_emission_i + nominator_local_emission_i).to_num::<u64>();

                // 3.b Increase the stake of the nominator.
                log::debug!("nominator: {:?}, global_emission: {:?}, local_emission: {:?}", nominator_i, nominator_global_emission_i, nominator_local_emission_i);
                residual -= nominator_emission_u64;
                Self::increase_stake_on_coldkey_hotkey_account(
                    &nominator_i,
                    delegate,
                    netuid,
                    nominator_emission_u64,
                );
            }
        }

        // --- 5. Last increase final account balance of delegate after 4, since 5 will change the stake proportion of
        // the delegate and effect calculation in 4.
        let total_delegate_emission: u64 = delegate_take_u64 + server_emission + residual;
        log::debug!("total_delegate_emission: {:?}", delegate_take_u64 + server_emission);
        Self::increase_stake_on_hotkey_account(
            delegate,
            netuid,
            total_delegate_emission,
        );
    }

    // Returns emission awarded to a hotkey as a function of its proportion of the total stake.
    //
    pub fn calculate_stake_proportional_emission(
        stake: u64,
        total_stake: u64,
        emission: u64,
    ) -> u64 {
        if total_stake == 0 {
            return 0;
        };
        let stake_proportion: I64F64 = I64F64::from_num(stake) / I64F64::from_num(total_stake);
        let proportional_emission: I64F64 = I64F64::from_num(emission) * stake_proportion;
        return proportional_emission.to_num::<u64>();
    }

    // Returns the delegated stake 'take' assigned to this key. (If exists, otherwise 0)
    //
    pub fn calculate_delegate_proportional_take(hotkey: &T::AccountId, emission: u64) -> u64 {
        if Self::hotkey_is_delegate(hotkey) {
            let take_proportion: I64F64 =
                I64F64::from_num(Delegates::<T>::get(hotkey)) / I64F64::from_num(u16::MAX);
            let take_emission: I64F64 = take_proportion * I64F64::from_num(emission);
            return take_emission.to_num::<u64>();
        } else {
            return 0;
        }
    }

    // Adjusts the network difficulties/burns of every active network. Resetting state parameters.
    //
    pub fn adjust_registration_terms_for_networks() {
        log::debug!("adjust_registration_terms_for_networks");

        // --- 1. Iterate through each network.
        for (netuid, _) in <NetworksAdded<T> as IterableStorageMap<u16, bool>>::iter() {
            // --- 2. Pull counters for network difficulty.
            let last_adjustment_block: u64 = Self::get_last_adjustment_block(netuid);
            let adjustment_interval: u16 = Self::get_adjustment_interval(netuid);
            let current_block: u64 = Self::get_current_block_as_u64();
            log::debug!("netuid: {:?} last_adjustment_block: {:?} adjustment_interval: {:?} current_block: {:?}", 
                netuid,
                last_adjustment_block,
                adjustment_interval,
                current_block
            );

            // --- 3. Check if we are at the adjustment interval for this network.
            // If so, we need to adjust the registration difficulty based on target and actual registrations.
            if (current_block - last_adjustment_block) >= adjustment_interval as u64 {
                log::debug!("interval reached.");

                // --- 4. Get the current counters for this network w.r.t burn and difficulty values.
                let current_burn: u64 = Self::get_burn_as_u64(netuid);
                let current_difficulty: u64 = Self::get_difficulty_as_u64(netuid);
                let registrations_this_interval: u16 =
                    Self::get_registrations_this_interval(netuid);
                let pow_registrations_this_interval: u16 =
                    Self::get_pow_registrations_this_interval(netuid);
                let burn_registrations_this_interval: u16 =
                    Self::get_burn_registrations_this_interval(netuid);
                let target_registrations_this_interval: u16 =
                    Self::get_target_registrations_per_interval(netuid);
                // --- 5. Adjust burn + pow
                // There are six cases to consider. A, B, C, D, E, F
                if registrations_this_interval > target_registrations_this_interval {
                    if pow_registrations_this_interval > burn_registrations_this_interval {
                        // A. There are too many registrations this interval and most of them are pow registrations
                        // this triggers an increase in the pow difficulty.
                        // pow_difficulty ++
                        Self::set_difficulty(
                            netuid,
                            Self::adjust_difficulty(
                                netuid,
                                current_difficulty,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                    } else if pow_registrations_this_interval < burn_registrations_this_interval {
                        // B. There are too many registrations this interval and most of them are burn registrations
                        // this triggers an increase in the burn cost.
                        // burn_cost ++
                        Self::set_burn(
                            netuid,
                            Self::adjust_burn(
                                netuid,
                                current_burn,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                    } else {
                        // F. There are too many registrations this interval and the pow and burn registrations are equal
                        // this triggers an increase in the burn cost and pow difficulty
                        // burn_cost ++
                        Self::set_burn(
                            netuid,
                            Self::adjust_burn(
                                netuid,
                                current_burn,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                        // pow_difficulty ++
                        Self::set_difficulty(
                            netuid,
                            Self::adjust_difficulty(
                                netuid,
                                current_difficulty,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                    }
                } else {
                    // Not enough registrations this interval.
                    if pow_registrations_this_interval > burn_registrations_this_interval {
                        // C. There are not enough registrations this interval and most of them are pow registrations
                        // this triggers a decrease in the burn cost
                        // burn_cost --
                        Self::set_burn(
                            netuid,
                            Self::adjust_burn(
                                netuid,
                                current_burn,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                    } else if pow_registrations_this_interval < burn_registrations_this_interval {
                        // D. There are not enough registrations this interval and most of them are burn registrations
                        // this triggers a decrease in the pow difficulty
                        // pow_difficulty --
                        Self::set_difficulty(
                            netuid,
                            Self::adjust_difficulty(
                                netuid,
                                current_difficulty,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                    } else {
                        // E. There are not enough registrations this interval and the pow and burn registrations are equal
                        // this triggers a decrease in the burn cost and pow difficulty
                        // burn_cost --
                        Self::set_burn(
                            netuid,
                            Self::adjust_burn(
                                netuid,
                                current_burn,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                        // pow_difficulty --
                        Self::set_difficulty(
                            netuid,
                            Self::adjust_difficulty(
                                netuid,
                                current_difficulty,
                                registrations_this_interval,
                                target_registrations_this_interval,
                            ),
                        );
                    }
                }

                // --- 6. Drain all counters for this network for this interval.
                Self::set_last_adjustment_block(netuid, current_block);
                Self::set_registrations_this_interval(netuid, 0);
                Self::set_pow_registrations_this_interval(netuid, 0);
                Self::set_burn_registrations_this_interval(netuid, 0);
            } else {
                log::debug!("interval not reached.");
            }

            // --- 7. Drain block registrations for each network. Needed for registration rate limits.
            Self::set_registrations_this_block(netuid, 0);
        }
    }

    // Performs the difficulty adjustment by multiplying the current difficulty by the ratio ( reg_actual + reg_target / reg_target * reg_target )
    // We use I110F18 to avoid any overflows on u64. Also min_difficulty and max_difficulty bound the range.
    //
    pub fn adjust_difficulty(
        netuid: u16,
        current_difficulty: u64,
        registrations_this_interval: u16,
        target_registrations_per_interval: u16,
    ) -> u64 {
        let updated_difficulty: I110F18 = I110F18::from_num(current_difficulty)
            * I110F18::from_num(registrations_this_interval + target_registrations_per_interval)
            / I110F18::from_num(
                target_registrations_per_interval + target_registrations_per_interval,
            );
        let alpha: I110F18 =
            I110F18::from_num(Self::get_adjustment_alpha(netuid)) / I110F18::from_num(u64::MAX);
        let next_value: I110F18 = alpha * I110F18::from_num(current_difficulty)
            + (I110F18::from_num(1.0) - alpha) * updated_difficulty;
        if next_value >= I110F18::from_num(Self::get_max_difficulty(netuid)) {
            return Self::get_max_difficulty(netuid);
        } else if next_value <= I110F18::from_num(Self::get_min_difficulty(netuid)) {
            return Self::get_min_difficulty(netuid);
        } else {
            return next_value.to_num::<u64>();
        }
    }

    // Performs the burn adjustment by multiplying the current difficulty by the ratio ( reg_actual + reg_target / reg_target * reg_target )
    // We use I110F18 to avoid any overflows on u64. Also min_burn and max_burn bound the range.
    //
    pub fn adjust_burn(
        netuid: u16,
        current_burn: u64,
        registrations_this_interval: u16,
        target_registrations_per_interval: u16,
    ) -> u64 {
        let updated_burn: I110F18 = I110F18::from_num(current_burn)
            * I110F18::from_num(registrations_this_interval + target_registrations_per_interval)
            / I110F18::from_num(
                target_registrations_per_interval + target_registrations_per_interval,
            );
        let alpha: I110F18 =
            I110F18::from_num(Self::get_adjustment_alpha(netuid)) / I110F18::from_num(u64::MAX);
        let next_value: I110F18 = alpha * I110F18::from_num(current_burn)
            + (I110F18::from_num(1.0) - alpha) * updated_burn;
        if next_value >= I110F18::from_num(Self::get_max_burn_as_u64(netuid)) {
            return Self::get_max_burn_as_u64(netuid);
        } else if next_value <= I110F18::from_num(Self::get_min_burn_as_u64(netuid)) {
            return Self::get_min_burn_as_u64(netuid);
        } else {
            return next_value.to_num::<u64>();
        }
    }
}
