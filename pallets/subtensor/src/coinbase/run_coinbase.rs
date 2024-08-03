use super::*;
use substrate_fixed::types::I96F32;

impl<T: Config> Pallet<T> {
    

    pub fn run_coinbase() {
        // --- 0. Get current block.
        let current_block: u64 = Self::get_current_block_as_u64();
        log::debug!("Current block: {:?}", current_block);

        // --- 1. Get all netuids.
        let subnets: Vec<u16> = Self::get_all_subnet_netuids();
        log::debug!("All subnet netuids: {:?}", subnets);

        // --- 2. Get the current coinbase emission.
        let block_emission: I96F32 = I96F32::from_num(Self::get_block_emission().unwrap_or(0));
        log::debug!("Block emission: {:?}", block_emission);

        // --- 3. Total subnet TAO.
        let total_issuance: I96F32 = I96F32::from_num(Self::get_total_issuance());
        log::debug!("Total issuance: {:?}", total_issuance);

        // --- 4. Compute EmissionValues per subnet.
        // This loop calculates the emission for each subnet based on its mechanism and proportion of TAO.
        // For each subnet s in a mechanism m:
        // 1. Calculate subnet's proportion of mechanism TAO: P_s = T_s / T_m
        // 2. Calculate subnet's TAO emission: E_s = P_s * E_m
        // 3. Convert TAO emission to alpha emission: E_α = tao_to_alpha(E_s)
        // 4. Update total issuance: I_new = I_old + E_s
        // 5. Update subnet TAO: T_s_new = T_s_old + E_s
        // 6. Update subnet alpha: A_s_new = A_s_old + E_α
        // 7. Accumulate pending emission: P_e_new = P_e_old + E_α
        for netuid in subnets.clone().iter() {
            // Do not emit into root network.
            if *netuid == 0 { continue }
            // 1. Get subnet mechanism ID
            let mechid: u16 = SubnetMechanism::<T>::get(*netuid);
            // 4. Get subnet TAO (T_s)
            let subnet_tao: I96F32 = I96F32::from_num(SubnetTAO::<T>::get(*netuid));
            // 5. Calculate subnet's proportion of mechanism TAO: P_s = T_s / T_m
            let subnet_proportion: I96F32 = subnet_tao
                .checked_div(total_issuance)
                .unwrap_or(I96F32::from_num(0));
            // 6. Calculate subnet's TAO emission: E_s = P_s * E_m
            let tao_emission: u64 = subnet_proportion
                .checked_mul(block_emission)
                .unwrap_or(I96F32::from_num(0))
                .to_num::<u64>();
            // 7. Store the block emission for this subnet
            EmissionValues::<T>::insert(*netuid, tao_emission);
            // Add the TAO into the subnet immediatetly.
            SubnetTAO::<T>::mutate(*netuid, |total| *total = total.saturating_add(tao_emission));
            // Increase total stake here.
            TotalStake::<T>::mutate(|total| *total = total.saturating_add(tao_emission));
            // Increase total issuance.
            TotalIssuance::<T>::mutate(|total| *total = total.saturating_add(tao_emission));
            // Switch on dynamic or Stable.
            if mechid == 1 {
                // Dynamic.
                // Add the SubnetAlpha directly into the pool immediately.
                SubnetAlphaIn::<T>::mutate(*netuid, |total| {
                    *total = total.saturating_add(block_emission.to_num::<u64>())
                });
                // Set the pending emission directly as alpha always block emission total
                PendingEmission::<T>::mutate(netuid, |total| {
                    *total = total.saturating_add(block_emission.to_num::<u64>())
                });
            } else {
                // Set the pending emission as tao emission.
                PendingEmission::<T>::mutate(netuid, |total| {
                    *total = total.saturating_add(tao_emission)
                });
            }
        }
        log::debug!(
            "Emission per subnet: {:?}",
            EmissionValues::<T>::iter().collect::<Vec<_>>()
        );
        log::debug!(
            "Pending Emission per subnet: {:?}",
            PendingEmission::<T>::iter().collect::<Vec<_>>()
        );

        // --- 5. Drain the accumulated subnet emissions, pass them through the epoch().
        // Before accumulating on the hotkeys the function redistributes the emission towards hotkey parents.
        // subnet_emission --> epoch() --> hotkey_emission --> (hotkey + parent hotkeys)
        let mut hotkey_emission_limit: u64 = 0;
        let mut hotkey_emission_tuples: Vec<(T::AccountId, u16, u64)> = vec![];
        for netuid in subnets.clone().iter() {
            // --- 5.1 Check to see if the subnet should run its epoch.
            if Self::should_run_epoch(*netuid, current_block) {
                // --- 5.2 Drain the subnet emission.
                let subnet_emission: u64 = PendingEmission::<T>::get(*netuid);
                PendingEmission::<T>::insert(*netuid, 0);
                log::debug!(
                    "Drained subnet emission for netuid {:?}: {:?}",
                    *netuid,
                    subnet_emission
                );

                // --- 5.3 Set last step counter.
                Self::set_blocks_since_last_step(*netuid, 0);
                Self::set_last_mechanism_step_block(*netuid, current_block);

                // Decrement the emission by the owner cut.
                let owner_cut: u64 = I96F32::from_num(subnet_emission).saturating_mul(I96F32::from_num(Self::get_subnet_owner_cut())).saturating_div(I96F32::from_num(u16::MAX)).to_num::<u64>();
                Self::distribute_owner_cut(*netuid, owner_cut);
                let remaining_emission: u64 = subnet_emission.saturating_sub(owner_cut);

                // 5.3 Pass emission through epoch() --> hotkey emission.
                let hotkey_emission: Vec<(T::AccountId, u64, u64)> =
                    Self::epoch(*netuid, remaining_emission);
                log::debug!(
                    "Hotkey emission results for netuid {:?}: {:?}",
                    *netuid,
                    hotkey_emission
                );

                // 5.4 Accumulate the tuples on hotkeys:
                for (hotkey, mining_emission, validator_emission) in hotkey_emission {
                    // Distribute the emission on the hotkey and parent hotkeys appending new vectors to hotkey_emission_tuples.
                    Self::source_hotkey_emission(
                        &hotkey,
                        *netuid,
                        validator_emission, // Amount received from validating
                        mining_emission,    // Amount recieved from mining.
                        &mut hotkey_emission_tuples,
                    );
                    hotkey_emission_limit = hotkey_emission_limit
                        .saturating_add(validator_emission.saturating_add(mining_emission));
                    log::debug!("Accumulated emissions on hotkey {:?} for netuid {:?}: mining {:?}, validator {:?}", hotkey, *netuid, mining_emission, validator_emission);
                }
            } else {
                log::debug!("Tempo not reached for subnet: {:?}", *netuid);
            }
        }

        // Finally apply the emission tuples;
        log::debug!("Hotkey Emission tuples: {:?}", hotkey_emission_tuples);
        let total_hotkey_emitted: u64 = hotkey_emission_tuples
            .iter()
            .map(|(_, _, amount)| amount)
            .sum();
        assert!(
            total_hotkey_emitted <= hotkey_emission_limit,
            "total_hotkey_emitted: ({}) > hotkey_emission_limit: ({})",
            total_hotkey_emitted,
            hotkey_emission_limit
        );
        Self::accumulate_hotkey_emission(&mut hotkey_emission_tuples);

        // --- 6. Drain the accumulated hotkey emissions through to the nominators.
        // The hotkey takes a proportion of the emission, the remainder is drained through to the nominators.
        // We keep track of the last stake increase event for accounting purposes.
        // hotkeys --> nominators.
        let mut nominator_emission_limit: u64 = 0;
        let mut nominator_emission: Vec<(T::AccountId, T::AccountId, u16, u64)> = vec![];
        let emission_tempo: u64 = Self::get_hotkey_emission_tempo();
        for (hotkey, netuid_i, hotkey_emission) in PendingdHotkeyEmissionOnNetuid::<T>::iter() {
            if Self::should_drain_hotkey(&hotkey, current_block, emission_tempo) {
                log::debug!(
                    "Draining hotkey {:?} on netuid {:?} on block {:?}: {:?}",
                    hotkey,
                    netuid_i,
                    current_block,
                    hotkey_emission
                );
                // Remove the hotkey emission from the pending emissions.
                PendingdHotkeyEmissionOnNetuid::<T>::remove(&hotkey, netuid_i);
                // Drain the hotkey emission.
                Self::source_nominator_emission(
                    &hotkey,
                    netuid_i,
                    hotkey_emission,
                    current_block,
                    &mut nominator_emission,
                );
                nominator_emission_limit = nominator_emission_limit.saturating_add(hotkey_emission);
            }
        }
        // Update drain blocks.
        for (hotkey, _, _) in PendingdHotkeyEmissionOnNetuid::<T>::iter() {
            if Self::should_drain_hotkey(&hotkey, current_block, emission_tempo) {
                LastHotkeyEmissionDrain::<T>::insert(hotkey, current_block);
            }
        }
        // Finally apply the emission tuples;
        log::debug!("Emission tuples: {:?}", nominator_emission);
        let total_nominator_emitted: u64 = nominator_emission
            .iter()
            .map(|(_, _, _, amount)| amount)
            .sum();
        assert!(
            total_nominator_emitted <= nominator_emission_limit,
            "total_nominator_emitted: ({}) > emission_limit: ({})",
            total_nominator_emitted,
            nominator_emission_limit
        );
        Self::accumulate_nominator_emission(&mut nominator_emission);
    }

