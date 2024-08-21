use super::*;
extern crate alloc;
use codec::Compact;
use frame_support::pallet_prelude::{Decode, Encode};


#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct SubnetState<T: Config> {
    netuid: Compact<u16>,
    hotkeys: Vec<T::AccountId>,
    coldkeys: Vec<T::AccountId>,
    active: Vec<bool>,
    validator_permit: Vec<bool>,
    pruning_score: Vec<Compact<u16>>,
    last_update: Vec<Compact<u64>>,
    emission: Vec<Compact<u64>>,
    dividends: Vec<Compact<u16>>,
    incentives: Vec<Compact<u16>>,
    consensus: Vec<Compact<u16>>,
    trust: Vec<Compact<u16>>,
    rank: Vec<Compact<u16>>,
    block_at_registration: Vec<Compact<u64>>,
    local_stake: Vec<Compact<u64>>,
    global_stake: Vec<Compact<u64>>,
    stake_weight: Vec<Compact<u16>>,
    emission_history: Vec<Vec<Compact<u64>>>,
    // identities: Vec<ChainIdentityOf>,
    // tao_stake: Compact<u64>,
    // incentive: Compact<u16>,
    // consensus: Compact<u16>,
    // trust: Compact<u16>,
    // validator_trust: Compact<u16>,
    // dividends: Compact<u16>,
    // // has no weights or bonds
}

impl<T: Config> Pallet<T> {

    /// Retrieves the emission history for a list of hotkeys across all subnets.
    ///
    /// This function iterates over all subnets and collects the last emission value
    /// for each hotkey in the provided list. The result is a vector of vectors, where
    /// each inner vector contains the emission values for a specific subnet.
    ///
    /// # Arguments
    ///
    /// * `hotkeys` - A vector of hotkeys (account IDs) for which the emission history is to be retrieved.
    ///
    /// # Returns
    ///
    /// * `Vec<Vec<Compact<u64>>>` - A vector of vectors containing the emission history for each hotkey across all subnets.
    pub fn get_emissions_history(hotkeys: Vec<T::AccountId>) -> Vec<Vec<Compact<u64>>> {
        let mut result: Vec<Vec<Compact<u64>>> = vec![];
        for netuid in Self::get_all_subnet_netuids() {
            let mut hotkeys_emissions: Vec<Compact<u64>> = vec![];
            for hotkey in hotkeys.clone() {
                let last_emission: Compact<u64> = LastHotkeyEmissionOnNetuid::<T>::get(hotkey.clone(), netuid).into();
                hotkeys_emissions.push(last_emission.into());
            }
            result.push(hotkeys_emissions.clone());
        }
        result
    }

    /// Retrieves the state of a specific subnet.
    ///
    /// This function gathers various metrics and data points for a given subnet, identified by its `netuid`.
    /// It collects information such as hotkeys, coldkeys, block at registration, active status, validator permits,
    /// pruning scores, last updates, emissions, dividends, incentives, consensus, trust, rank, local stake, global stake,
    /// stake weight, and emission history.
    ///
    /// # Arguments
    ///
    /// * `netuid` - The unique identifier of the subnet for which the state is to be retrieved.
    ///
    /// # Returns
    ///
    /// * `Option<SubnetState<T>>` - An optional `SubnetState` struct containing the collected data for the subnet.
    ///   Returns `None` if the subnet does not exist.
    pub fn get_subnet_state(netuid: u16) -> Option<SubnetState<T>> {
        if !Self::if_subnet_exist(netuid) { return None; }
        let n: u16 = Self::get_subnetwork_n(netuid);
        let mut hotkeys: Vec<T::AccountId> = vec![];
        let mut coldkeys: Vec<T::AccountId> = vec![];
        let mut block_at_registration: Vec<Compact<u64>> = vec![];
        // let mut identities: Vec<ChainIdentityOf> = vec![];
        for uid in 0..n {
            let hotkey = Keys::<T>::get(netuid, uid);
            let coldkey = Owner::<T>::get( hotkey.clone() );
            hotkeys.push( hotkey );
            coldkeys.push( coldkey );
            block_at_registration.push( BlockAtRegistration::<T>::get( netuid, uid ).into() );
            // identities.push( Identities::<T>::get( coldkey.clone() ) );
        }
        let active: Vec<bool> = Active::<T>::get( netuid );
        let validator_permit: Vec<bool> = ValidatorPermit::<T>::get( netuid );
        let pruning_score: Vec<Compact<u16>> = PruningScores::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let last_update: Vec<Compact<u64>> = LastUpdate::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let emission: Vec<Compact<u64>> = Emission::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let dividends: Vec<Compact<u16>> = Dividends::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let incentives: Vec<Compact<u16>> = Incentive::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let consensus: Vec<Compact<u16>> = Consensus::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let trust: Vec<Compact<u16>> = Trust::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let rank: Vec<Compact<u16>> = Rank::<T>::get(netuid).into_iter().map(Compact::from).collect();
        let local_stake: Vec<Compact<u64>> = if LocalStake::<T>::contains_key(netuid) {
            LocalStake::<T>::get(netuid).into_iter().map(Compact::from).collect()
        } else {
            vec![Compact(0); n as usize]
        };
        let global_stake: Vec<Compact<u64>> = if GlobalStake::<T>::contains_key(netuid) {
            GlobalStake::<T>::get(netuid).into_iter().map(Compact::from).collect()
        } else {
            vec![Compact(0); n as usize]
        };
        let stake_weight: Vec<Compact<u16>> = if StakeWeight::<T>::contains_key(netuid) {
            StakeWeight::<T>::get(netuid).into_iter().map(Compact::from).collect()
        } else {
            vec![Compact(0); n as usize]
        };
        let emission_history: Vec<Vec<Compact<u64>>> = Self::get_emissions_history( hotkeys.clone() );
        Some( SubnetState {
            netuid: netuid.into(),
            hotkeys: hotkeys.into(),
            coldkeys: coldkeys.into(),
            active: active.into(),
            validator_permit: validator_permit.into(),
            pruning_score: pruning_score.into(),
            last_update: last_update.into(),
            emission: emission.into(),
            dividends: dividends.into(),
            incentives: incentives.into(),
            consensus: consensus.into(),
            trust: trust.into(),
            rank: rank.into(),
            block_at_registration: block_at_registration.into(),
            local_stake: local_stake.into(),
            global_stake: global_stake.into(),
            stake_weight: stake_weight.into(),
            emission_history: emission_history,
        } )
    }
}