    /// Accumulates and distributes mining and validator emissions for a hotkey.
    ///
    /// This function performs the following key operations:
    /// 1. Calculates the hotkey's share of the validator emission based on its delegation status.
    /// 2. Computes the remaining validator emission to be distributed among parents.
    /// 3. Retrieves the list of parents and their stake contributions.
    /// 4. Calculates the total global and alpha (subnet-specific) stakes from parents.
    /// 5. Distributes the remaining validator emission to parents based on their contributions.
    /// 6. Allocates any undistributed validator emission, the hotkey's take, and the mining emission to the hotkey itself.
    ///
    /// # Arguments
    /// * `hotkey` - The account ID of the hotkey for which emissions are being calculated.
    /// * `netuid` - The unique identifier of the subnet to which the hotkey belongs.
    /// * `validating_emission` - The amount of validator emission allocated to the hotkey.
    /// * `mining_emission` - The amount of mining emission allocated to the hotkey.
    /// * `hotkey_emission_tuples` - A mutable reference to a vector that will be populated with emission distribution data.
    ///
    /// # Effects
    /// - Modifies `hotkey_emission_tuples` by adding entries for each parent receiving emission and the hotkey itself.
    /// - Does not directly modify any storage; all changes are recorded in `hotkey_emission_tuples` for later processing.
    ///
    /// # Note
    /// This function ensures fair distribution of emissions based on stake proportions and delegation agreements.
    /// It handles edge cases such as zero contributions and potential overflows using saturating arithmetic.
    pub fn source_hotkey_emission(
        hotkey: &T::AccountId,
        netuid: u16,
        validating_emission: u64,
        mining_emission: u64,
        hotkey_emission_tuples: &mut Vec<(T::AccountId, u16, u64)>,
    ) {
        // Calculate the hotkey's share of the validator emission based on its delegation status
        let validating_emission: I96F32 = I96F32::from_num(validating_emission);
        let take_proportion: I96F32 = I96F32::from_num(Delegates::<T>::get(hotkey))
            .saturating_div(I96F32::from_num(u16::MAX));
        let hotkey_take: I96F32 = take_proportion.saturating_mul(validating_emission);

        // Initialize variables to track emission distribution
        let mut to_parents: u64 = 0;
        let parent_emission: I96F32 = validating_emission.saturating_sub(hotkey_take);

        // Retrieve the hotkey's inherited stakes (not used in this function, consider removing)
        let hotkey_inherited_alpha: I96F32 = I96F32::from_num(Self::get_inherited_alpha_for_hotkey_on_subnet(hotkey, netuid));
        let hotkey_inherited_global: I96F32 = I96F32::from_num(Self::get_inherited_global_for_hotkey_on_subnet(hotkey, netuid));  

        // Initialize variables to calculate total stakes from parents
        let mut total_global: I96F32 = I96F32::from_num(0);
        let mut total_alpha: I96F32 = I96F32::from_num(0);
        let mut contributions: Vec<(T::AccountId, I96F32, I96F32)> = Vec::new();

        // Calculate total global and alpha (subnet-specific) stakes from all parents
        for (proportion, parent) in Self::get_parents(hotkey, netuid) {
            // Convert the parent's stake proportion to a fractional value
            let parent_proportion: I96F32 = I96F32::from_num(proportion).saturating_div(I96F32::from_num(u64::MAX));
            
            // Get the parent's global and subnet-specific (alpha) stakes
            let parent_global: I96F32 = I96F32::from_num(Self::get_global_for_hotkey(&parent));
            let parent_alpha: I96F32 = I96F32::from_num(Self::get_stake_for_hotkey_on_subnet(&parent, netuid));
            
            // Calculate the parent's contribution to the hotkey's stakes
            let parent_alpha_contribution: I96F32 = parent_alpha.saturating_mul(parent_proportion);
            let parent_global_contribution: I96F32 = parent_global.saturating_mul(parent_proportion);
            
            // Add to the total stakes
            total_global = total_global.saturating_add(parent_global_contribution);
            total_alpha = total_alpha.saturating_add(parent_alpha_contribution);
            
            // Store the parent's contributions for later use
            contributions.push((parent.clone(), parent_alpha_contribution, parent_global_contribution));
        }

        // Get the weights for global and alpha stakes in emission distribution
        let global_weight: I96F32 = Self::get_global_weight();
        let alpha_weight: I96F32 = I96F32::from_num(1.0).saturating_sub(global_weight);

        // Distribute emission to parents based on their contributions
        for (parent, alpha_contribution, global_contribution) in contributions {
            // Calculate emission based on alpha (subnet-specific) stake
            let alpha_emission: I96F32 = alpha_weight
                .saturating_mul(parent_emission)
                .saturating_mul(alpha_contribution)
                .checked_div(total_alpha)
                .unwrap_or(I96F32::from_num(0.0));
            
            // Calculate emission based on global stake
            let global_emission: I96F32 = global_weight
                .saturating_mul(parent_emission)
                .saturating_mul(global_contribution)
                .checked_div(total_global)
                .unwrap_or(I96F32::from_num(0.0));
            
            // Sum up the total emission for this parent
            let total_emission: u64 = alpha_emission.saturating_add(global_emission).to_num::<u64>();
            
            // Add the parent's emission to the distribution list
            hotkey_emission_tuples.push((parent, netuid, total_emission));
            
            // Keep track of total emission distributed to parents
            to_parents += total_emission;
        }
        
        // Calculate the final emission for the hotkey itself
        let remainder: u64 = validating_emission.to_num::<u64>().saturating_sub(to_parents).saturating_sub(hotkey_take.to_num::<u64>());
        let hotkey_take_u64 = hotkey_take.to_num::<u64>();
        let final_hotkey_emission = hotkey_take_u64.saturating_add(remainder).saturating_add(mining_emission);
        
        // Add the hotkey's own emission to the distribution list
        hotkey_emission_tuples.push((
            hotkey.clone(),
            netuid,
            final_hotkey_emission,
        ));
    }

    /// Distributes emission to nominators and the hotkey owner based on their contributions and delegation status.
    ///
    /// This function performs the following steps:
    /// 1. Calculates the hotkey's share of the emission based on its delegation status.
    /// 2. Computes the remaining emission to be distributed among nominators.
    /// 3. Retrieves global and alpha scores for the hotkey.
    /// 4. Iterates over all nominators, calculating their individual contributions based on alpha and global scores.
    /// 5. Distributes the emission to nominators proportionally based on their contributions.
    /// 6. Allocates any remaining emission and the hotkey's take to the hotkey owner.
    ///
    /// # Arguments
    /// * `hotkey` - The AccountId of the hotkey.
    /// * `netuid` - The subnet ID.
    /// * `emission` - The total emission to be distributed.
    /// * `_block_number` - The current block number (unused in this function).
    /// * `emission_tuples` - A mutable reference to a vector that will be populated with emission distribution data.
    ///
    /// # Effects
    /// - Modifies `emission_tuples` by adding entries for each nominator receiving emission and the hotkey owner.
    /// - Does not directly modify any storage; all changes are recorded in `emission_tuples` for later processing.
    ///
    /// # Note
    /// This function ensures fair distribution of emissions based on stake proportions and delegation agreements.
    /// It handles edge cases such as zero contributions and potential overflows using saturating arithmetic.
    pub fn source_nominator_emission(
        hotkey: &T::AccountId,
        netuid: u16,
        emission: u64,
        _block_number: u64,
        emission_tuples: &mut Vec<(T::AccountId, T::AccountId, u16, u64)>,
    ) {
        // Calculate the hotkey's share of the emission based on its delegation status
        let emission: I96F32 = I96F32::from_num(emission);
        let take_proportion: I96F32 = I96F32::from_num(Delegates::<T>::get(hotkey)).saturating_div(I96F32::from_num(u16::MAX));
        let hotkey_take: I96F32 = take_proportion.saturating_mul(emission);

        // Initialize variables to track emission distribution
        let mut to_nominators: u64 = 0;
        let nominator_emission: I96F32 = emission.saturating_sub(hotkey_take);
        let hotkey_global: I96F32 = I96F32::from_num(Self::get_global_for_hotkey(hotkey));
        let hotkey_alpha: I96F32 = I96F32::from_num(Self::get_stake_for_hotkey_on_subnet(hotkey, netuid));

        // Prepare to calculate contributions from nominators
        let mut total_global: I96F32 = I96F32::from_num(0);
        let mut total_alpha: I96F32 = I96F32::from_num(0);
        let mut contributions: Vec<(T::AccountId, I96F32, I96F32)> = Vec::new();

        // Calculate total global and alpha scores for all nominators
        for (nominator, _) in Stake::<T>::iter_prefix(hotkey) {
            let alpha_contribution: I96F32 = I96F32::from_num(Alpha::<T>::get((&hotkey, nominator.clone(), netuid)));
            let global_contribution: I96F32 = I96F32::from_num(Self::get_global_for_hotkey_and_coldkey(hotkey, &nominator));
            total_global += global_contribution;
            total_alpha += alpha_contribution;
            contributions.push((nominator.clone(), alpha_contribution, global_contribution));
        }

        // Get the weights for global and alpha scores
        let global_weight: I96F32 = Self::get_global_weight();
        let alpha_weight: I96F32 = I96F32::from_num(1.0).saturating_sub(global_weight);

        // Distribute emission to nominators based on their contributions
        if total_alpha > I96F32::from_num(0) || total_global > I96F32::from_num(0) {
            for (nominator, alpha_contribution, global_contribution) in contributions {
                // Calculate emission for this nominator based on alpha and global scores
                let alpha_emission: I96F32 = nominator_emission.saturating_mul(alpha_weight).saturating_mul(alpha_contribution).checked_div(total_alpha).unwrap_or(I96F32::from_num(0));
                let global_emission: I96F32 = nominator_emission.saturating_mul(global_weight).saturating_mul(global_contribution).checked_div(total_global).unwrap_or(I96F32::from_num(0));
                let total_emission: u64 = alpha_emission.saturating_add(global_emission).to_num::<u64>();
                if total_emission > 0 {
                    // Record the emission for this nominator
                    to_nominators += total_emission;
                    emission_tuples.push((
                        hotkey.clone(),
                        nominator.clone(),
                        netuid,
                        total_emission,
                    ));
                }
            }
        }

        // Calculate and distribute the remaining emission to the hotkey
        let hotkey_owner: T::AccountId = Owner::<T>::get(hotkey);
        let remainder: u64 = emission.to_num::<u64>().saturating_sub(hotkey_take.to_num::<u64>()).saturating_sub(to_nominators);
        let final_hotkey_emission:u64 = hotkey_take.to_num::<u64>() + remainder;
        emission_tuples.push((
            hotkey.clone(),
            hotkey_owner.clone(),
            netuid,
            final_hotkey_emission
        ));
    }
    
    /// Accumulates emissions for hotkeys across different subnets.
    ///
    /// This function takes a vector of tuples, each containing a hotkey account ID,
    /// a subnet ID (netuid), and an emission value. It updates the pending emission
    /// for each hotkey on the specified subnet by adding the given emission value.
    ///
    /// # Arguments
    ///
    /// * `hotkey_tuples` - A mutable reference to a vector of tuples, each containing:
    ///   - `T::AccountId`: The account ID of the hotkey
    ///   - `u16`: The subnet ID (netuid)
    ///   - `u64`: The emission value to be added
    pub fn accumulate_hotkey_emission(hotkey_tuples: &mut Vec<(T::AccountId, u16, u64)>) {
        for (hotkey, netuid, emission) in hotkey_tuples {
            PendingdHotkeyEmissionOnNetuid::<T>::mutate(hotkey, *netuid, |pending_emission| {
                *pending_emission = pending_emission.saturating_add(*emission);
            });
        }
    }

    /// Accumulates emissions for nominators and updates total hotkey alpha.
    ///
    /// This function processes a vector of tuples containing nominator emission data.
    /// It updates two storage items:
    /// 1. The total alpha for each hotkey on a specific subnet.
    /// 2. The individual alpha for each nominator (coldkey) associated with a hotkey on a subnet.
    ///
    /// # Arguments
    ///
    /// * `nominator_tuples` - A mutable reference to a vector of tuples, each containing:
    ///   - `T::AccountId`: The account ID of the hotkey
    ///   - `T::AccountId`: The account ID of the coldkey (nominator)
    ///   - `u16`: The subnet ID (netuid)
    ///   - `u64`: The emission value to be added
    pub fn accumulate_nominator_emission(
        nominator_tuples: &mut Vec<(T::AccountId, T::AccountId, u16, u64)>,
    ) {
        for (hotkey, coldkey, netuid, emission) in nominator_tuples {
            Self::emit_into_subnet(hotkey, coldkey, *netuid, *emission);
        }
    }

    ///////////////
    /// Helpers ///
    ///////////////
    /// Determines whether the hotkey emission should be drained based on the current block and index.
    ///
    /// # Arguments
    /// * `hotkey_i` - The hotkey identifier.
    /// * `index` - The index of the hotkey in the iterable storage.
    /// * `block` - The current block number.
    ///
    /// # Returns
    /// * `bool` - True if the hotkey emission should be drained, false otherwise.
    pub fn should_drain_hotkey(hotkey: &T::AccountId, block: u64, emit_tempo: u64) -> bool {
        let hotkey_idx: u64 = Self::hash_hotkey_to_u64(hotkey);
        block.rem_euclid(emit_tempo.saturating_add(1))
            == hotkey_idx.rem_euclid(emit_tempo.saturating_add(1))
    }

    /// Checks if the epoch should run for a given subnet based on the current block.
    ///
    /// # Arguments
    /// * `netuid` - The unique identifier of the subnet.
    ///
    /// # Returns
    /// * `bool` - True if the epoch should run, false otherwise.
    pub fn should_run_epoch(netuid: u16, current_block: u64) -> bool {
        Self::blocks_until_next_epoch(netuid, Self::get_tempo(netuid), current_block) == 0
    }

    /// Helper function which returns the number of blocks remaining before we will run the epoch on this
    /// network. Networks run their epoch when (block_number + netuid + 1 ) % (tempo + 1) = 0
    /// tempo | netuid | # first epoch block
    ///   1        0               0
    ///   1        1               1
    ///   2        0               1
    ///   2        1               0
    ///   100      0              99
    ///   100      1              98
    /// Special case: tempo = 0, the network never runs.
    ///
    pub fn blocks_until_next_epoch(netuid: u16, tempo: u16, block_number: u64) -> u64 {
        if tempo == 0 {
            return u64::MAX;
        }
        let netuid_plus_one = (netuid as u64).saturating_add(1);
        let tempo_plus_one = (tempo as u64).saturating_add(1);
        let adjusted_block = block_number.wrapping_add(netuid_plus_one);
        let remainder = adjusted_block % tempo_plus_one;
        (tempo as u64).saturating_sub(remainder)
    }

    /// Returns the emission value for the given subnet.
    ///
    /// This function retrieves the emission value for the given subnet.
    ///
    /// # Returns:
    /// * 'u64': The emission value for the given subnet.
    ///
    pub fn get_subnet_emission_value(netuid: u16) -> u64 {
        EmissionValues::<T>::get(netuid)
    }

    /// Returns the pending hotkey emission for a given hotkey on a specific subnet.
    ///
    /// This function retrieves the accumulated emission that is pending for a hotkey
    /// on a particular subnet. This emission is accumulated during the coinbase process
    /// and is typically distributed at the end of an epoch.
    ///
    /// # Arguments
    /// * `hotkey` - The account ID of the hotkey.
    /// * `netuid` - The unique identifier of the subnet.
    ///
    /// # Returns
    /// * `u64` - The pending emission amount for the hotkey on the specified subnet.
    pub fn get_pending_hotkey_emission_on_netuid(hotkey: &T::AccountId, netuid: u16) -> u64 {
        PendingdHotkeyEmissionOnNetuid::<T>::get(hotkey, netuid)
    }
}